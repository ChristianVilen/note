use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use crate::app::{App, Focus, InputMode};

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

    let border_style = if app.focus == Focus::Sidebar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::RIGHT).title(header).border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    let title = app.selected_note()
        .map(|n| format!(" {} ", n.title))
        .unwrap_or_default();
    let is_focused = app.focus == Focus::Editor;

    if let Some(ref mut ta) = app.editor {
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        ta.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style)
        );

        if is_focused {
            ta.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            ta.set_cursor_style(Style::default());
        }

        f.render_widget(&*ta, area);
    } else {
        let msg = Paragraph::new("No notes yet. Press 'n' to create one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(msg, area);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let count = app.notes.len();
    let filter = if app.show_archived { " [showing archived]" } else { "" };
    let dirty = if app.dirty { " [modified]" } else { "" };
    let focus = if app.focus == Focus::Editor { " [editing]" } else { "" };
    let hints = " Tab:focus  j/k:nav  n:new  a:archive  E:ext-editor  d:delete  ?:help  q:quit";
    let right = format!(" {count} notes{filter}{dirty}{focus} ");
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
    let popup = centered_rect(60, 16, area);
    f.render_widget(Clear, popup);
    let help = vec![
        "Tab            Toggle sidebar / editor focus",
        "j / k          Navigate notes (sidebar)",
        "E              Edit in external $EDITOR",
        "n              Create new note",
        "a              Archive / unarchive note",
        "A              Toggle show archived",
        "d              Delete note",
        "Ctrl+U / Ctrl+R  Undo / Redo (editor)",
        "Drag border    Resize sidebar",
        "?              Toggle this help",
        "q              Quit (sidebar focus)",
    ];
    let lines: Vec<Line> = help.iter().map(|l| Line::from(*l)).collect();
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Keybindings "))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(para, popup);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = (area.width.saturating_sub(popup_width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;
    Rect::new(x, y, popup_width, height)
}
