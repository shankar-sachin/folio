# Folio

A live-edit Markdown reader for macOS. Your notes render beautifully, and any block — a paragraph, heading, list, table or code fence — turns into its Markdown the moment you click it. Press <kbd>Esc</kbd> and it's back to the rendered page. Written in Rust (Tauri).

## What's inside

- **Live mode** (default): read the rendered document and edit any block in place. Tick task-list checkboxes straight from the reader. Click below the last block to keep writing.
- **Split / Source modes**: a syntax-highlighted Markdown editor with line numbers, list continuation and synced scrolling.
- **Workspaces**: named sets of folders plus the tabs you had open. Switch with <kbd>⌃⌘R</kbd>, or from the Workspaces view.
- **Explorer**: a multi-root file tree with new file/folder, rename, drag to move, Move to Trash, Reveal in Finder, and an Open Editors list.
- **VS Code-style tabs**: preview tabs (italic) that the next single-click replaces, drag to reorder, middle-click to close, and Reopen Closed Tab.
- **Search across the workspace** (case, whole word, regex), **Outline** with the current heading tracked, and **breadcrumbs**.
- **Command palette** <kbd>⇧⌘P</kbd>, **Go to File** <kbd>⌘P</kbd>, **Go to Heading** <kbd>⇧⌘O</kbd> (or type `@` in ⌘P), and **Go to Line** <kbd>⌃G</kbd>.
- GitHub-flavored Markdown: tables that scroll instead of crushing their columns, task lists, `> [!NOTE]` callouts, YAML front matter, syntax highlighting for 30+ languages, and copy buttons on code.
- Four themes (Folio Dark, Midnight OLED, Folio Light, Paper), plus reader typefaces (SF, New York, Charter, Iowan), text size and column width.
- Picks up external changes automatically, with a conflict banner when you have unsaved edits. Unsaved work is guarded when you close or quit.
- Local only: no network, no telemetry. Remote images stay blocked unless you allow them in Settings.

## Keys

| | |
|---|---|
| ⌘1 / ⌘2 / ⌘3 | Live / Split / Source |
| ⌘P · ⇧⌘P · ⇧⌘O | Go to File · Commands · Go to Heading |
| ⌘S · ⌥⌘S | Save · Save All |
| ⌘B · ⌘I · ⌘K | Bold · Italic · Link |
| ⌘F · ⇧⌘F | Find · Search workspace |
| ⌘\\ · ⌃⌘J | Toggle sidebar · Zen mode |
| ⌥⌘O · ⌃⌘N | Add folder · New workspace |

## Build

Requirements: Xcode Command Line Tools and Rust (stable). No Node, no npm, no bundler.

```bash
bash scripts/build.sh          # -> ../deliverables/Folio.app
bash scripts/build.sh --dmg    # also writes a DMG
CARGO_OFFLINE=1 bash scripts/build.sh   # build from the cargo cache only
```

`scripts/build.sh` builds the Rust app in `src-tauri/` (the web assets in `web/` are embedded at compile time), then assembles and ad-hoc signs `Folio.app`. You don't need the Tauri CLI. Set `FOLIO_OUTPUT` to choose where the app goes.

The self-test (`Folio.app/Contents/MacOS/Folio --self-test`) writes its results to `$FOLIO_TEST_OUTPUT`. `cargo test` in `folio-core/` runs the Markdown engine tests.

Workspace state lives in `~/Library/Application Support/Folio/state.json`.

## How it works

Rust owns everything: documents, tabs, workspaces, the file tree, search, live editing, the command palette and settings. The UI is server-rendered HTML in the style of LiveView: Rust renders each region of the window, diffs it, and sends only the changed regions to the webview. A small generic script (`web/folio.js`) forwards clicks and keys to Rust and applies the patches; it has no app logic of its own.

## Layout

```
folio-core/  Markdown engine (no dependencies): GFM parser, safe HTML renderer, syntax highlighter, icons
src-tauri/   The app: state, events, views, file system, search, watcher, dialogs, menu, mdasset:// images
web/         index.html (window skeleton), style.css, folio.js (event/patch shim)
scripts/     build.sh, Info.plist
```
