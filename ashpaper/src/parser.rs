use cmudict::Cmudict;
use lazy_static::lazy_static;
use regex::Regex;

/// represents a single line and its metadata
#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
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

pub struct Line {
    pub instruction: Instruction,
    pub end_word: cmudict::Rule,
}

lazy_static! {
    static ref NUM_RE: Regex = Regex::new(r"[0-9]").unwrap();
    static ref INT_CAP_RE: Regex = Regex::new(r"\b\S+[A-Z]\S+\b").unwrap();
    static ref CAP_RE: Regex = Regex::new(r"\b[A-Z][^A-Z]+\b").unwrap();
    static ref SIMILIE_RE: Regex = Regex::new(r"\b(like|as)\b").unwrap();
    static ref WS_START_RE: Regex = Regex::new(r"^\s").unwrap();
}

pub struct Parser {
    cmudict: Cmudict,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            // TODO: figure out a way to properly bridge the gap between failure and anyhow (or fork the Cmudit source to thisError)
            cmudict: Cmudict::new("cmudict.dict").unwrap(),
        }
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

    fn count_word_syllables(&self, word: &str) -> usize {
        // TODO: add handling if not in cmudict
        let rule = self.cmudict.get(word).unwrap();
        let pronunciation = rule.pronunciation();
        // TODO: inconsitent with python interpeter due to varying lengths of cmudict pronunciations
        let count = pronunciation.iter().filter(|po| po.is_syllable()).count();
        count
    }

    fn count_syllables(&self, input: &str) -> usize {
        input.split(' ').map(|w| self.count_word_syllables(w)).sum()
    }

    fn check_end_rhyme(&self, last_line_option: Option<&str>, cur_line: &str) -> bool {
        if let Some(last_line) = last_line_option {
            // end-rhyme handling
            if let (Some(last_line_word), Some(last_word)) = (
                last_line.split(' ').rev().next(),
                cur_line.split(' ').rev().next(),
            ) {
                if let (Some(last_line_rule), Some(last_rule)) = (
                    self.cmudict.get(last_line_word),
                    self.cmudict.get(last_word),
                ) {
                    return cmudict::rhymes(&last_line_rule, &last_rule);
                }
            }
        }
        false
    }

    pub fn parse(&self, input: &str) -> Vec<Instruction> {
        let mut last_line_option: Option<&str> = None;
        let mut lines = Vec::new();
        for line in input.split('\n') {
            // short-circuit on noop
            if line == "" {
                continue;
            }
            // everything else
            let ins = if line.contains('/') {
                Instruction::Compare
            } else if INT_CAP_RE.is_match(line) {
                Instruction::Negate
            } else if CAP_RE.is_match(line) {
                Instruction::Multiply
            } else if SIMILIE_RE.is_match(line) {
                Instruction::Add
            } else if line.contains('?') {
                Instruction::PrintChar
            } else if line.contains('.') {
                Instruction::Print
            } else if line.contains(',') {
                Instruction::Pop
            } else if line.contains('-') {
                Instruction::Push
            } else if Self::has_alliteration(line) {
                Instruction::Goto
            } else if self.check_end_rhyme(last_line_option, line) {
                Instruction::LineConditional {
                    prev_syllables: self.count_syllables(last_line_option.unwrap()),
                    cur_syllables: self.count_syllables(line),
                }
            } else {
                Instruction::Store(self.count_syllables(line))
            };
            lines.push(ins);
            last_line_option = Some(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_alliteration() {
        assert!(Parser::has_alliteration("she sells sea shells"));
        assert!(!Parser::has_alliteration("no alliteration here"));
        assert!(!Parser::has_alliteration("one"));
        assert!(!Parser::has_alliteration(""));
    }

    #[test]
    fn rhyming() {
        let source = r#"
he thrust every elf
far back on the shelf
"#;
        let parser = Parser::new();
        let tokens = parser.parse(source);
        let parsed = vec![
            Instruction::Goto,
            Instruction::LineConditional {
                prev_syllables: 6,
                cur_syllables: 5,
            },
        ];
        assert_eq!(tokens, parsed);
    }
}
