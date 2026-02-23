use crate::db::{self, Note};
use ratatui_textarea::TextArea;
use rusqlite::Connection;
use std::time::Instant;

#[derive(PartialEq)]
pub enum Focus {
    Sidebar,
    Editor,
}

pub enum InputMode {
    Normal,
    TitleInput,
    ConfirmDelete,
}

pub struct App {
    pub conn: Connection,
    pub notes: Vec<Note>,
    pub selected: usize,
    pub running: bool,
    pub show_archived: bool,
    pub sidebar_width: u16,
    pub input_mode: InputMode,
    pub input_buf: String,
    pub show_help: bool,
    pub dragging_sidebar: bool,
    pub focus: Focus,
    pub editor: Option<TextArea<'static>>,
    pub editing_note_id: Option<i64>,
    pub dirty: bool,
    pub last_edit: Option<Instant>,
}

impl App {
    pub fn new(conn: Connection) -> Self {
        let notes = db::list_notes(&conn, false).unwrap_or_default();
        let mut app = Self {
            conn,
            notes,
            selected: 0,
            running: true,
            show_archived: false,
            sidebar_width: 30,
            input_mode: InputMode::Normal,
            input_buf: String::new(),
            show_help: false,
            dragging_sidebar: false,
            focus: Focus::Sidebar,
            editor: None,
            editing_note_id: None,
            dirty: false,
            last_edit: None,
        };
        app.load_note_into_editor();
        app
    }

    pub fn load_note_into_editor(&mut self) {
        if let Some(note) = self.notes.get(self.selected) {
            let mut ta = TextArea::from(note.content.lines());
            ta.set_cursor_line_style(ratatui::style::Style::default());
            self.editing_note_id = Some(note.id);
            self.editor = Some(ta);
        } else {
            self.editor = None;
            self.editing_note_id = None;
        }
        self.dirty = false;
        self.last_edit = None;
    }

    pub fn save_editor_content(&mut self) {
        if !self.dirty { return; }
        if let (Some(ta), Some(id)) = (&self.editor, self.editing_note_id) {
            let content = ta.lines().join("\n");
            let title = content.lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").trim().to_string())
                .unwrap_or_else(|| {
                    self.notes.iter().find(|n| n.id == id)
                        .map(|n| n.title.clone())
                        .unwrap_or_default()
                });
            let _ = db::update_note(&self.conn, id, &title, &content);
            self.dirty = false;
            self.last_edit = None;
            self.refresh_notes();
        }
    }

    pub fn refresh_notes(&mut self) {
        self.notes = db::list_notes(&self.conn, self.show_archived).unwrap_or_default();
        if self.selected >= self.notes.len() && !self.notes.is_empty() {
            self.selected = self.notes.len() - 1;
        }
    }

    pub fn selected_note(&self) -> Option<&Note> {
        self.notes.get(self.selected)
    }

    pub fn move_down(&mut self) {
        if !self.notes.is_empty() && self.selected < self.notes.len() - 1 {
            self.save_editor_content();
            self.selected += 1;
            self.load_note_into_editor();
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.save_editor_content();
            self.selected -= 1;
            self.load_note_into_editor();
        }
    }

    pub fn toggle_archive(&mut self) {
        if let Some(note) = self.notes.get(self.selected) {
            let id = note.id;
            if note.archived {
                let _ = db::unarchive_note(&self.conn, id);
            } else {
                let _ = db::archive_note(&self.conn, id);
            }
            self.dirty = false;
            self.refresh_notes();
            self.load_note_into_editor();
        }
    }

    pub fn toggle_show_archived(&mut self) {
        self.save_editor_content();
        self.show_archived = !self.show_archived;
        self.refresh_notes();
        self.load_note_into_editor();
    }

    pub fn delete_selected(&mut self) {
        if let Some(note) = self.notes.get(self.selected) {
            let _ = db::delete_note(&self.conn, note.id);
            self.dirty = false;
            self.refresh_notes();
            self.load_note_into_editor();
        }
    }

    pub fn create_note_from_input(&mut self) -> Option<i64> {
        let title = self.input_buf.trim().to_string();
        if title.is_empty() { return None; }
        self.save_editor_content();
        let id = db::create_note(&self.conn, &title).ok()?;
        self.refresh_notes();
        self.selected = 0;
        self.load_note_into_editor();
        Some(id)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
    }

    pub fn check_autosave(&mut self) {
        if self.dirty {
            if let Some(t) = self.last_edit {
                if t.elapsed().as_secs() >= 1 {
                    self.save_editor_content();
                }
            }
        }
    }
}
