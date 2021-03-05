use std::mem;

use cranelift::{
    codegen::ir::{FuncRef, JumpTable, StackSlot},
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use itertools::{EitherOrBoth, Itertools};

use super::{
    errors::jit::JitResult,
    parser::{InsType, Instruction, Register},
    rt::{put_char, put_value},
};

#[derive(Debug)]
struct Stack {
    stack: StackSlot,
    ptr: Variable,
    start: Variable,
    end: Variable,
    overflow_trap: Block,
}

const STACK_SIZE: u32 = 128;

pub struct JIT {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: JITModule,
}

impl Default for JIT {
    fn default() -> Self {
        let mut builder = JITBuilder::new(cranelift_module::default_libcall_names());
        // import runtime functions into jit
        let put_val_addr: *const u8 = unsafe { mem::transmute(put_value as fn(_)) };
        builder.symbol("put_value", put_val_addr);
        let put_char_addr: *const u8 = unsafe { mem::transmute(put_char as fn(_)) };
        builder.symbol("put_char", put_char_addr);
        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        }
    }
}

impl JIT {
    pub fn compile(&mut self, ast: &[Instruction]) -> JitResult<fn()> {
        let int = self.module.target_config().pointer_type();

        // create imported funcs before builder
        let put_val_id = self.make_put_value()?;
        let put_char_id = self.make_put_char()?;

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        // declare runtime functions
        let put_val_func = self
            .module
            .declare_func_in_func(put_val_id, &mut builder.func);
        let put_char_func = self
            .module
            .declare_func_in_func(put_char_id, &mut builder.func);

        // build stack
        let stack_byte_size = STACK_SIZE * int.bytes();
        // create stack parts
        let stack_slot = builder.create_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            stack_byte_size,
        ));
        let stack_ptr = Variable::new(0);
        let stack_start = Variable::new(1);
        let stack_end = Variable::new(2);
        // declare stack parts
        builder.declare_var(stack_ptr, int);
        builder.declare_var(stack_start, int);
        builder.declare_var(stack_end, int);

        // create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // define stack parts
        let stack_ptr_val = builder.ins().stack_addr(int, stack_slot, 0);
        builder.def_var(stack_ptr, stack_ptr_val);
        let stack_start_val = builder.use_var(stack_ptr);
        builder.def_var(stack_start, stack_start_val);
        let stack_start_val = builder.use_var(stack_ptr);
        let stack_size_val = builder.ins().iconst(int, stack_byte_size as i64);
        let stack_end_val = builder.ins().iadd(stack_start_val, stack_size_val);
        builder.def_var(stack_end, stack_end_val);

        let stack_overflow_trap = builder.create_block();

        let stack = Stack {
            stack: stack_slot,
            ptr: stack_ptr,
            start: stack_start,
            end: stack_end,
            overflow_trap: stack_overflow_trap,
        };

        let r0 = Variable::new(3);
        let r1 = Variable::new(4);

        builder.declare_var(r0, int);
        builder.declare_var(r1, int);

        let zero1 = builder.ins().iconst(int, 0);
        builder.def_var(r0, zero1);
        let zero2 = builder.ins().iconst(int, 0);
        builder.def_var(r1, zero2);

        let mut jump_table_data = JumpTableData::new();

        let mut blocks = Vec::new();
        // create blocks and add to jump table
        for _ in ast {
            let block = builder.create_block();
            jump_table_data.push_entry(block);
            blocks.push(block);
        }

        let jump_table = builder.create_jump_table(jump_table_data);

        // connect entry block to first block
        Self::connect_end(&mut builder, blocks.first().copied());

        // build stack overflow trap block
        builder.switch_to_block(stack_overflow_trap);
        builder.seal_block(stack_overflow_trap);
        builder.ins().trap(TrapCode::StackOverflow);

        // build unreachable trap block
        let unreach_trap_block = builder.create_block();
        builder.switch_to_block(unreach_trap_block);
        builder.ins().trap(TrapCode::UnreachableCodeReached);

        if !blocks.is_empty() {
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
                    &stack,
                    jump_table,
                    unreach_trap_block,
                    next,
                    &mut builder,
                    put_val_func,
                    put_char_func,
                    r0,
                    r1,
                );
            }
        }

        builder.seal_all_blocks();

        let id = self
            .module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)?;

        self.module
            .define_function(id, &mut self.ctx, &mut codegen::binemit::NullTrapSink {})?;

        self.module.clear_context(&mut self.ctx);

        self.module.finalize_definitions();

        let ptr = self.module.get_finalized_function(id);

        Ok(unsafe { std::mem::transmute::<_, fn()>(ptr) })
    }

    pub fn make_put_value(&mut self) -> JitResult<FuncId> {
        let int = self.module.target_config().pointer_type();

        self.ctx.func.signature.params.push(AbiParam::new(int));

        let put_value =
            self.module
                .declare_function("put_value", Linkage::Import, &self.ctx.func.signature)?;
        self.module.clear_context(&mut self.ctx);
        Ok(put_value)
    }

    pub fn make_put_char(&mut self) -> JitResult<FuncId> {
        let int = self.module.target_config().pointer_type();
        self.ctx.func.signature.params.push(AbiParam::new(int));

        let put_char =
            self.module
                .declare_function("put_char", Linkage::Import, &self.ctx.func.signature)?;
        self.module.clear_context(&mut self.ctx);
        Ok(put_char)
    }

    fn translate_instruction(
        ins: &Instruction,
        int: Type,
        stack: &Stack,
        jump_table: JumpTable,
        unreach_trap: Block,
        next_block: Option<Block>,
        builder: &mut FunctionBuilder,
        put_val_func: FuncRef,
        put_char_func: FuncRef,
        r0: Variable,
        r1: Variable,
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
                builder.def_var(active_reg, store_val);
                Self::connect_end(builder, next_block);
            }
            InsType::Negate => {
                let reg_val = builder.use_var(active_reg);
                let neg = builder.ins().ineg(reg_val);
                builder.def_var(active_reg, neg);
                Self::connect_end(builder, next_block);
            }
            InsType::Multiply => {
                let active_val = builder.use_var(active_reg);
                let inactive_val = builder.use_var(inactive_reg);
                let mult = builder.ins().imul(active_val, inactive_val);
                builder.def_var(active_reg, mult);
                Self::connect_end(builder, next_block);
            }
            InsType::Add => {
                let active_val = builder.use_var(active_reg);
                let inactive_val = builder.use_var(inactive_reg);
                let add = builder.ins().iadd(active_val, inactive_val);
                builder.def_var(active_reg, add);
                Self::connect_end(builder, next_block);
            }
            InsType::Goto => {
                let index_val = builder.use_var(active_reg);
                builder.ins().br_table(index_val, unreach_trap, jump_table);
            }
            InsType::ConditionalGoto(syl) => {
                let syl_val = builder.ins().iconst(int, *syl as i64);
                let reg_val = builder.use_var(active_reg);
                let cond_val = builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThan, reg_val, syl_val);
                let then_block = builder.create_block();
                let merge_block = builder.create_block();
                builder.ins().brnz(cond_val, then_block, &[]);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(then_block);
                let index_val = builder.use_var(inactive_reg);
                builder.ins().br_table(index_val, unreach_trap, jump_table);

                builder.switch_to_block(merge_block);
                Self::connect_end(builder, next_block);
            }
            InsType::Push => {
                Self::translate_push(int, active_reg, builder, stack);
                Self::connect_end(builder, next_block);
            }
            InsType::Pop => {
                Self::translate_pop(int, active_reg, builder, stack);
                Self::connect_end(builder, next_block);
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
                let cur_val = builder.ins().iconst(int, *cur_syllables as i64);
                Self::translate_push_val(int, cur_val, builder, stack);
                builder.ins().jump(merge_block, &[]);

                builder.switch_to_block(then_block);
                let prev_val = builder.ins().iconst(int, *prev_syllables as i64);
                Self::translate_push_val(int, prev_val, builder, stack);
                builder.ins().jump(merge_block, &[]);
                Self::connect_end(builder, next_block);
            }
            InsType::PrintValue => {
                let reg_val = builder.use_var(active_reg);
                builder.ins().call(put_val_func, &[reg_val]);
                Self::connect_end(builder, next_block);
            }
            InsType::PrintChar => {
                let reg_val = builder.use_var(active_reg);
                builder.ins().call(put_char_func, &[reg_val]);
                Self::connect_end(builder, next_block);
            }
            InsType::Noop => Self::connect_end(builder, next_block),
        }
    }

    fn translate_pop(int: Type, reg: Variable, builder: &mut FunctionBuilder, stack: &Stack) {
        let top_val = builder.use_var(stack.ptr);
        let stack_start_val = builder.use_var(stack.start);
        let comp = builder
            .ins()
            .icmp(IntCC::SignedLessThanOrEqual, top_val, stack_start_val);
        let then_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.ins().brnz(comp, merge_block, &[]);
        builder.ins().jump(then_block, &[]);

        builder.switch_to_block(then_block);
        let ptr_size = builder.ins().iconst(int, int.bytes() as i64);
        let dec = builder.ins().isub(top_val, ptr_size);
        builder.def_var(stack.ptr, dec);
        let top_val = builder.use_var(stack.ptr);
        let loaded_val = builder.ins().load(int, MemFlags::new(), top_val, 0);
        builder.def_var(reg, loaded_val);
        builder.ins().jump(merge_block, &[]);

        builder.switch_to_block(merge_block);
    }

    fn translate_push_val(int: Type, value: Value, builder: &mut FunctionBuilder, stack: &Stack) {
        let ptr_val = builder.use_var(stack.ptr);
        builder.ins().store(MemFlags::new(), value, ptr_val, 0);
        let size = builder.ins().iconst(int, int.bytes() as i64);
        let inc = builder.ins().iadd(ptr_val, size);
        builder.def_var(stack.ptr, inc);
    }

    fn translate_push(int: Type, reg: Variable, builder: &mut FunctionBuilder, stack: &Stack) {
        let store_val = builder.use_var(reg);
        let ptr_val = builder.use_var(stack.ptr);
        builder.ins().store(MemFlags::new(), store_val, ptr_val, 0);
        let size = builder.ins().iconst(int, int.bytes() as i64);
        let inc = builder.ins().iadd(ptr_val, size);
        builder.def_var(stack.ptr, inc);
    }

    fn connect_end(builder: &mut FunctionBuilder, next_block: Option<Block>) {
        if let Some(next) = next_block {
            builder.ins().jump(next, &[]);
        } else {
            builder.ins().return_(&[]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn basic_goto() {
        let source = include_str!("../poems/goto-test.eso");
        let tokens = parser::parse(source);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }

    #[test]
    fn factorial() {
        let source = include_str!("../poems/original-factorial.eso");
        let tokens = parser::parse(source);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }

    #[test]
    fn stack() {
        let source = include_str!("../poems/stack-test.eso");
        let tokens = parser::parse(source);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }

    #[test]
    fn cond_goto() {
        let source = include_str!("../poems/cond-goto-test.eso");
        let tokens = parser::parse(source);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }

    #[test]
    fn math() {
        let source = include_str!("../poems/math-test.eso");
        let tokens = parser::parse(source);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }

    #[test]
    fn empty() {
        let tokens = parser::parse("");
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
    }
}
