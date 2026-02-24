use crate::db::{self, Note};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub enum InputMode {
    Normal,
    TitleInput,
    ConfirmDelete,
    LeaderF,
    SearchTitle,
    SearchContent,
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
    pub picker: Option<Picker>,
    pub image_states: HashMap<PathBuf, StatefulProtocol>,
    pub status_msg: Option<(String, Instant)>,
    pub search_query: String,
    pub search_results: Vec<(usize, String, Option<String>)>,
    pub search_selected: usize,
    pub highlight_term: Option<String>,
    pub scroll_offset: u16,
}

impl App {
    pub fn new(conn: Connection, picker: Option<Picker>) -> Self {
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
            picker,
            image_states: HashMap::new(),
            status_msg: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            highlight_term: None,
            scroll_offset: 0,
        };
        app.reload_image_states();
        app
    }

    pub fn reload_image_states(&mut self) {
        self.scroll_offset = 0;
        let picker = match &self.picker {
            Some(p) => p,
            None => { self.image_states.clear(); return; }
        };
        let content = match self.notes.get(self.selected) {
            Some(n) => &n.content,
            None => { self.image_states.clear(); return; }
        };
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let image_lines = crate::images::find_image_lines(&lines);
        let needed: std::collections::HashSet<PathBuf> = image_lines.iter().map(|(_, p)| p.clone()).collect();
        self.image_states.retain(|k, _| needed.contains(k));
        for (_, path) in image_lines {
            if self.image_states.contains_key(&path) { continue; }
            if let Ok(dyn_img) = image::ImageReader::open(&path).and_then(|r| Ok(r.decode().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)) {
                let proto = picker.new_resize_protocol(dyn_img);
                self.image_states.insert(path, proto);
            }
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
            self.reload_image_states();
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.reload_image_states();
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
            self.reload_image_states();
        }
    }

    pub fn toggle_show_archived(&mut self) {
        self.show_archived = !self.show_archived;
        self.refresh_notes();
        self.reload_image_states();
    }

    pub fn delete_selected(&mut self) {
        if let Some(note) = self.notes.get(self.selected) {
            let _ = db::delete_note(&self.conn, note.id);
            self.refresh_notes();
            self.reload_image_states();
        }
    }

    pub fn create_note_from_input(&mut self) -> Option<i64> {
        let title = self.input_buf.trim().to_string();
        if title.is_empty() { return None; }
        let id = db::create_note(&self.conn, &title).ok()?;
        self.refresh_notes();
        self.selected = 0;
        self.reload_image_states();
        Some(id)
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_msg = Some((msg.to_string(), Instant::now()));
    }

    pub fn current_status(&self) -> Option<&str> {
        if let Some((msg, t)) = &self.status_msg {
            if t.elapsed().as_secs() < 3 { return Some(msg); }
        }
        None
    }

    pub fn search_notes_by_title(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.search_results = self.notes.iter().enumerate()
            .filter(|(_, n)| n.title.to_lowercase().contains(&q))
            .map(|(i, n)| (i, n.title.clone(), None))
            .collect();
        self.search_selected = 0;
    }

    pub fn search_notes_by_content(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.search_results = self.notes.iter().enumerate()
            .filter_map(|(i, n)| {
                let snippet = n.content.lines()
                    .find(|l| l.to_lowercase().contains(&q))
                    .map(|l| l.trim().to_string());
                snippet.map(|s| (i, n.title.clone(), Some(s)))
            })
            .collect();
        self.search_selected = 0;
    }

    pub fn select_search_result(&mut self) -> bool {
        if let Some(&(idx, _, _)) = self.search_results.get(self.search_selected) {
            self.selected = idx;
            self.reload_image_states();
            return true;
        }
        false
    }

    /// Insert text at the end of the current note's content and save.
    pub fn append_to_current_note(&mut self, text: &str) {
        if let Some(note) = self.notes.get(self.selected) {
            let id = note.id;
            let mut content = note.content.clone();
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(text);
            content.push('\n');
            let title = content.lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").trim().to_string())
                .unwrap_or_else(|| note.title.clone());
            let _ = db::update_note(&self.conn, id, &title, &content);
            self.refresh_notes();
            self.reload_image_states();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let conn = db::open_memory().unwrap();
        App::new(conn, None)
    }

    #[test]
    fn test_search_by_title() {
        let mut app = test_app();
        db::create_note(&app.conn, "Rust notes").unwrap();
        db::create_note(&app.conn, "Python tips").unwrap();
        db::create_note(&app.conn, "Rusty bucket").unwrap();
        app.refresh_notes();

        app.search_notes_by_title("rust");
        assert_eq!(app.search_results.len(), 2);
        assert!(app.search_results.iter().all(|(_, t, _)| t.to_lowercase().contains("rust")));

        app.search_notes_by_title("python");
        assert_eq!(app.search_results.len(), 1);
        assert_eq!(app.search_results[0].1, "Python tips");

        app.search_notes_by_title("nonexistent");
        assert_eq!(app.search_results.len(), 0);
    }

    #[test]
    fn test_search_by_content() {
        let mut app = test_app();
        let id1 = db::create_note(&app.conn, "Note A").unwrap();
        db::update_note(&app.conn, id1, "Note A", "# Note A\nHello world from Rust").unwrap();
        let id2 = db::create_note(&app.conn, "Note B").unwrap();
        db::update_note(&app.conn, id2, "Note B", "# Note B\nPython is great").unwrap();
        let id3 = db::create_note(&app.conn, "Note C").unwrap();
        db::update_note(&app.conn, id3, "Note C", "# Note C\nRust and Python together").unwrap();
        app.refresh_notes();

        app.search_notes_by_content("rust");
        assert_eq!(app.search_results.len(), 2);
        assert!(app.search_results.iter().all(|(_, _, s)| s.is_some()));

        app.search_notes_by_content("python");
        assert_eq!(app.search_results.len(), 2);

        app.search_notes_by_content("nonexistent");
        assert_eq!(app.search_results.len(), 0);
    }

    #[test]
    fn test_select_search_result() {
        let mut app = test_app();
        db::create_note(&app.conn, "First").unwrap();
        db::create_note(&app.conn, "Second").unwrap();
        db::create_note(&app.conn, "Third").unwrap();
        app.refresh_notes();

        app.search_notes_by_title("second");
        assert_eq!(app.search_results.len(), 1);
        assert!(app.select_search_result());
        assert_eq!(app.notes[app.selected].title, "Second");
    }
}
