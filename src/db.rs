use rusqlite::{Connection, Result, params};
use std::fs;
use std::path::PathBuf;

pub struct Note {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub archived: bool,
#[allow(dead_code)]
    pub created_at: String,
    pub updated_at: String,
}

fn db_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    let dir = home.join(".note");
    fs::create_dir_all(&dir).expect("Could not create ~/.note directory");
    dir.join("notes.db")
}

pub fn open_db() -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    init_db(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    init_db(&conn)?;
    Ok(conn)
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn create_note(conn: &Connection, title: &str) -> Result<i64> {
    let ts = now();
    conn.execute(
        "INSERT INTO notes (title, content, archived, created_at, updated_at) VALUES (?1, ?2, 0, ?3, ?4)",
        params![title, "", &ts, &ts],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_note(conn: &Connection, id: i64, title: &str, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE notes SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
        params![title, content, now(), id],
    )?;
    Ok(())
}

pub fn get_note(conn: &Connection, id: i64) -> Result<Note> {
    conn.query_row(
        "SELECT id, title, content, archived, created_at, updated_at FROM notes WHERE id = ?1",
        params![id],
        |row| Ok(Note {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            archived: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }),
    )
}

pub fn list_notes(conn: &Connection, show_archived: bool) -> Result<Vec<Note>> {
    let sql = if show_archived {
        "SELECT id, title, content, archived, created_at, updated_at FROM notes ORDER BY updated_at DESC"
    } else {
        "SELECT id, title, content, archived, created_at, updated_at FROM notes WHERE archived = 0 ORDER BY updated_at DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let notes = stmt.query_map([], |row| {
        Ok(Note {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            archived: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(notes)
}

pub fn archive_note(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE notes SET archived = 1, updated_at = ?1 WHERE id = ?2", params![now(), id])?;
    Ok(())
}

pub fn unarchive_note(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE notes SET archived = 0, updated_at = ?1 WHERE id = ?2", params![now(), id])?;
    Ok(())
}

pub fn delete_note(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crud() {
        let conn = open_memory().unwrap();
        let id = create_note(&conn, "Test").unwrap();
        let note = get_note(&conn, id).unwrap();
        assert_eq!(note.title, "Test");
        assert!(!note.archived);

        update_note(&conn, id, "Updated", "# Hello").unwrap();
        let note = get_note(&conn, id).unwrap();
        assert_eq!(note.title, "Updated");
        assert_eq!(note.content, "# Hello");
    }

    #[test]
    fn test_archive() {
        let conn = open_memory().unwrap();
        let id = create_note(&conn, "Note").unwrap();

        let notes = list_notes(&conn, false).unwrap();
        assert_eq!(notes.len(), 1);

        archive_note(&conn, id).unwrap();
        let notes = list_notes(&conn, false).unwrap();
        assert_eq!(notes.len(), 0);

        let notes = list_notes(&conn, true).unwrap();
        assert_eq!(notes.len(), 1);
        assert!(notes[0].archived);

        unarchive_note(&conn, id).unwrap();
        let notes = list_notes(&conn, false).unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_delete() {
        let conn = open_memory().unwrap();
        let id = create_note(&conn, "Gone").unwrap();
        delete_note(&conn, id).unwrap();
        let notes = list_notes(&conn, true).unwrap();
        assert_eq!(notes.len(), 0);
    }
}
