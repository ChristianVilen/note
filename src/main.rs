mod app;
mod db;
mod ui;

use app::{App, Focus, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::process::Command;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let conn = db::open_db().map_err(|e| anyhow::anyhow!(e))?;
    let mut app = App::new(conn);

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while app.running {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press { continue; }

                    match app.input_mode {
                        InputMode::TitleInput => match key.code {
                            KeyCode::Enter => {
                                app.create_note_from_input();
                                app.input_mode = InputMode::Normal;
                                app.focus = Focus::Editor;
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

                            match app.focus {
                                Focus::Sidebar => match key.code {
                                    KeyCode::Tab => {
                                        if app.editor.is_some() {
                                            app.focus = Focus::Editor;
                                        }
                                    }
                                    KeyCode::Char('q') => {
                                        app.save_editor_content();
                                        app.running = false;
                                    }
                                    KeyCode::Char('j') => app.move_down(),
                                    KeyCode::Char('k') => app.move_up(),
                                    KeyCode::Char('n') => {
                                        app.input_buf.clear();
                                        app.input_mode = InputMode::TitleInput;
                                    }
                                    KeyCode::Char('E') => {
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
                                    KeyCode::Char('?') => app.show_help = !app.show_help,
                                    _ => {}
                                },
                                Focus::Editor => match key.code {
                                    KeyCode::Tab => {
                                        app.focus = Focus::Sidebar;
                                    }
                                    KeyCode::Esc => {
                                        app.focus = Focus::Sidebar;
                                    }
                                    _ => {
                                        if let Some(ref mut ta) = app.editor {
                                            if ta.input(key) {
                                                app.mark_dirty();
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if app.focus == Focus::Editor {
                                if let Some(ref mut ta) = app.editor {
                                    ta.scroll((-3, 0));
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.focus == Focus::Editor {
                                if let Some(ref mut ta) = app.editor {
                                    ta.scroll((3, 0));
                                }
                            }
                        }
                        MouseEventKind::Down(_) => {
                            let border = app.sidebar_width;
                            if mouse.column >= border.saturating_sub(1) && mouse.column <= border + 1 {
                                app.dragging_sidebar = true;
                            }
                        }
                        MouseEventKind::Drag(_) if app.dragging_sidebar => {
                            app.sidebar_width = mouse.column.max(15).min(60);
                        }
                        MouseEventKind::Up(_) => {
                            app.dragging_sidebar = false;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else {
            // No event — check debounced auto-save
            app.check_autosave();
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

    // Save any pending inline edits first
    app.save_editor_content();

    let note = db::get_note(&app.conn, note_id).map_err(|e| anyhow::anyhow!(e))?;
    let tmp = std::env::temp_dir().join(format!("note-{}.md", note_id));
    std::fs::write(&tmp, &note.content)?;

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    Command::new(&editor).arg(&tmp).status()?;

    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    terminal::enable_raw_mode()?;
    terminal.clear()?;

    let content = std::fs::read_to_string(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let title = content.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or(note.title);
    db::update_note(&app.conn, note_id, &title, &content).map_err(|e| anyhow::anyhow!(e))?;
    app.refresh_notes();
    app.load_note_into_editor();
    Ok(())
}
