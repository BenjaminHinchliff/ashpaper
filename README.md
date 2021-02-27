[![.github/workflows/ci.yml](https://github.com/BenjaminHinchliff/ashpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/BenjaminHinchliff/ashpaper/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/BenjaminHinchliff/ashpaper/branch/main/graph/badge.svg?token=SED7QMHHER)](https://codecov.io/gh/BenjaminHinchliff/ashpaper)
<!--
[![Crates.io Version](https://img.shields.io/crates/v/ashpaper.svg)](https://crates.io/crates/ashpaper)
[![Crates.io](https://img.shields.io/crates/d/ashpaper.svg)](https://crates.io/crates/ashpaper)
-->

# AshPaper
An inpterpreter for the Esopo language AshPaper conceived by [William Hicks](https://github.com/wphicks). You can read about it and the Esopo project in Willian Hick's own words [here](https://wphicks.github.io/esopo/). Daniel Temkin also wrote about it on esoteric.codes, you can read that [here](https://esoteric.codes/blog/esopo-turing-complete-poetry). And of course the spec! Checkout that out [here](https://github.com/wphicks/Esopo/blob/master/AshPaper/informal_specs.txt).

## How it works

Poetry is your program.

You have two registers at your disposal, r0 and r1 which store signed integers (`i64`).
You also have an stack which can store signed integers (bounds are only that of `Vec<i64>`).

Here are the instructions at your disposal (in order that they get precedence):
- _End rhyme with previous line_: Unimplemented.
- Line contains `/`: If the value in the active register is greater than the number of syllables in the line, go to the line number that corresponds to the value in the **non-active** register. If abs(n) <= lines then n, else n % lines.
- _Capital letter appears inside a word_: Negate the active register.
- _Capital letter appears at the beginning of a word_: Multiply registers and store result in the active register.
- _Line contains the words 'like' or 'as'_: Add registers and store in the active register.
- _Line contains `?`_: Print ASCII character associated with value of the active register. If abs(n) <= u8::MAX n, else n % u8::MAX.
- _Line contains `.`_: Print integer value of the active register.
- _Line contains `,`_: Pop from the stack and store in the active register.
- _Line contains `-`_: Push the value of the active register to the stack.
- _Alleteration of consecutive words_: Unimplemented.
- _Blank line_: no-op.
- _Everything else_: Store number of syllables in the line to the active register.


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
Using this library, you can run it with a program that looked like this:
```rust
extern crate ashpaper;

use std::fs;

pub fn main() {
    let fname = "lovely-poem.eso";
    let contents = fs::read_to_string(fname).expect("Something went wrong reading input file!");
    match ashpaper::program::execute(&contents) {
        Ok(res) => print!("{}", res),
        Err(e) => eprintln!("{}", e),
    }
}
```

And it will produce the following String:
```txt
24
```

When `RUST_LOG=info` is set and the caller initializes logging, you can get at program evaluation info. Here's what `lovely-poem.eso` looks like.
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
poem or a calculator or nothing                     |  10  |  24  | []
how lovely can it be?                               |  10  |  24  | []
```

## Some caveats about compliance with the informal spec
- It is possible at this point that my implementation deviates from the spec in unintended ways. If you spot anything like that, please raise an issue :heart: :heart:
- The alliteration and rhyming rules are still unimplemented.
