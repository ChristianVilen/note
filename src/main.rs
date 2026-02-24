mod app;
mod db;
mod images;
mod ui;

use app::{App, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind, EnableMouseCapture, DisableMouseCapture, EnableBracketedPaste, DisableBracketedPaste};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui_image::picker::Picker;
use std::io;
use std::process::Command;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let conn = db::open_db().map_err(|e| anyhow::anyhow!(e))?;

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;

    let picker = Picker::from_query_stdio().ok();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(conn, picker);

    while app.running {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press { continue; }

                    match app.input_mode {
                        InputMode::TitleInput => match key.code {
                            KeyCode::Enter => {
                                if let Some(id) = app.create_note_from_input() {
                                    app.input_mode = InputMode::Normal;
                                    launch_editor(&mut terminal, &mut app, Some(id))?;
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
                        InputMode::LeaderF => match key.code {
                            KeyCode::Char('f') => {
                                app.search_query.clear();
                                app.search_results.clear();
                                app.search_selected = 0;
                                app.input_mode = InputMode::SearchTitle;
                            }
                            KeyCode::Char('w') => {
                                app.search_query.clear();
                                app.search_results.clear();
                                app.search_selected = 0;
                                app.input_mode = InputMode::SearchContent;
                            }
                            _ => app.input_mode = InputMode::Normal,
                        },
                        InputMode::SearchTitle => match key.code {
                            KeyCode::Esc => {
                                app.search_query.clear();
                                app.search_results.clear();
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Enter => {
                                if app.select_search_result() {
                                    app.search_query.clear();
                                    app.search_results.clear();
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                            KeyCode::Down => {
                                if app.search_selected + 1 < app.search_results.len() {
                                    app.search_selected += 1;
                                }
                            }
                            KeyCode::Up => {
                                app.search_selected = app.search_selected.saturating_sub(1);
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.search_notes_by_title(&app.search_query.clone());
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.search_notes_by_title(&app.search_query.clone());
                            }
                            _ => {}
                        },
                        InputMode::SearchContent => match key.code {
                            KeyCode::Esc => {
                                app.search_query.clear();
                                app.search_results.clear();
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Enter => {
                                let term = app.search_query.clone();
                                if app.select_search_result() {
                                    app.highlight_term = Some(term);
                                    app.search_query.clear();
                                    app.search_results.clear();
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                            KeyCode::Down => {
                                if app.search_selected + 1 < app.search_results.len() {
                                    app.search_selected += 1;
                                }
                            }
                            KeyCode::Up => {
                                app.search_selected = app.search_selected.saturating_sub(1);
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.search_notes_by_content(&app.search_query.clone());
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.search_notes_by_content(&app.search_query.clone());
                            }
                            _ => {}
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
                                KeyCode::Char('q') => {
                                    app.running = false;
                                }
                                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                                KeyCode::Char('n') => {
                                    app.input_buf.clear();
                                    app.input_mode = InputMode::TitleInput;
                                }
                                KeyCode::Char('e') | KeyCode::Enter => {
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
                                KeyCode::Char('f') => {
                                    app.input_mode = InputMode::LeaderF;
                                }
                                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    handle_paste_image(&mut app);
                                }
                                KeyCode::Char('?') => app.show_help = !app.show_help,
                                KeyCode::Esc => {
                                    app.highlight_term = None;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
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
                        MouseEventKind::ScrollDown => {
                            app.scroll_offset = app.scroll_offset.saturating_add(3);
                        }
                        MouseEventKind::ScrollUp => {
                            app.scroll_offset = app.scroll_offset.saturating_sub(3);
                        }
                        _ => {}
                    }
                }
                Event::Paste(text) => {
                    let trimmed = text.trim();
                    if is_image_path(trimmed) {
                        handle_drop_image(&mut app, trimmed);
                    }
                }
                _ => {}
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture, DisableBracketedPaste)?;
    Ok(())
}

fn handle_paste_image(app: &mut App) {
    match images::paste_image_from_clipboard() {
        Ok(path) => {
            let link = format!("![screenshot]({})", path.display());
            app.append_to_current_note(&link);
            app.set_status("Screenshot pasted");
        }
        Err(_) => {
            app.set_status("No image in clipboard");
        }
    }
}

fn is_image_path(s: &str) -> bool {
    let cleaned = s.trim_matches('\'').trim_matches('"')
        .strip_prefix("file://").unwrap_or(s.trim_matches('\'').trim_matches('"'));
    let lower = percent_decode(cleaned).to_lowercase();
    (lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
        || lower.ends_with(".gif") || lower.ends_with(".webp") || lower.ends_with(".bmp"))
        && !lower.starts_with("http")
}

fn handle_drop_image(app: &mut App, path_str: &str) {
    let cleaned = path_str
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .strip_prefix("file://").unwrap_or(path_str.trim().trim_matches('\'').trim_matches('"'));
    let decoded = percent_decode(cleaned);
    let src = std::path::Path::new(&decoded);
    if !src.exists() {
        app.set_status(&format!("File not found: {}", decoded));
        return;
    }
    let dest_dir = dirs::home_dir().unwrap().join(".note").join("attachments");
    let _ = std::fs::create_dir_all(&dest_dir);
    let filename = src.file_name().unwrap_or_default();
    let dest = dest_dir.join(filename);
    if let Err(_) = std::fs::copy(src, &dest) {
        app.set_status("Failed to copy image");
        return;
    }
    let link = format!("![screenshot]({})", dest.display());
    app.append_to_current_note(&link);
    app.set_status("Image added");
}

fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(0);
            let lo = chars.next().unwrap_or(0);
            if let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo)) {
                result.push((h << 4 | l) as char);
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
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
    let tmp = std::env::temp_dir().join(format!("note-{}.txt", note_id));
    std::fs::write(&tmp, &note.content)?;

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());
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
    app.reload_image_states();
    Ok(())
}
