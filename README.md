# note

A fast, keyboard-driven terminal note-taking app. Just local markdown notes in SQLite.

## Install

```bash
cargo install --path .
```

Or use the release script:

```bash
./release.sh
```

## Usage

```bash
note
```

Notes are stored in `~/.note/notes.db`. Editing opens `$EDITOR` (defaults to nvim).

### Keybindings

| Key                   | Action                          |
| --------------------- | ------------------------------- |
| `j` / `k` / `↑` / `↓` | Navigate notes                  |
| `e` / `Enter`         | Edit note in $EDITOR            |
| `n`                   | Create new note                 |
| `ff`                  | Find note by title              |
| `fw`                  | Grep note content               |
| `a`                   | Archive / unarchive             |
| `A`                   | Toggle show archived            |
| `d`                   | Delete (with confirmation)      |
| `Ctrl+S`              | Paste screenshot from clipboard |
| `y`                   | Yank (copy) selection           |
| `Esc`                 | Clear search highlight          |
| `?`                   | Help                            |
| `q`                   | Quit                            |

Drag and drop image files into the terminal to attach them.

### Screenshots

Images pasted via `Ctrl+S` or drag-and-drop are saved to `~/.note/attachments/` and rendered inline in the preview pane using the Kitty graphics protocol.

Supported terminals: Ghostty, Kitty, WezTerm, iTerm2. Falls back to text in unsupported terminals.

## Roadmap

- [x] Full-text search / fuzzy find
- [ ] Export / import notes
- [x] Preview pane scrolling for long notes
- [ ] Better markdown rendering (bold/italic, code spans, numbered lists)
- [ ] Orphaned attachment cleanup

## License

MIT
