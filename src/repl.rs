use crate::syntax;

use lalrpop_util::ParseError;
use reedline::{Prompt, PromptEditMode, PromptHistorySearch, ValidationResult, Validator};

use std::borrow::Cow;

pub struct LoxPrompt;

impl Prompt for LoxPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Borrowed(">>> ")
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(&self, _: PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("")
    }
}

pub struct LoxValidator;

impl Validator for LoxValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        match syntax::parse(line) {
            Err(ParseError::UnrecognizedEOF { .. }) => ValidationResult::Incomplete,
            _ => ValidationResult::Complete,
        }
    }
}
