use cranelift::{
    codegen::{
        entity,
        ir::{JumpTable, StackSlot},
    },
    frontend::Switch,
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Linkage, Module};
use itertools::{EitherOrBoth, Itertools};

use crate::parser::{InsType, Instruction, Register};

const STACK_SIZE: u32 = 128;

pub struct JIT {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    data_ctx: DataContext,
    module: JITModule,
}

impl Default for JIT {
    fn default() -> Self {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            data_ctx: DataContext::new(),
            module,
        }
    }
}

impl JIT {
    pub fn compile(&mut self, ast: &[Instruction]) -> Result<(), String> {
        // needed due to unused block jump table problems
        // (https://github.com/bytecodealliance/wasmtime/issues/2670)
        // not yet patched in cargo release
        let ast: Vec<_> = ast
            .iter()
            .filter(|n| match n.instruction {
                InsType::Noop | InsType::PrintChar | InsType::PrintValue => false,
                _ => true,
            })
            .collect();
        let int = self.module.target_config().pointer_type();

        println!("{:?}", int);

        self.ctx.func.signature.returns.push(AbiParam::new(int));

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        // program stack
        let stack = builder.create_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            STACK_SIZE * int.bytes(),
        ));

        let r0 = Variable::new(0);
        let r1 = Variable::new(1);
        let top = Variable::new(2);
        builder.declare_var(r0, int);
        builder.declare_var(r1, int);
        // top of stack
        builder.declare_var(top, int);

        let unreach_trap_block = builder.create_block();

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let zero1 = builder.ins().iconst(int, 0);
        builder.def_var(r0, zero1);
        let zero2 = builder.ins().iconst(int, 0);
        builder.def_var(r1, zero2);
        let top_ptr = builder.ins().stack_addr(int, stack, 0);
        builder.def_var(top, top_ptr);

        let switch_fallback = builder.create_block();

        let mut blocks = Vec::new();
        let mut lines = Vec::new();
        // create blocks and add to jump table
        for (line, _) in ast
            .iter()
            .filter(|n| n.instruction != InsType::Noop)
            .enumerate()
        {
            let block = builder.create_block();
            blocks.push(block);
            lines.push(line);
        }

        // connect entry block to first block
        // TODO: prevent crash on empty program
        builder.ins().jump(*blocks.first().unwrap(), &[]);

        for (node, block_and_next) in ast
            .iter()
            .zip(blocks.iter().zip_longest(blocks[1..].iter()))
        {
            let (block, next) = match block_and_next {
                EitherOrBoth::Left(l) => (*l, None),
                EitherOrBoth::Both(l, r) => (*l, Some(*r)),
                EitherOrBoth::Right(_) => unreachable!(),
            };
            // get block ready for instructions
            builder.switch_to_block(block);

            // actually translate an instructon to CLIR
            Self::translate_instruction(
                node,
                int,
                switch_fallback,
                unreach_trap_block,
                next,
                &blocks,
                &lines,
                &mut builder,
                r0,
                r1,
                top,
            );
        }

        builder.switch_to_block(unreach_trap_block);
        builder.seal_block(unreach_trap_block);
        builder.ins().trap(TrapCode::UnreachableCodeReached);

        builder.seal_all_blocks();

        println!("{:?}", self.ctx.func);

        let id = self
            .module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)
            .unwrap();

        self.module
            .define_function(id, &mut self.ctx, &mut codegen::binemit::NullTrapSink {})
            .unwrap();

        self.module.clear_context(&mut self.ctx);

        self.module.finalize_definitions();

        let ptr = self.module.get_finalized_function(id);

        let out_fn = unsafe { std::mem::transmute::<_, fn() -> i64>(ptr) };

        println!("{:?}", out_fn());

        Ok(())
    }

    // I'm so mad that switches can't be created multiple times and
    // I have to do this crap
    fn make_switch(blocks: &[Block], lines: &[usize]) -> Switch {
        let mut switch = Switch::new();
        for (line, block) in lines.iter().zip(blocks) {
            switch.set_entry(*line as u128, *block);
        }
        switch
    }

    fn translate_instruction(
        ins: &Instruction,
        int: Type,
        fallback: Block,
        trap_block: Block,
        next_block: Option<Block>,
        all_blocks: &[Block],
        line_numbers: &[usize],
        builder: &mut FunctionBuilder,
        r0: Variable,
        r1: Variable,
        top: Variable,
    ) {
        let Instruction {
            instruction: kind,
            register: reg,
            line: _line,
        } = ins;
        let active_reg = match reg {
            Register::Register0 => r0,
            Register::Register1 => r1,
        };
        let inactive_reg = match reg {
            Register::Register0 => r1,
            Register::Register1 => r0,
        };
        match kind {
            InsType::Store(syl) => {
                let store_val = builder.ins().iconst(int, *syl as i64);
                let reg_val = builder.use_var(active_reg);
                let add = builder.ins().iadd(reg_val, store_val);
                builder.def_var(active_reg, add);
            }
            InsType::Negate => {
                let reg_val = builder.use_var(active_reg);
                let neg = builder.ins().ineg(reg_val);
                builder.def_var(active_reg, neg);
            }
            InsType::Multiply => {
                let active_val = builder.use_var(active_reg);
                let inactive_val = builder.use_var(inactive_reg);
                let mult = builder.ins().imul(active_val, inactive_val);
                builder.def_var(active_reg, mult);
            }
            InsType::Add => {
                let active_val = builder.use_var(active_reg);
                let inactive_val = builder.use_var(inactive_reg);
                let add = builder.ins().iadd(active_val, inactive_val);
                builder.def_var(active_reg, add);
            }
            InsType::Goto => {
                let index_val = builder.use_var(active_reg);
                let switch = Self::make_switch(all_blocks, line_numbers);
                switch.emit(builder, index_val, fallback);
            }
            InsType::Push => Self::translate_push(int, active_reg, builder, top),
            InsType::Pop => {
                let top_val = builder.use_var(top);
                let size = builder.ins().iconst(int, int.bytes() as i64);
                let dec = builder.ins().isub(top_val, size);
                builder.def_var(top, dec);
                let top_val = builder.use_var(top);
                let loaded_val = builder.ins().load(int, MemFlags::new(), top_val, 0);
                builder.def_var(active_reg, loaded_val);
            }
            InsType::ConditionalPush {
                prev_syllables,
                cur_syllables,
            } => {
                let active_val = builder.use_var(active_reg);
                let inactive_val = builder.use_var(inactive_reg);
                let cond_val = builder
                    .ins()
                    .icmp(IntCC::SignedLessThan, active_val, inactive_val);
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                builder.ins().brz(cond_val, else_block, &[]);
                builder.ins().jump(then_block, &[]);

                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                let cur_val = builder.ins().iconst(int, *cur_syllables as i64);
                Self::translate_push_val(int, cur_val, builder, top);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                let prev_val = builder.ins().iconst(int, *prev_syllables as i64);
                Self::translate_push_val(int, prev_val, builder, top);
                builder.ins().jump(merge_block, &[]);
            }
            _ => (),
        }
        Self::connect_end(builder, next_block, r0);
    }

    fn translate_push_val(int: Type, value: Value, builder: &mut FunctionBuilder, top: Variable) {
        let top_val = builder.use_var(top);
        builder.ins().store(MemFlags::new(), value, top_val, 0);
        let size = builder.ins().iconst(int, int.bytes() as i64);
        let inc = builder.ins().iadd(top_val, size);
        builder.def_var(top, inc);
    }

    fn translate_push(int: Type, reg: Variable, builder: &mut FunctionBuilder, top: Variable) {
        let store_val = builder.use_var(reg);
        let top_val = builder.use_var(top);
        builder.ins().store(MemFlags::new(), store_val, top_val, 0);
        let size = builder.ins().iconst(int, int.bytes() as i64);
        let inc = builder.ins().iadd(top_val, size);
        builder.def_var(top, inc);
    }

    fn connect_end(builder: &mut FunctionBuilder, next_block: Option<Block>, r0: Variable) {
        if let Some(next) = next_block {
            builder.ins().jump(next, &[]);
        } else {
            let ret_val = builder.use_var(r0);
            builder.ins().return_(&[ret_val]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn jit() {
        let source = include_str!("../poems/goto-test.eso"); 
        let tokens = parser::parse(source);
        println!("{:#?}", tokens);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
        panic!()
    }
}
