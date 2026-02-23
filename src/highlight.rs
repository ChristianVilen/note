use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;
use ratatui::style::Color;
use ratatui::text::{Line, Span};

pub struct Highlighter {
    ps: SyntaxSet,
    ts: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight<'a>(&self, content: &str) -> Vec<Line<'a>> {
        let syntax = self.ps.find_syntax_by_extension("md")
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let theme = &self.ts.themes["base16-eighties.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        content.lines().map(|line| {
            let regions = h.highlight_line(line, &self.ps)
                .unwrap_or_default();
            let spans: Vec<Span<'a>> = regions.iter().map(|(style, text)| {
                Span::styled(text.to_string(), to_ratatui_style(style))
            }).collect();
            Line::from(spans)
        }).collect()
    }
}

fn to_ratatui_style(style: &Style) -> ratatui::style::Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    ratatui::style::Style::default().fg(fg)
}
