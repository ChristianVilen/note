mod app;
mod db;
mod highlight;
mod ui;

use app::{App, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let conn = db::open_db().map_err(|e| anyhow::anyhow!(e))?;
    let mut app = App::new(conn);

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while app.running {
        terminal.draw(|f| ui::draw(f, &app))?;

        match event::read()? {
            Event::Key(key) => {
            if key.kind != KeyEventKind::Press { continue; }

            match app.input_mode {
                InputMode::TitleInput => match key.code {
                    KeyCode::Enter => {
                        let id = app.create_note_from_input();
                        app.input_mode = InputMode::Normal;
                        if let Some(note_id) = id {
                            launch_editor(&mut terminal, &mut app, Some(note_id))?;
                        }
                    }
                    KeyCode::Esc => {
                        app.input_buf.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Backspace => { app.input_buf.pop(); }
                    KeyCode::Char(c) => app.input_buf.push(c),
                    _ => {}
                },
                InputMode::ConfirmDelete => match key.code {
                    KeyCode::Char('y') => {
                        app.delete_selected();
                        app.input_mode = InputMode::Normal;
                    }
                    _ => app.input_mode = InputMode::Normal,
                },
                InputMode::Normal => {
                    if app.show_help {
                        match key.code {
                            KeyCode::Char('?') | KeyCode::Esc => app.show_help = false,
                            _ => {}
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') => app.running = false,
                        KeyCode::Char('j') => app.move_down(),
                        KeyCode::Char('k') => app.move_up(),
                        KeyCode::Char('n') => {
                            app.input_buf.clear();
                            app.input_mode = InputMode::TitleInput;
                        }
                        KeyCode::Char('e') => {
                            launch_editor(&mut terminal, &mut app, None)?;
                        }
                        KeyCode::Char('a') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.toggle_archive();
                        }
                        KeyCode::Char('A') => app.toggle_show_archived(),
                        KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                            if app.selected_note().is_some() {
                                app.input_mode = InputMode::ConfirmDelete;
                            }
                        }
                        KeyCode::Tab => app.focus_mode = !app.focus_mode,
                        KeyCode::Char('?') => app.show_help = !app.show_help,
                        _ => {}
                    }
                }
            }
            }
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_add(3);
                    }
                    MouseEventKind::Down(_) if !app.focus_mode => {
                        // Start drag if clicking near sidebar border
                        let border = app.sidebar_width;
                        if mouse.column >= border.saturating_sub(1) && mouse.column <= border + 1 {
                            app.dragging_sidebar = true;
                        }
                    }
                    MouseEventKind::Drag(_) if app.dragging_sidebar => {
                        let new_width = mouse.column.max(15).min(60);
                        app.sidebar_width = new_width;
                    }
                    MouseEventKind::Up(_) => {
                        app.dragging_sidebar = false;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn launch_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    new_note_id: Option<i64>,
) -> anyhow::Result<()> {
    let note_id = new_note_id.or_else(|| app.selected_note().map(|n| n.id));
    let note_id = match note_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let note = db::get_note(&app.conn, note_id).map_err(|e| anyhow::anyhow!(e))?;
    let tmp = std::env::temp_dir().join(format!("note-{}.md", note_id));
    std::fs::write(&tmp, &note.content)?;

    // Suspend TUI
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    Command::new(&editor).arg(&tmp).status()?;

    // Resume TUI
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    terminal::enable_raw_mode()?;
    terminal.clear()?;

    let content = std::fs::read_to_string(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    // Save with title extraction
    let title = content.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or(note.title);
    db::update_note(&app.conn, note_id, &title, &content).map_err(|e| anyhow::anyhow!(e))?;
    app.refresh_notes();
    Ok(())
}
