use cranelift::{
    codegen::{
        entity,
        ir::{JumpTable, StackSlot},
    },
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Linkage, Module};

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
            .filter(|n| n.instruction != InsType::Noop)
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
        builder.switch_to_block(unreach_trap_block);
        builder.seal_block(unreach_trap_block);
        builder.ins().trap(TrapCode::UnreachableCodeReached);

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

        let mut blocks = Vec::new();
        let mut table_dat = JumpTableData::new();
        // create blocks and add to jump table
        for _ in &ast {
            let block = builder.create_block();
            table_dat.push_entry(block);
            blocks.push(block);
        }

        let jump_table = builder.create_jump_table(table_dat);

        for (node, block) in ast.iter().zip(blocks) {
            builder.ins().jump(block, &[]);
            builder.switch_to_block(block);
            builder.seal_block(block);

            Self::translate_instruction(
                node,
                int,
                jump_table,
                unreach_trap_block,
                &mut builder,
                r0,
                r1,
                stack,
                top,
            );
        }

        let ret_val = builder.use_var(r0);
        builder.ins().return_(&[ret_val]);

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

    fn translate_instruction(
        ins: &Instruction,
        int: Type,
        jt: JumpTable,
        trap_block: Block,
        builder: &mut FunctionBuilder,
        r0: Variable,
        r1: Variable,
        stack: StackSlot,
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
                builder.ins().br_table(index_val, trap_block, jt);
            }
            InsType::Push => {}
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn jit() {
        let source = r#"
lovely poem

  it is a calculator, like a
      poem, is a poem, and finds
        factori-
          als
  The input is the syllAbles
in the title, count them, as one counts
  (q) what other poem, programs can be writ
  (a) anything a Turing
    machine-machine-machine
    would do
re/cur
    sion works too, in poems, programs, and this
       a lovely.
poem or calculator or nothing
how lovely can it be?
"#;
        let tokens = parser::parse(source);
        println!("{:#?}", tokens);
        let mut jit = JIT::default();
        jit.compile(&tokens).unwrap();
        panic!()
    }
}
