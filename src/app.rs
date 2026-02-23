use crate::db::{self, Note};
use rusqlite::Connection;

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
    pub focus_mode: bool,
    pub sidebar_width: u16,
    pub scroll_offset: u16,
    pub input_mode: InputMode,
    pub input_buf: String,
    pub show_help: bool,
    pub dragging_sidebar: bool,
    pub highlight: crate::highlight::Highlighter,
}

impl App {
    pub fn new(conn: Connection) -> Self {
        let notes = db::list_notes(&conn, false).unwrap_or_default();
        Self {
            conn,
            notes,
            selected: 0,
            running: true,
            show_archived: false,
            focus_mode: false,
            sidebar_width: 30,
            scroll_offset: 0,
            input_mode: InputMode::Normal,
            input_buf: String::new(),
            show_help: false,
            dragging_sidebar: false,
            highlight: crate::highlight::Highlighter::new(),
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
            self.selected += 1;
            self.scroll_offset = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll_offset = 0;
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
            self.refresh_notes();
        }
    }

    pub fn toggle_show_archived(&mut self) {
        self.show_archived = !self.show_archived;
        self.refresh_notes();
    }

    pub fn delete_selected(&mut self) {
        if let Some(note) = self.notes.get(self.selected) {
            let _ = db::delete_note(&self.conn, note.id);
            self.refresh_notes();
        }
    }

    pub fn create_note_from_input(&mut self) -> Option<i64> {
        let title = self.input_buf.trim().to_string();
        if title.is_empty() { return None; }
        let id = db::create_note(&self.conn, &title).ok()?;
        self.refresh_notes();
        self.selected = 0; // newest note is first (ordered by updated_at DESC)
        Some(id)
    }


}
