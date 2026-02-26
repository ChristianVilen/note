use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui_image::StatefulImage;
use crate::app::{App, InputMode};
use crate::images;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ]).split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    let panes = Layout::horizontal([
        Constraint::Length(app.sidebar_width),
        Constraint::Min(1),
    ]).split(main_area);
    draw_sidebar(f, app, panes[0]);
    draw_content(f, app, panes[1]);
    draw_selection_highlight(f, app);

    draw_status_bar(f, app, status_area);

    if let InputMode::TitleInput = app.input_mode {
        draw_input(f, app, main_area);
    }
    if let InputMode::ConfirmDelete = app.input_mode {
        draw_confirm_delete(f, app, main_area);
    }
    if app.show_help {
        draw_help(f, main_area);
    }
    match app.input_mode {
        InputMode::SearchTitle => draw_search_popup(f, app, main_area, " Find note (title) "),
        InputMode::SearchContent => draw_search_popup(f, app, main_area, " Grep note (content) "),
        _ => {}
    }
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let header = if app.show_archived { " Notes [+archived] " } else { " Notes " };
    let items: Vec<ListItem> = app.notes.iter().map(|note| {
        let ts = &note.updated_at[..16.min(note.updated_at.len())];
        let mut style = Style::default();
        if note.archived {
            style = style.fg(Color::DarkGray);
        }
        let title = if note.title.len() > (app.sidebar_width as usize - 4) {
            format!("{}…", &note.title[..app.sidebar_width as usize - 5])
        } else {
            note.title.clone()
        };
        ListItem::new(vec![
            Line::from(Span::styled(title, style.add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(format!(" {ts}"), style.fg(Color::DarkGray))),
        ])
    }).collect();

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let list = List::new(items)
        .block(Block::default().borders(Borders::RIGHT).title(header).border_style(Style::default().fg(Color::Yellow)))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    let title = app.selected_note()
        .map(|n| format!(" {} ", n.title))
        .unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.preview_area = Some(inner);

    let content = match app.selected_note() {
        Some(n) => &n.content,
        None => {
            let msg = Paragraph::new("No notes yet. Press 'n' to create one.")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(msg, inner);
            return;
        }
    };

    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let image_lines = images::find_image_lines(&lines);

    // No images or no picker — render as plain markdown text
    if image_lines.is_empty() || app.picker.is_none() {
        let styled = style_markdown(&lines, app.highlight_term.as_deref());
        let para = Paragraph::new(styled).wrap(Wrap { trim: false }).scroll((app.scroll_offset, 0));
        f.render_widget(para, inner);
        return;
    }

    let image_line_set: std::collections::HashSet<usize> = image_lines.iter().map(|(i, _)| *i).collect();
    let image_height: u16 = 10;

    let mut constraints: Vec<Constraint> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut text_start: Option<usize> = None;

    for i in 0..lines.len() {
        if image_line_set.contains(&i) {
            if let Some(start) = text_start.take() {
                let count = i - start;
                constraints.push(Constraint::Length(count as u16));
                segments.push(Segment::Text(start, i));
            }
            let path = image_lines.iter().find(|(li, _)| *li == i).map(|(_, p)| p.clone()).unwrap();
            constraints.push(Constraint::Length(image_height));
            segments.push(Segment::Image(path));
        } else if text_start.is_none() {
            text_start = Some(i);
        }
    }
    if let Some(start) = text_start {
        let count = lines.len() - start;
        constraints.push(Constraint::Length(count as u16));
        segments.push(Segment::Text(start, lines.len()));
    }

    let total_height: u16 = constraints.iter().map(|c| match c {
        Constraint::Length(h) => *h,
        _ => 0,
    }).sum();
    if total_height > inner.height {
        // Fallback: just render text if it doesn't fit
        let styled = style_markdown(&lines, app.highlight_term.as_deref());
        let para = Paragraph::new(styled).wrap(Wrap { trim: false }).scroll((app.scroll_offset, 0));
        f.render_widget(para, inner);
        return;
    }

    constraints.push(Constraint::Min(0));
    let chunk_areas = Layout::vertical(&constraints).split(inner);

    for (idx, seg) in segments.iter().enumerate() {
        match seg {
            Segment::Text(start, end) => {
                let styled = style_markdown(&lines[*start..*end], app.highlight_term.as_deref());
                let para = Paragraph::new(styled);
                f.render_widget(para, chunk_areas[idx]);
            }
            Segment::Image(path) => {
                if let Some(state) = app.image_states.get_mut(path) {
                    let img_widget = StatefulImage::default();
                    f.render_stateful_widget(img_widget, chunk_areas[idx], state);
                } else {
                    let placeholder = Paragraph::new("[image not found]")
                        .style(Style::default().fg(Color::DarkGray));
                    f.render_widget(placeholder, chunk_areas[idx]);
                }
            }
        }
    }
}

enum Segment {
    Text(usize, usize),
    Image(std::path::PathBuf),
}

fn draw_selection_highlight(f: &mut Frame, app: &App) {
    let (mut start, mut end) = match (app.selection_start, app.selection_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return,
    };
    let pa = match app.preview_area {
        Some(a) => a,
        None => return,
    };
    if start > end { std::mem::swap(&mut start, &mut end); }
    let hl = Style::default().bg(Color::Blue).fg(Color::White);
    let buf = f.buffer_mut();
    for row in start.0..=end.0 {
        let abs_y = pa.y + row;
        if abs_y >= pa.y + pa.height { break; }
        let c0 = if row == start.0 { start.1 } else { 0 };
        let c1 = if row == end.0 { end.1 } else { pa.width };
        for col in c0..c1 {
            let abs_x = pa.x + col;
            if abs_x >= pa.x + pa.width { break; }
            buf[(abs_x, abs_y)].set_style(hl);
        }
    }
}

/// Basic markdown styling for the preview pane.
fn style_markdown<'a>(lines: &'a [String], highlight: Option<&str>) -> Vec<Line<'a>> {
    let hl = highlight.map(|h| h.to_lowercase());
    lines.iter().map(|line| {
        let base_line = if line.starts_with("# ") {
            Line::from(Span::styled(&line[2..], Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        } else if line.starts_with("## ") {
            Line::from(Span::styled(&line[3..], Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        } else if line.starts_with("### ") {
            Line::from(Span::styled(&line[4..], Style::default().fg(Color::Cyan)))
        } else if line.starts_with("- ") || line.starts_with("* ") {
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Yellow)),
                Span::raw(&line[2..]),
            ])
        } else if line.starts_with("> ") {
            Line::from(Span::styled(line.as_str(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)))
        } else if line.starts_with("```") {
            Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Green)))
        } else if line.starts_with("![") {
            Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Blue)))
        } else {
            Line::from(line.as_str())
        };
        if let Some(ref hl_term) = hl {
            highlight_line(base_line, hl_term)
        } else {
            base_line
        }
    }).collect()
}

fn highlight_line<'a>(line: Line<'a>, term: &str) -> Line<'a> {
    let hl_style = Style::default().fg(Color::Black).bg(Color::Yellow);
    let mut new_spans = Vec::new();
    for span in line.spans {
        let text = span.content.to_string();
        let lower = text.to_lowercase();
        if !lower.contains(term) {
            new_spans.push(Span::styled(text, span.style));
            continue;
        }
        let mut remaining = text.as_str();
        let mut lower_remaining = lower.as_str();
        while let Some(pos) = lower_remaining.find(term) {
            if pos > 0 {
                new_spans.push(Span::styled(remaining[..pos].to_string(), span.style));
            }
            new_spans.push(Span::styled(remaining[pos..pos + term.len()].to_string(), hl_style));
            remaining = &remaining[pos + term.len()..];
            lower_remaining = &lower_remaining[pos + term.len()..];
        }
        if !remaining.is_empty() {
            new_spans.push(Span::styled(remaining.to_string(), span.style));
        }
    }
    Line::from(new_spans)
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let count = app.notes.len();
    let filter = if app.show_archived { " [showing archived]" } else { "" };
    let version = env!("CARGO_PKG_VERSION");

    if let Some(status) = app.current_status() {
        let bar = Line::from(Span::styled(format!(" {status}"), Style::default().fg(Color::Green)));
        f.render_widget(Paragraph::new(bar), area);
        return;
    }

    if matches!(app.input_mode, InputMode::LeaderF) {
        let bar = Line::from(Span::styled(" f-…", Style::default().fg(Color::Yellow)));
        f.render_widget(Paragraph::new(bar), area);
        return;
    }

    let hints = " j/k:nav  e:edit  n:new  ff:find  fw:grep  a:archive  d:del  ?:help  q:quit";
    let right = format!(" {count} notes{filter}  v{version} ");
    let left_width = area.width.saturating_sub(right.len() as u16) as usize;
    let left = if hints.len() > left_width {
        format!("{}", &hints[..left_width])
    } else {
        format!("{:<width$}", hints, width = left_width)
    };
    let bar = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::DarkGray)),
        Span::styled(right, Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(bar), area);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 3, area);
    f.render_widget(Clear, popup);
    let input = Paragraph::new(app.input_buf.as_str())
        .block(Block::default().borders(Borders::ALL).title(" New note title "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input, popup);
    f.set_cursor_position((popup.x + app.input_buf.len() as u16 + 1, popup.y + 1));
}

fn draw_confirm_delete(f: &mut Frame, app: &App, area: Rect) {
    let title = app.selected_note().map(|n| n.title.as_str()).unwrap_or("?");
    let popup = centered_rect(50, 3, area);
    f.render_widget(Clear, popup);
    let msg = Paragraph::new(format!("Delete '{title}'? (y/n)"))
        .block(Block::default().borders(Borders::ALL).title(" Confirm "))
        .style(Style::default().fg(Color::Red));
    f.render_widget(msg, popup);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 18, area);
    f.render_widget(Clear, popup);
    let help = vec![
        "j / k / ↑ / ↓  Navigate notes",
        "e / Enter      Edit note in $EDITOR (nvim)",
        "n              Create new note",
        "ff             Find note by title",
        "fw             Grep note content",
        "a              Archive / unarchive note",
        "A              Toggle show archived",
        "d              Delete note",
        "Ctrl+S         Paste screenshot from clipboard",
        "y              Yank (copy) selection",
        "Esc            Clear search highlight",
        "Drag border    Resize sidebar",
        "?              Toggle this help",
        "q              Quit",
    ];
    let lines: Vec<Line> = help.iter().map(|l| Line::from(*l)).collect();
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Keybindings "))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(para, popup);
}

fn draw_search_popup(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let height = 12u16.min(area.height.saturating_sub(4));
    let popup = centered_rect(60, height, area);
    f.render_widget(Clear, popup);

    let block = Block::default().borders(Borders::ALL).title(title)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if inner.height < 2 { return; }

    let query_area = Rect { height: 1, ..inner };
    let results_area = Rect { y: inner.y + 1, height: inner.height - 1, ..inner };

    let query_line = Line::from(vec![
        Span::styled("❯ ", Style::default().fg(Color::Yellow)),
        Span::raw(&app.search_query),
    ]);
    f.render_widget(Paragraph::new(query_line), query_area);
    f.set_cursor_position((query_area.x + 2 + app.search_query.len() as u16, query_area.y));

    let items: Vec<ListItem> = app.search_results.iter().map(|(_, title, snippet)| {
        let mut lines = vec![Line::from(Span::styled(title.as_str(), Style::default().add_modifier(Modifier::BOLD)))];
        if let Some(s) = snippet {
            let display: String = if s.len() > results_area.width as usize - 4 {
                format!("  {}…", &s[..results_area.width as usize - 5])
            } else {
                format!("  {s}")
            };
            lines.push(Line::from(Span::styled(display, Style::default().fg(Color::DarkGray))));
        }
        ListItem::new(lines)
    }).collect();

    let mut state = ListState::default();
    if !app.search_results.is_empty() {
        state.select(Some(app.search_selected));
    }

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_stateful_widget(list, results_area, &mut state);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = (area.width.saturating_sub(popup_width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;
    Rect::new(x, y, popup_width, height)
}
