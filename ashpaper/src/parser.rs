use std::cmp;

use cmudict_fast::{self as cmudict, Cmudict};
use lazy_static::lazy_static;
use regex::Regex;

/// represents a single line and its metadata
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InsType {
    LineConditional {
        prev_syllables: usize,
        cur_syllables: usize,
    },
    Compare,
    Negate,
    Multiply,
    Add,
    PrintChar,
    Print,
    Pop,
    Push,
    Goto,
    Store(usize),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Register {
    Register0,
    Register1,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Instruction {
    pub instruction: InsType,
    pub register: Register,
    pub line: String,
}

lazy_static! {
    // * it is assumed that these Regexes are valid
    static ref NUM_RE: Regex = Regex::new(r"[0-9]").unwrap();
    static ref INT_CAP_RE: Regex = Regex::new(r"\b\S+[A-Z]\S+\b").unwrap();
    static ref CAP_RE: Regex = Regex::new(r"\b[A-Z][^A-Z]+\b").unwrap();
    static ref SIMILIE_RE: Regex = Regex::new(r"\b(like|as)\b").unwrap();
    static ref WS_START_RE: Regex = Regex::new(r"^\s").unwrap();
    static ref VOWEL_CLUSTER_RE: Regex = Regex::new(r"[^aeiouy]+").unwrap();
    // * no error handling
    static ref CMUDICT: Cmudict = Cmudict::new("cmudict.dict").unwrap();
}

/// test for alliteration by checking if multiple words in the input
/// start with the same letter
fn has_alliteration(input: &str) -> bool {
    let lower_input = input.to_lowercase();
    let mut input_iter = lower_input.split(' ').filter(|w| !w.is_empty());

    if let Some(start_word) = input_iter.next() {
        let mut cur_start_letter = start_word.chars().next().unwrap();
        for word in input_iter {
            if word.starts_with(cur_start_letter) {
                return true;
            }
            cur_start_letter = word.chars().next().unwrap();
        }
    }
    false
}

fn approximate_syllables(word: &str) -> usize {
    let clusters: Vec<_> = VOWEL_CLUSTER_RE.split(word).collect();
    const DIPHTHONGS: &[&'static str] = &[
        "ai", "au", "ay", "ea", "ee", "ei", "ey", "oa", "oe", "oi", "oo", "ou", "oy", "ua", "ue",
        "ui",
    ];
    println!("{:?}", clusters);
    let mut count: usize = 0;
    for cluster in clusters {
        count += if DIPHTHONGS.contains(&cluster) {
            1
        } else {
            cmp::min(2, cluster.len())
        }
    }
    count
}

fn count_word_syllables(word: &str) -> usize {
    if let Some(rules) = CMUDICT.get(word) {
        rules
            .iter()
            .map(|r| {
                r.pronunciation()
                    .iter()
                    .filter(|po| po.is_syllable())
                    .count()
            })
            .max()
            .unwrap()
    } else {
        approximate_syllables(word)
    }
}

fn count_syllables(input: &str) -> usize {
    input
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(|w| count_word_syllables(w))
        .sum()
}

fn check_end_rhyme(last_line_option: Option<&str>, cur_line: &str) -> bool {
    if let Some(last_line) = last_line_option {
        // end-rhyme handling
        if let (Some(last_line_word), Some(last_word)) = (
            last_line.split(' ').rev().next(),
            cur_line.split(' ').rev().next(),
        ) {
            if let (Some(last_line_rule), Some(last_rule)) =
                (CMUDICT.get(last_line_word), CMUDICT.get(last_word))
            {
                return cmudict::rhymes(&last_line_rule, &last_rule);
            }
        }
    }
    false
}

pub fn parse(input: &str) -> Vec<Instruction> {
    let mut last_line_option: Option<&str> = None;
    let mut lines = Vec::new();
    for line in input.split('\n') {
        // short-circuit on noop
        if line != "" {
            // everything else
            let insType = if line.contains('/') {
                InsType::Compare
            } else if INT_CAP_RE.is_match(line) {
                InsType::Negate
            } else if CAP_RE.is_match(line) {
                InsType::Multiply
            } else if SIMILIE_RE.is_match(line) {
                InsType::Add
            } else if line.contains('?') {
                InsType::PrintChar
            } else if line.contains('.') {
                InsType::Print
            } else if line.contains(',') {
                InsType::Pop
            } else if line.contains('-') {
                InsType::Push
            } else if has_alliteration(line) {
                InsType::Goto
            } else if check_end_rhyme(last_line_option, line) {
                println!("{:?}", last_line_option);
                InsType::LineConditional {
                    prev_syllables: count_syllables(last_line_option.unwrap()),
                    cur_syllables: count_syllables(line),
                }
            } else {
                InsType::Store(count_syllables(line))
            };
            let register = if WS_START_RE.is_match(line) {
                Register::Register1
            } else {
                Register::Register0
            };
            let ins = Instruction {
                instruction: insType,
                register,
                line: line.to_string(),
            };
            lines.push(ins);
        }
        last_line_option = Some(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{InsType, Instruction, Register};
    use pretty_assertions::assert_eq;

    #[test]
    fn has_alliteration() {
        assert!(super::has_alliteration("she sells sea shells"));
        assert!(!super::has_alliteration("no alliteration here"));
        assert!(!super::has_alliteration("one"));
        assert!(!super::has_alliteration(""));
    }

    #[test]
    fn syllable_counting() {
        let exact = super::count_syllables("antidisestablishmentarianism");
        assert_eq!(exact, 12);
        let approx = super::count_syllables("supercalifragilisticexpialidocious");
        assert_eq!(approx, 15);
    }

    #[test]
    fn rhyming() {
        let source = r#"
he thrust every elf
    far back on the shelf
"#;
        let tokens = super::parse(source);
        let mut split = source.trim().split('\n');
        let parsed = vec![
            Instruction {
                instruction: InsType::Goto,
                register: Register::Register0,
                line: split.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::LineConditional {
                    prev_syllables: 6,
                    cur_syllables: 5,
                },
                register: Register::Register1,
                line: split.next().unwrap().to_string(),
            },
        ];
        assert_eq!(tokens, parsed);
    }
}
