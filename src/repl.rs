use anyhow::{Context, Result};
use nu_ansi_term::{Color, Style};
use reedline::{
    EditCommand, Emacs, FileBackedHistory, KeyCode, KeyModifiers, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, StyledText, ValidationResult,
};
use tree_sitter_highlight::{self, HighlightConfiguration, HighlightEvent};
use tree_sitter_lox::{self, HIGHLIGHTS_QUERY};

use std::borrow::Cow;

pub fn editor() -> Result<Reedline> {
    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );

    let highlighter = Box::new(Highlighter::new()?);

    let data_dir = dirs::data_dir().context("could not find data directory")?;
    let history_path = data_dir.join("lox/history.txt");
    let history = Box::new(
        FileBackedHistory::with_file(10000, history_path.clone())
            .with_context(|| format!("could not open history file: {}", history_path.display()))?,
    );

    let validator = Box::new(Validator);

    let editor = Reedline::create()
        .with_edit_mode(Box::new(Emacs::new(keybindings)))
        .with_highlighter(highlighter)
        .with_history(history)
        .with_validator(validator);
    Ok(editor)
}

struct PaletteItem<'a> {
    name: &'a str,
    fg: Color,
}

// Color scheme inspired by base16-google-dark.
//
// The base16 style guide tells you which base16 color code to use for each
// language construct:
// https://github.com/chriskempson/base16/blob/39fb23df970d4d6190d000271dec260250986012/styling.md
//
// The base16-vim theme contains the 8-bit ANSI codes associated with each
// base16 color code (assume we are not working in a 256-color terminal):
// https://github.com/chriskempson/base16-vim/blob/c156b909af619cdd097d8d1e2cd1dce1f45dfba1/colors/base16-google-dark.vim#L52
//
// This page gives you an idea of what color is associated with a particular
// 8-bit ANSI code:
// https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
//
// Since this color scheme makes use of both Red and LightRed, we replace
// LightRed with LightCyan to better distinguish between the two.
//
// Then, we replace each color with its high-intensity variant, since the
// standard colors can be harder to read on some terminals.
//
const PALETTE: &[PaletteItem] = &[
    PaletteItem { name: "", fg: Color::LightGray },
    PaletteItem { name: "class", fg: Color::LightYellow },
    PaletteItem { name: "comment", fg: Color::DarkGray },
    PaletteItem { name: "constant", fg: Color::LightCyan },
    PaletteItem { name: "function", fg: Color::LightBlue },
    PaletteItem { name: "keyword", fg: Color::LightPurple },
    PaletteItem { name: "operator", fg: Color::LightGray },
    PaletteItem { name: "punctuation", fg: Color::LightGray },
    PaletteItem { name: "string", fg: Color::LightGreen },
    PaletteItem { name: "variable", fg: Color::LightRed },
];

struct Highlighter {
    config: HighlightConfiguration,
}

impl Highlighter {
    pub fn new() -> Result<Self> {
        let highlight_names = PALETTE.iter().map(|item| item.name).collect::<Vec<_>>();
        let mut config =
            HighlightConfiguration::new(tree_sitter_lox::language(), HIGHLIGHTS_QUERY, "", "")
                .context("failed to create highlight configuration")?;
        config.configure(&highlight_names);
        Ok(Self { config })
    }
}

impl reedline::Highlighter for Highlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        let mut output = StyledText::new();

        let mut highlighter = tree_sitter_highlight::Highlighter::new();
        let highlights = match highlighter.highlight(&self.config, line.as_bytes(), None, |_| None)
        {
            Ok(highlights) => highlights,
            Err(_) => {
                let style = Style::new().fg(PALETTE[0].fg);
                output.push((style, line.to_string()));
                return output;
            }
        };

        let mut curr_fg = PALETTE[0].fg;
        let mut curr_end = 0;

        for event in highlights {
            match event {
                Ok(HighlightEvent::HighlightStart(highlight)) => {
                    curr_fg = PALETTE[highlight.0].fg;
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    let style = Style::new().fg(curr_fg);
                    let text = line[start..end].to_string();
                    output.push((style, text));
                    curr_end = end;
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    curr_fg = PALETTE[0].fg;
                }
                Err(_) => {
                    let style = Style::new().fg(PALETTE[0].fg);
                    let text = line.get(curr_end..).unwrap_or_default().to_string();
                    output.push((style, text));
                    break;
                }
            }
        }

        output
    }
}

struct Validator;

impl reedline::Validator for Validator {
    fn validate(&self, line: &str) -> ValidationResult {
        if lox_syntax::is_complete(line) {
            ValidationResult::Complete
        } else {
            ValidationResult::Incomplete
        }
    }
}

pub struct Prompt;

impl reedline::Prompt for Prompt {
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
