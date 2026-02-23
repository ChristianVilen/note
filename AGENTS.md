# AGENTS.md

## Product Vision

**note** is a fast, keyboard-driven terminal note-taking app for developers who live in the terminal. It should feel like a natural extension of the shell — instant startup, zero config, no cloud, no accounts. Your notes live locally in SQLite and are always a keystroke away.

The north star: **capture and find thoughts faster than switching to any GUI app.**

### Design Principles

- **Speed over features.** Startup must be instant. Every interaction should feel zero-latency.
- **Keyboard-first.** Mouse is supported but optional. A power user should never need it.
- **Local and private.** All data stays on disk at `~/.note/notes.db`. No network calls, ever.
- **Minimal UI, maximum utility.** Two-pane layout (sidebar + editor) is the entire interface. No menus, no tabs, no settings screens.
- **Markdown-native.** Notes are plain markdown. The first `# ` heading becomes the note title automatically.

## Architecture

```
src/
├── main.rs      # Event loop, key routing, terminal setup, external editor launch
├── app.rs       # App state, focus management, editor lifecycle, auto-save logic
├── ui.rs        # All rendering — sidebar, editor pane, status bar, popups
├── db.rs        # SQLite schema, CRUD operations, all DB access
└── highlight.rs # (orphaned, unused — was syntect-based markdown highlighting, replaced by ratatui-textarea)
```

### Key Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` 0.30 | TUI framework |
| `crossterm` 0.29 | Terminal backend (raw mode, events, mouse) |
| `ratatui-textarea` 0.8 | Inline text editor widget (undo/redo, selection, yank, scroll) |
| `rusqlite` (bundled) | SQLite database |
| `chrono` | Timestamps |
| `dirs` | Home directory resolution |
| `anyhow` | Error handling in main |

### Data Model

Single `notes` table in `~/.note/notes.db`:

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | Auto-increment |
| title | TEXT | Derived from first `# ` heading on save |
| content | TEXT | Raw markdown |
| archived | INTEGER | 0 or 1 |
| created_at | TEXT | ISO-ish local time |
| updated_at | TEXT | Updated on every save |

### App State (`App` struct)

- `focus: Focus` — `Sidebar` or `Editor`. Determines where key events route.
- `editor: Option<TextArea<'static>>` — The inline editor widget. `None` when no notes exist.
- `editing_note_id: Option<i64>` — Which note is loaded in the editor.
- `dirty: bool` + `last_edit: Option<Instant>` — Debounced auto-save. Saves after 1 second of inactivity.
- `input_mode: InputMode` — `Normal`, `TitleInput` (new note popup), or `ConfirmDelete`.

### Event Loop

The event loop polls with a 200ms timeout (`event::poll`), which serves double duty:
1. Process key/mouse events when available.
2. On timeout, check if a debounced auto-save is due.

Key routing in `Normal` mode:
- **Sidebar focus:** `j/k` navigate, `n` new note, `a` archive, `d` delete, `E` external editor, `q` quit, `Tab` → editor.
- **Editor focus:** All keys go to `TextArea::input()`. `Tab` or `Esc` → sidebar.

## Conventions

- **Auto-save, not manual save.** There is no save command. Content saves automatically after 1 second of idle, on note switch, and on quit.
- **Title extraction.** On save, the title is derived from the first `# ` line in the content. If none exists, the original title is preserved.
- **Editor reload.** Any operation that changes which note is selected (navigate, create, delete, archive) calls `load_note_into_editor()` to sync the `TextArea`.
- **External editor.** `E` (shift) suspends the TUI, launches `$EDITOR` (default: vim) on a temp file, then reloads on return.

## Testing

- DB module has unit tests using an in-memory SQLite database (`db::open_memory()`).
- Run with `cargo test`.
- No UI tests currently — the TUI is tested manually.

## Things to Know

- `highlight.rs` is orphaned. It was the old syntect-based renderer before inline editing was added. Safe to delete.
- The `TextArea` widget from `ratatui-textarea` handles its own scrolling, cursor movement, undo/redo, and selection. Don't reimplement these.
- `draw()` takes `&mut App` (not `&App`) because it needs to call `ta.set_block()` and `ta.set_cursor_style()` before rendering. The title is extracted before the mutable borrow to satisfy the borrow checker.
- Notes are ordered by `updated_at DESC` — most recently edited note is always at the top.

## Future Ideas (not yet implemented)

- Full-text search / fuzzy find across notes
- Tags or categories
- Markdown preview pane (rendered markdown alongside raw editor)
- Export to file / import from file
- Configurable keybindings
- Syntax highlighting in the editor (the `TextArea` widget supports custom styling but it's not wired up yet)
- Note linking (wiki-style `[[note title]]` references)
