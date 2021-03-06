[![.github/workflows/ci.yml](https://github.com/BenjaminHinchliff/ashpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/BenjaminHinchliff/ashpaper/actions/workflows/ci.yml)
[![Crates.io Version](https://img.shields.io/crates/v/ashpaper-plus)](https://crates.io/crates/ashpaper-plus)
[![Crates.io](https://img.shields.io/crates/l/ashpaper-plus)](https://crates.io/crates/ashpaper-plus)
[![Crates.io](https://img.shields.io/crates/d/ashpaper-plus)](https://crates.io/crates/ashpaper-plus)
[![Crates.io](https://img.shields.io/docsrs/ashpaper-plus)](https://docs.rs/ashpaper-plus)

# AshPaper Plus
A fully spec complaint inpterpreter for the Esopo language AshPaper conceived by [William Hicks](https://github.com/wphicks). You can read about it and the Esopo project in Willian Hick's own words [here](https://wphicks.github.io/esopo/). Daniel Temkin also wrote about it on esoteric.codes, you can read that [here](https://esoteric.codes/blog/esopo-turing-complete-poetry). And of course the spec! Checkout that out [here](https://github.com/wphicks/Esopo/blob/master/AshPaper/informal_specs.txt).

## Installation

### CLI
```bash
cargo install --features="cli" ashpaper-plus
```

#### With JIT Compilation
```bash
cargo install --features="cli jit" ashpaper-plus
```

### Library
add this to your `cargo.toml`
```toml
ashpaper-plus = "0.5"
```

#### With JIT Compilation
```toml
ashpaper-plus = { version = "0.5", features = ["jit"] }
```

## Usage

### From the CLI
```bash
# execute a program
ashpaper-plus poems/lovely-poem.eso # prints 24
# jit execute a program
ashpaper-plus --jit poems/lovely-poem.eso # prints 24
# count syllables
ashpaper-plus -s "hello world, born to think and not to feel" # prints 10
```

### As a Library
```rust
use std::fs;

use ashpaper_plus as ashpaper;

pub fn main() {
    let fname = "lovely-poem.eso";
    let contents = fs::read_to_string(fname).expect("Something went wrong reading input file!");
    print!("{}", ashpaper::program::execute(&contents));
    // or for jit compilation:
    print!("{}", ashpaper::program::jit_execute(&contents).unwrap())
}
```

Will produce the following String:
```txt
24
```

## How it works

Poetry is your program.

You have two registers at your disposal, r0 and r1 which store signed integers (`i64`).
You also have an stack which can store signed integers (bounds are only that of `Vec<i64>` (`isize::MAX = 9_223_372_036_854_775_807`), or 128 for the JIT).

The register is chosen based on if a line is indented - if so, r1 and if not r0

Here are the instructions at your disposal (in order of precedence):
- *End rhyme with previous line*:If register 0 < register 1, push the number of
syllables present in the previous line to the stack. Otherwise, push the number of
syllables in the current line to the stack.
- *Line contains `/`*: If the value in the active register is greater than the number of syllables in the line, go to the line number that corresponds to the value in the **non-active** register. If abs(n) <= lines then n, else n % lines.
- *Capital letter appears inside a word*: Negate the active register.
- *Capital letter appears at the beginning of a word*: Multiply registers and store result in the active register.
- *Line contains the words 'like' or 'as'*: Add registers and store in the active register.
- *Line contains `?`*: Print ASCII character associated with value of the active register. If abs(n) <= u8::MAX n, else n % u8::MAX.
- *Line contains `.`*: Print integer value of the active register.
- *Line contains `,`*: Pop from the stack and store in the active register.
- *Line contains `-`*: Push the value of the active register to the stack.
- *Alleteration of consecutive words*: Goto line indicated by active register
- *Blank line*: no-op.
- *Everything else*: Store number of syllables in the line to the active register.


Let's take this poem in a file called `lovely-poem.eso`. This poem-program (poegram‽) calculates factorials and input in the number of syllables in the title. (I learned a lot from reading the poem "other woodwork" by William Hicks)
```txt
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
poem or a calculator or nothing
how lovely can it be?
```

When `RUST_LOG=info` is set in the envrionment varables for the cli, you can get at program evaluation info. Here's what `lovely-poem.eso` looks like.
```txt
instruction                                         |  r0  |  r1  |  stack
--------------------------------------------------- | ---- | ---- | -------
lovely poem                                         |  4   |  0   | []
                                                    |  4   |  0   | []
  it is a calculator, like a                        |  4   |  4   | []
      poem, is a poem, and finds                    |  4   |  4   | []
        factori-                                    |  4   |  4   | [4]
          als                                       |  4   |  1   | [4]
  The input is the syllAbles                        |  4   |  -1  | [4]
in the title, count them, as one counts             |  3   |  -1  | [4]
  (q) what other poem, programs can be writ         |  3   |  4   | []
  (a) anything a Turing                             |  3   |  12  | []
    machine-machine-machine                         |  3   |  12  | [12]
    would do                                        |  3   |  2   | [12]
  it is a calculator, like a                        |  3   |  5   | [12]
      poem, is a poem, and finds                    |  3   |  12  | []
        factori-                                    |  3   |  12  | [12]
          als                                       |  3   |  1   | [12]
  The input is the syllAbles                        |  3   |  -1  | [12]
in the title, count them, as one counts             |  2   |  -1  | [12]
  (q) what other poem, programs can be writ         |  2   |  12  | []
  (a) anything a Turing                             |  2   |  24  | []
    machine-machine-machine                         |  2   |  24  | [24]
    would do                                        |  2   |  2   | [24]
re/cur                                              |  2   |  2   | [24]
    sion works too, in poems, programs, and this    |  2   |  24  | []
       a lovely.                                    |  2   |  24  | []
poem or calculator or nothing                     |  10  |  24  | []
how lovely can it be?                               |  10  |  24  | []
```

## Caveat about compliance with the informal spec
- It is possible at this point that my implementation deviates from the spec in unintended ways. If you spot anything like that, please raise an issue :heart: :heart:
