use crate::jit::JIT;

use super::parser::{
    self, Register, InsType, Instruction,
};
use super::errors::Result;

#[derive(Debug, Clone)]
struct Memory {
    register0: i64,
    register1: i64,
    stack: Vec<i64>,
}

impl Memory {
    fn new() -> Memory {
        Memory {
            register0: 0,
            register1: 0,
            stack: vec![],
        }
    }

    fn store_syllables(&mut self, register: Register, syllables: i64) {
        match register {
            Register::Register0 => self.register0 = syllables,
            Register::Register1 => self.register1 = syllables,
        }
    }

    fn push_to_stack(&mut self, val: i64) {
        self.stack.push(val);
    }

    fn push(&mut self, register: Register) {
        match register {
            Register::Register0 => self.stack.push(self.register0),
            Register::Register1 => self.stack.push(self.register1),
        }
    }

    fn pop(&mut self, register: Register) {
        if let Some(val) = self.stack.pop() {
            match register {
                Register::Register0 => self.register0 = val,
                Register::Register1 => self.register1 = val,
            }
        }
    }

    fn multiply(&mut self, register: Register) {
        match register {
            Register::Register0 => self.register0 *= self.register1,
            Register::Register1 => self.register1 *= self.register0,
        }
    }

    fn add(&mut self, register: Register) {
        match register {
            Register::Register0 => self.register0 += self.register1,
            Register::Register1 => self.register1 += self.register0,
        }
    }

    fn get_active(&self, register: Register) -> i64 {
        match register {
            Register::Register0 => self.register0,
            Register::Register1 => self.register1,
        }
    }

    fn get_inactive(&self, register: Register) -> i64 {
        match register {
            Register::Register0 => self.register1,
            Register::Register1 => self.register0,
        }
    }

    fn negate(&mut self, register: Register) {
        match register {
            Register::Register0 => self.register0 = -self.register0,
            Register::Register1 => self.register1 = -self.register1,
        }
    }
}

pub struct Program {
    pub ast: Vec<Instruction>,
}

impl Program {
    pub fn create(source: &str) -> Program {
        Program {
            ast: parser::parse(source),
        }
    } 

    pub fn execute(&self) -> String {
        let mut mem = Memory::new();
        let mut output: String = String::new();

        let mut instruction_pointer: usize = 0;

        log::info!(
            "{: <51} | {: ^4} | {: ^4} | {: ^7}",
            "instruction",
            "r0",
            "r1",
            "stack"
        );
        log::info!("{:-<51} | {:-^4} | {:-^4} | {:-^7}", "", "", "", "");

        'outer: while let Some(ins) = self.ast.get(instruction_pointer) {
            let Instruction {
                instruction,
                register: reg,
                ref line,
            } = *ins;

            match instruction {
                InsType::ConditionalGoto(syllables) => {
                    if mem.get_active(reg) > syllables as i64 {
                        instruction_pointer =
                            (mem.get_inactive(reg).abs() as usize) % (self.ast.len() as usize);
                        continue 'outer;
                    }
                }
                InsType::Negate => mem.negate(reg),
                InsType::Multiply => mem.multiply(reg),
                InsType::Add => mem.add(reg),
                InsType::PrintChar => {
                    let printable = (mem.get_active(reg).abs() % std::u8::MAX as i64) as u8;
                    output = format!("{}{}", output, printable as char);
                }
                InsType::PrintValue => output = format!("{}{}", output, mem.get_active(reg)),
                InsType::Pop => mem.pop(reg),
                InsType::Push => mem.push(reg),
                InsType::Store(syllables) => mem.store_syllables(reg, syllables as i64),
                InsType::ConditionalPush {
                    prev_syllables,
                    cur_syllables,
                } => {
                    if mem.get_active(reg) < mem.get_inactive(reg) {
                        mem.push_to_stack(prev_syllables as i64);
                    } else {
                        mem.push_to_stack(cur_syllables as i64);
                    }
                }
                InsType::Goto => {
                    instruction_pointer =
                        (mem.get_active(reg).abs() as usize) % (self.ast.len() as usize);
                    continue 'outer;
                }
                InsType::Noop => (),
            }

            log::info!(
                "{: <51} | {: ^4} | {: ^4} | {:^?}",
                line,
                mem.register0,
                mem.register1,
                mem.stack
            );

            instruction_pointer += 1;
        }

        output
    }
    
    #[cfg(feature = "jit")]
    pub fn jit_execute(&self) -> Result<()> {
        let mut jit = JIT::default();
        let func = jit.compile(&self.ast)?;
        func();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn mem_get_inactive() {
        let mut mem = Memory::new();
        let r0 = 10;
        let r1 = 11;
        mem.store_syllables(Register::Register0, r0);
        mem.store_syllables(Register::Register1, r1);

        assert_eq!(mem.get_inactive(Register::Register0), r1);
        assert_eq!(mem.get_inactive(Register::Register1), r0);
    }

    #[test]
    fn mem_push() {
        let mut mem = Memory::new();
        let reg = Register::Register0;
        mem.store_syllables(reg, 1);
        mem.push(reg);
        assert_eq!(mem.stack, vec![1]);
        let reg = Register::Register1;
        mem.store_syllables(reg, 2);
        mem.push(reg);
        assert_eq!(mem.stack, vec![1, 2]);
    }

    #[test]
    fn alliteration() {
        let alliteration_program = r#"
poem or calculator or nothing
    somebody once
    fish fosh
word.

"#
        .trim_start();

        let program = Program::create(alliteration_program);
        let result = program.execute();
        assert_eq!(result, "");
    }

    #[test]
    fn rhyming() {
        let rhyming_program = r#"
somebody once told me 
    he took a new elf 
and stabbed it with a shelf
pop,
print.
then he took blue
and stabbed it with some you 
pop,
print.
"#;

        let program = Program::create(rhyming_program);
        let result = program.execute();
        assert_eq!(result, "64");
    }

    #[test]
    fn factorial() {
        let factorial_program = r#"

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
        let four_factorial = format!("lovely poem\n{}", factorial_program);
        println!("{}", four_factorial);
        let four_factorial_res = "24\n".to_string();
        let program = Program::create(&four_factorial);
        assert_eq!(program.execute(), four_factorial_res);

        let five_factorial = format!("lovely poem and\n{}", factorial_program);
        let program = Program::create(&five_factorial);
        let five_factorial_res = "120\n".to_string();
        assert_eq!(program.execute(), five_factorial_res);
    }

    #[test]
    fn logging() {
        // everything should work as expected if logging is enabled.
        std::env::set_var("RUST_LOG", "info");
        factorial();
    }
}
