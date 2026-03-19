# vischat

A terminal UI for browsing AI agent chat history stored in JSONL format (as produced by Claude Code sessions).

## Features

- Color-coded role badges: `[SYS]`, `[ASST]`, `[THINK]`, `[TOOL>]`, `[TOOL<]`
- Three-pane layout: message list, detail view, status bar; or two-pane inline expansion
- Vim-style navigation
- Thinking blocks folded (shown as `...`) by default; press `t` to expand

## Usage

```bash
cargo run -- <path-to-history.jsonl>
```

## Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Jump to first item |
| `G` | Jump to last item |
| `Enter` / `Space` | Expand/collapse selected item inline |
| `Ctrl-d` | Scroll detail pane (or expanded block) down |
| `Ctrl-u` | Scroll detail pane (or expanded block) up |
| `t` | Toggle thinking blocks (folded `...` ↔ expanded) |
| `q` / `Esc` | Quit |

## JSONL Format

Each line is a streaming event from a Claude Code session:

- `type: "system"` — session init (model, tools, cwd)
- `type: "assistant"` — one content block per line, grouped by `message.id`
  - `thinking` — internal reasoning
  - `text` — visible response
  - `tool_use` — tool invocation with name and JSON input
- `type: "user"` — tool results

## Building

```bash
cargo build --release
```
