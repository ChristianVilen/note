# AGENTS.md

## Product Vision

**note** is a fast, keyboard-driven terminal note-taking app for developers who live in the terminal. It should feel like a natural extension of the shell — instant startup, zero config, no cloud, no accounts. Your notes live locally in SQLite and are always a keystroke away.

The north star: **capture and find thoughts faster than switching to any GUI app.**

### Design Principles

- **Speed over features.** Startup must be instant. Every interaction should feel zero-latency.
- **Keyboard-first.** Mouse is supported but optional. A power user should never need it.
- **Local and private.** All data stays on disk at `~/.note/notes.db`. No network calls, ever.
- **Minimal UI, maximum utility.** Two-pane layout (sidebar + markdown preview) is the entire interface. No menus, no tabs, no settings screens.
- **Markdown-native.** Notes are plain markdown. The first `# ` heading becomes the note title automatically.
- **Use your own editor.** Editing is done in `$EDITOR` (defaults to nvim). The TUI is for browsing and previewing.

## Architecture

```
src/
├── main.rs      # Event loop, key routing, terminal setup, external editor launch
├── app.rs       # App state, note management, image state management
├── ui.rs        # All rendering — sidebar, markdown preview pane, status bar, popups
├── db.rs        # SQLite schema, CRUD operations, all DB access
├── images.rs    # Clipboard image paste, image line parsing, attachment storage
└── highlight.rs # (orphaned, unused — safe to delete)
```

### Key Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` 0.30 | TUI framework |
| `crossterm` 0.29 | Terminal backend (raw mode, events, mouse, bracketed paste) |
| `ratatui-image` 10.0 | Inline image rendering via Kitty/Sixel/iTerm2 graphics protocols |
| `arboard` 3.4 | Cross-platform clipboard access (image paste) |
| `image` 0.25 | Image decoding (PNG, JPEG, etc.) |
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

Image attachments are stored as files in `~/.note/attachments/` and referenced via markdown image links `![screenshot](/path/to/file.png)` in note content.

### App State (`App` struct)

- `input_mode: InputMode` — `Normal`, `TitleInput` (new note popup), or `ConfirmDelete`.
- `picker: Option<Picker>` — `ratatui-image` picker for terminal graphics protocol detection. `None` if terminal doesn't support image rendering.
- `image_states: HashMap<PathBuf, StatefulProtocol>` — Loaded image render states for the current note, keyed by file path.
- `status_msg: Option<(String, Instant)>` — Transient status message shown in the status bar for 3 seconds.

### Event Loop

The event loop polls with a 200ms timeout (`event::poll`).

Key routing in `Normal` mode:
- `j/k/↑/↓` navigate notes
- `e` or `Enter` opens the selected note in `$EDITOR` (default: nvim)
- `n` creates a new note (title popup → then opens in editor)
- `a` archive/unarchive, `A` toggle show archived
- `d` delete (with confirmation)
- `Ctrl+S` paste screenshot from clipboard
- `?` help, `q` quit

Bracketed paste is enabled — dragging an image file into the terminal is detected and handled (copies to attachments, inserts markdown link).

### Image Support

- **Clipboard paste (`Ctrl+S`):** Reads image data from clipboard via `arboard`, saves as PNG to `~/.note/attachments/` with timestamped filename, appends `![screenshot](path)` to the note.
- **Drag and drop:** Bracketed paste detects image file paths (including `file://` URLs with percent-encoding), copies the file to attachments, appends the link.
- **Inline rendering:** The preview pane renders images inline using `ratatui-image` with the Kitty graphics protocol. Works in Ghostty, Kitty, WezTerm, iTerm2, and other supported terminals. Falls back to plain text if the terminal doesn't support graphics.
- **Image state management:** `reload_image_states()` is called when switching notes — decodes new images, removes stale entries from the HashMap.

## Conventions

- **Edit in $EDITOR, not inline.** There is no built-in text editor. Pressing `e` or `Enter` launches nvim (or `$EDITOR`) on a temp file, then saves back to the DB on return.
- **Title extraction.** On save, the title is derived from the first `# ` line in the content. If none exists, the original title is preserved.
- **Markdown preview.** The right pane renders styled markdown: headings in cyan/bold, bullets with `•`, blockquotes in italic, image links in blue, code fences in green.
- **Notes are ordered by `updated_at DESC`** — most recently edited note is always at the top.

## Testing

- DB module has unit tests using an in-memory SQLite database (`db::open_memory()`).
- Image module has unit tests for path parsing and image line detection.
- Run with `cargo test`.
- No UI tests currently — the TUI is tested manually.

## Building & Installing

- `cargo build` compiles to `target/debug/note`.
- `./release.sh` builds a release binary and installs it to `~/.cargo/bin/note`.
- Version is displayed in the status bar (bottom right), read from `Cargo.toml` at compile time via `env!("CARGO_PKG_VERSION")`.

## Things to Know

- `highlight.rs` is orphaned. Safe to delete.
- `draw()` takes `&mut App` (not `&App`) because `ratatui-image`'s `StatefulImage` requires mutable access to `StatefulProtocol` during rendering.
- `Picker::from_query_stdio()` must be called after entering alternate screen but before reading events. It's wrapped in `Option` so the app degrades gracefully if it fails.
- Ghostty (and most terminals) intercept `Cmd+key` combos — they never reach the app. That's why screenshot paste uses `Ctrl+S` instead of `Cmd+V`.
- Bracketed paste mode is enabled so drag-and-drop file paths arrive as a single `Event::Paste(String)` rather than individual key events.

## Future Ideas (not yet implemented)

- Full-text search / fuzzy find across notes
- Tags or categories
- Export to file / import from file
- Configurable keybindings
- Note linking (wiki-style `[[note title]]` references)
- Scrolling in the preview pane for long notes
- Better markdown rendering (inline bold/italic, code spans, numbered lists)
