use std::{cmp, str::FromStr};

use cmudict_fast::Cmudict;
use cmudict_fast::{self as cmudict};
use lazy_static::lazy_static;
use regex::Regex;

/// represents a single line and its metadata
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InsType {
    ConditionalPush {
        prev_syllables: usize,
        cur_syllables: usize,
    },
    ConditionalGoto(usize),
    Negate,
    Multiply,
    Add,
    PrintChar,
    PrintValue,
    Pop,
    Push,
    Goto,
    Store(usize),
    Noop,
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
    static ref CMUDICT: Cmudict = Cmudict::from_str(include_str!("../res/cmudict.dict")).unwrap();
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

fn check_end_rhyme(last_line_option: Option<&str>, cur_line: &str) -> bool {
    if let Some(last_line) = last_line_option {
        // end-rhyme handling
        if let (Some(last_line_word), Some(last_word)) = (
            last_line.split(' ').rev().filter(|s| !s.is_empty()).next(),
            cur_line.split(' ').rev().filter(|s| !s.is_empty()).next(),
        ) {
            if let (Some(last_line_rule), Some(last_rule)) = (
                CMUDICT.get(&last_line_word.to_lowercase()),
                CMUDICT.get(&last_word.to_lowercase()),
            ) {
                return cmudict::rhymes(last_line_rule, last_rule);
            }
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

pub fn count_syllables(input: &str) -> usize {
    input
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(|w| count_word_syllables(&w.to_lowercase()))
        .sum()
}

pub fn parse(input: &str) -> Vec<Instruction> {
    let mut last_line_option: Option<&str> = None;
    let mut lines = Vec::new();
    for line in input.lines() {
        // short-circuit on noop

        // everything else
        let ins_type = if line.trim().is_empty() {
            InsType::Noop
        } else if check_end_rhyme(last_line_option, line) {
            InsType::ConditionalPush {
                prev_syllables: count_syllables(last_line_option.unwrap()),
                cur_syllables: count_syllables(line),
            }
        } else if line.contains('/') {
            InsType::ConditionalGoto(count_syllables(line))
        } else if INT_CAP_RE.is_match(line) {
            InsType::Negate
        } else if CAP_RE.is_match(line) {
            InsType::Multiply
        } else if SIMILIE_RE.is_match(line) {
            InsType::Add
        } else if line.contains('?') {
            InsType::PrintChar
        } else if line.contains('.') {
            InsType::PrintValue
        } else if line.contains(',') {
            InsType::Pop
        } else if line.contains('-') {
            InsType::Push
        } else if has_alliteration(line) {
            InsType::Goto
        } else {
            InsType::Store(count_syllables(line))
        };
        let register = if WS_START_RE.is_match(line) {
            Register::Register1
        } else {
            Register::Register0
        };
        let ins = Instruction {
            instruction: ins_type,
            register,
            line: line.trim_end().to_string(),
        };
        lines.push(ins);
        last_line_option = Some(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let exact = count_syllables("antidisestablishmentarianism");
        assert_eq!(exact, 12);
        let approx = count_syllables("supercalifragilisticexpialidocious");
        assert_eq!(approx, 15);
        let misc = count_syllables("a lovely poem");
        assert_eq!(misc, 5);
    }

    #[test]
    fn cond_push() {
        let source = r#"
he thrust every elf
    far back on the shelf
"#
        .trim();

        let tokens = parse(source);
        let mut split = source.trim().split('\n');
        let parsed = vec![
            Instruction {
                instruction: InsType::Goto,
                register: Register::Register0,
                line: split.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::ConditionalPush {
                    prev_syllables: 6,
                    cur_syllables: 5,
                },
                register: Register::Register1,
                line: split.next().unwrap().to_string(),
            },
        ];
        assert_eq!(tokens, parsed);
    }

    #[test]
    fn negate() {
        let source = "tEst";

        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Negate,
            register: Register::Register0,
            line: source.to_string(),
        }];
        assert_eq!(tokens, target);
    }

    #[test]
    fn multiply() {
        let source = "  Test";
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Multiply,
            register: Register::Register1,
            line: source.to_string(),
        }];
        assert_eq!(tokens, target);
    }

    #[test]
    fn add() {
        let source = r#"
fish are like trout
    birds as food
"#
        .trim();

        let mut lines = source.lines();
        let tokens = parse(source);
        let target = vec![
            Instruction {
                instruction: InsType::Add,
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::Add,
                register: Register::Register1,
                line: lines.next().unwrap().to_string(),
            },
        ];
        assert_eq!(tokens, target);
    }

    #[test]
    fn print_char() {
        let source = r#"
oceania directory execution bureaucratic oceania a
printing?
        "#
        .trim();

        let mut lines = source.lines();
        let tokens = parse(source);
        let target = vec![
            Instruction {
                instruction: InsType::Store(21),
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::PrintChar,
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
        ];
        assert_eq!(tokens, target)
    }

    #[test]
    fn print_value() {
        let source = r#"
fish
print. it.
        "#
        .trim();

        let mut lines = source.lines();
        let tokens = parse(source);
        let target = vec![
            Instruction {
                instruction: InsType::Store(1),
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::PrintValue,
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
        ];
        assert_eq!(tokens, target);
    }

    #[test]
    fn pop() {
        let source = "test,";
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Pop,
            register: Register::Register0,
            line: source.to_string(),
        }];
        assert_eq!(tokens, target);
    }

    #[test]
    fn push() {
        let source = "push-it";
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Push,
            register: Register::Register0,
            line: source.to_string(),
        }];
        assert_eq!(tokens, target);
    }

    #[test]
    fn store() {
        let source = "somebody once";
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Store(4),
            register: Register::Register0,
            line: source.to_string(),
        }];
        assert_eq!(tokens, target);
    }

    #[test]
    fn conditional_push() {
        let source = r#"
sombody once told me
the world was gonna roll me
        "#
        .trim();

        let mut lines = source.lines();
        let tokens = parse(source);
        let target = vec![
            Instruction {
                instruction: InsType::Store(6),
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::ConditionalPush {
                    prev_syllables: 6,
                    cur_syllables: 7,
                },
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
        ];

        assert_eq!(tokens, target);
    }

    #[test]
    fn goto() {
        let source = "sells sea shells";
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Goto,
            register: Register::Register0,
            line: source.to_string(),
        }];

        assert_eq!(tokens, target);
    }

    #[test]
    fn noop() {
        let source = r#"
"#;
        let tokens = parse(source);
        let target = vec![Instruction {
            instruction: InsType::Noop,
            register: Register::Register0,
            line: "".to_string(),
        }];

        assert_eq!(tokens, target);
    }

    #[test]
    fn registers() {
        let source = r#"
register zero
    register one
        "#
        .trim();
        let mut lines = source.lines();
        let tokens = parse(source);
        let target = vec![
            Instruction {
                instruction: InsType::Store(5),
                register: Register::Register0,
                line: lines.next().unwrap().to_string(),
            },
            Instruction {
                instruction: InsType::Store(4),
                register: Register::Register1,
                line: lines.next().unwrap().to_string(),
            },
        ];

        assert_eq!(tokens, target);
    }
}
