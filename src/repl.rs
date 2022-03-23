use crate::syntax;

use lalrpop_util::ParseError;
use nu_ansi_term as nat;
use reedline as rl;
use tree_sitter_highlight as tsh;
use tree_sitter_lox as tsl;

use std::borrow::Cow;

struct PaletteItem<'a> {
    name: &'a str,
    fg: nat::Color,
}

const PALETTE: &[PaletteItem] = &[
    PaletteItem { name: "", fg: nat::Color::White },
    PaletteItem { name: "conditional", fg: nat::Color::LightPurple },
    PaletteItem { name: "constant", fg: nat::Color::LightCyan },
    PaletteItem { name: "field", fg: nat::Color::LightBlue },
    PaletteItem { name: "function", fg: nat::Color::LightBlue },
    PaletteItem { name: "keyword.function", fg: nat::Color::LightPurple },
    PaletteItem { name: "keyword.return", fg: nat::Color::LightPurple },
    PaletteItem { name: "keyword", fg: nat::Color::LightPurple },
    PaletteItem { name: "method", fg: nat::Color::LightBlue },
    PaletteItem { name: "number", fg: nat::Color::LightCyan },
    PaletteItem { name: "operator", fg: nat::Color::White },
    PaletteItem { name: "parameter", fg: nat::Color::LightRed }, // FIXME: more than one parameter doesn't highlight
    PaletteItem { name: "punctuation.bracket", fg: nat::Color::White },
    PaletteItem { name: "punctuation.delimiter", fg: nat::Color::White },
    PaletteItem { name: "repeat", fg: nat::Color::LightPurple },
    PaletteItem { name: "string", fg: nat::Color::LightGreen },
    PaletteItem { name: "type", fg: nat::Color::LightYellow },
];

pub struct Highlighter {
    config: tsh::HighlightConfiguration,
}

impl Highlighter {
    pub fn new() -> Self {
        let highlight_names = PALETTE.iter().map(|item| item.name).collect::<Vec<_>>();
        let mut config =
            tsh::HighlightConfiguration::new(tsl::language(), tsl::HIGHLIGHTS_QUERY, "", "")
                .expect("failed to create highlight configuration");
        config.configure(&highlight_names);
        Self { config }
    }
}

impl rl::Highlighter for Highlighter {
    fn highlight(&self, line: &str, _: usize) -> rl::StyledText {
        let mut highlighter = tsh::Highlighter::new();
        let highlights =
            highlighter.highlight(&self.config, line.as_bytes(), None, |_| None).unwrap();

        let mut output = rl::StyledText::new();
        let mut fg = PALETTE[0].fg;

        for event in highlights {
            match event.unwrap() {
                tsh::HighlightEvent::HighlightStart(highlight) => {
                    fg = PALETTE[highlight.0].fg;
                }
                tsh::HighlightEvent::Source { start, end } => {
                    let style = nat::Style::new().fg(fg);
                    let text = line[start..end].to_string();
                    output.push((style, text));
                }
                tsh::HighlightEvent::HighlightEnd => {
                    fg = PALETTE[0].fg;
                }
            }
        }

        output
    }
}

pub struct Prompt;

impl rl::Prompt for Prompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Borrowed(">>> ")
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _: rl::PromptEditMode) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(&self, _: rl::PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("")
    }
}

pub struct Validator;

impl rl::Validator for Validator {
    fn validate(&self, line: &str) -> rl::ValidationResult {
        match syntax::parse(line) {
            Err(ParseError::UnrecognizedEOF { .. }) => rl::ValidationResult::Incomplete,
            _ => rl::ValidationResult::Complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Highlighter;

    #[test]
    fn highlight_configuration() {
        // This should not panic.
        let _ = Highlighter::new();
    }
}
