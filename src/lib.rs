//! # AshPaper
//! An inpterpreter for the Esopo language AshPaper conceived by [William Hicks](https://github.com/wphicks).
//! You can read about it and the Esopo project in Willian Hick's own words [here](https://wphicks.github.io/esopo/).
//! Daniel Temkin also wrote about it on esoteric.codes, you can read that [here](https://esoteric.codes/blog/esopo-turing-complete-poetry).
//! And of course the spec! Checkout that out [here](https://github.com/wphicks/Esopo/blob/master/AshPaper/informal_specs.txt).
//!
//! ## How it works
//!
//! You can execute poetry!
//!
//! Let's take this poem that calculates factorials and input in the number of syllables in the title.
//! (I (Shea Newton) learned a lot from reading the poem "other woodwork" by William Hicks)
//! ```txt
//! lovely poem
//!
//!   it is a calculator, like a
//!       poem, is a poem, and finds
//!         factori-
//!           als
//!   The input is the syllAbles
//! in the title, count them, as one counts
//!   (q) what other poem, programs can be writ
//!   (a) anything a Turing
//!     machine-machine-machine
//!     would do
//! re/cur
//!     sion works too, in poems, programs, and this
//!        a lovely.
//! poem or a calculator or nothing
//! how lovely can it be?
//! ```
//! Using this library, you can run it with a program that looked like this:
//! ```rust
//! use std::fs;
//!
//! use ashpaper_plus::program::Program;
//!
//! pub fn main() {
//!    let contents = "
//!lovely poem
//!
//!  it is a calculator, like a
//!      poem, is a poem, and finds
//!        factori-
//!          als
//!  The input is the syllAbles
//!in the title, count them, as one counts
//!  (q) what other poem, programs can be writ
//!  (a) anything a Turing
//!    machine-machine-machine
//!    would do
//!re/cur
//!    sion works too, in poems, programs, and this
//!       a lovely.
//!poem or a calculator or nothing
//!how lovely can it be?
//!                    ";
//!
//!     let program = Program::create(&contents);
//!     print!("{}", program.execute())
//! }
//! ```
//!
//! And it will produce the output:
//! ```txt
//! 24
//!
//! ```
//!
//! unlike the original, you can also install the binary from this crate
//! (you need to enable the cli feature so the executable is built)
//! ```bash
//! cargo install --features cli ashpaper-plus
//! ```
//!
//! ## Some caveats about compliance with the informal spec
//! - It's entirely possible at this point that some of the implementation deviates from the spec in unintended ways. If you spot anything like that, please raise an issue
mod parser;
mod program;
pub use program::Program;
pub use parser::count_syllables;
