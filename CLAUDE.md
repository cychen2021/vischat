# vischat

A terminal user interface (TUI) application for browsing AI agent chat history stored in JSONL format.

## Project Overview

vischat is a Rust-based TUI application that allows users to browse and view AI agent conversation history. It reads JSONL files where each line contains an OpenAI-compatible message object and displays them in an interactive, readable format with Markdown rendering support.

## Architecture

### Core Components

1. **JSONL Parser** - Reads and deserializes chat history from JSONL files
2. **Message Model** - Data structures representing different message types (system, assistant, user)
3. **TUI Application** - Main event loop and UI state management using ratatui
4. **Message Renderer** - Renders messages with Markdown support using md-tui
5. **Navigation Controller** - Handles keyboard input and message navigation

### Data Structures

Based on [example-history.jsonl](example-history.jsonl), the message format includes:

- **System Messages**: `type: "system"`, includes initialization data, session info, available tools
- **Assistant Messages**: `type: "assistant"`, contains model responses with content blocks:
  - `thinking` blocks with reasoning (optional)
  - `text` blocks with response text
  - `tool_use` blocks with tool invocations
- **User Messages**: `type: "user"`, contains tool results or user input

Each message has:
- `type`: Message type (system/assistant/user)
- `message`: The actual message content with OpenAI-compatible structure
- `session_id`: Session identifier
- `uuid`: Unique message identifier
- `timestamp`: Optional timestamp for tracking

### Key Features

1. **Message Display**
   - List view showing all messages in chronological order
   - Expandable/collapsible messages
   - Syntax highlighting for code blocks
   - Markdown rendering for formatted text

2. **Navigation**
   - Scroll through messages (j/k, arrow keys)
   - Jump to message (g/G for first/last)
   - Search/filter messages
   - Session boundaries visualization

3. **Content Rendering**
   - Render thinking blocks with distinct styling
   - Display tool uses with formatted JSON
   - Show tool results
   - Handle long messages with pagination

4. **File Handling**
   - Load JSONL files from command line argument
   - Handle large files efficiently (streaming/lazy loading)
   - Watch for file updates (optional live mode)

## Dependencies

### Core Dependencies
- **ratatui** (0.30.0) - Terminal UI framework
- **md-tui** (0.9.5) - Markdown rendering for terminal
- **serde** + **serde_json** - JSON serialization (add to Cargo.toml)
- **crossterm** - Terminal manipulation (ratatui backend, add to Cargo.toml)

### Dev Dependencies
- **insta** (1.46.3) - Snapshot testing for UI states and message parsing

### Additional Dependencies to Add
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
crossterm = "0.28"
anyhow = "1"  # Error handling
clap = { version = "4", features = ["derive"] }  # CLI argument parsing
```

## Testing Strategy

### Unit Tests
- Message parsing and deserialization
- Navigation state management
- Message filtering and search
- Each module should have tests in the same file

### Integration Tests
- Full JSONL file loading and parsing
- UI state transitions
- End-to-end navigation flows
- Place in `tests/` directory

### Snapshot Tests (cargo-insta)
- Message rendering outputs
- UI layout snapshots for different terminal sizes
- Markdown formatting results
- Use `cargo insta test` and `cargo insta review`

### Test Data
- Use [example-history.jsonl](example-history.jsonl) as primary test fixture
- Create smaller focused fixtures for unit tests
- Test edge cases: empty files, malformed JSON, missing fields

## Development Workflow

### Running the Application
```bash
cargo run -- example-history.jsonl
```

### Running Tests
```bash
# All tests
cargo test

# Snapshot tests
cargo insta test

# Review snapshot changes
cargo insta review
```

### Project Structure
```
vischat/
├── src/
│   ├── main.rs           # Entry point and CLI
│   ├── app.rs            # Main application state and event loop
│   ├── message.rs        # Message data structures
│   ├── parser.rs         # JSONL parsing logic
│   ├── ui.rs             # UI rendering logic
│   └── navigation.rs     # Navigation and input handling
├── tests/
│   ├── integration.rs    # Integration tests
│   └── fixtures/         # Test JSONL files
├── example-history.jsonl
├── Cargo.toml
└── CLAUDE.md
```

## Implementation Checklist

### Phase 1: Core Data Structures
- [ ] Define message types and enums
- [ ] Implement serde deserialization for OpenAI message format
- [ ] Create JSONL parser with error handling
- [ ] Write unit tests for parsing

### Phase 2: Basic TUI
- [ ] Set up ratatui application structure
- [ ] Implement basic message list view
- [ ] Add simple text rendering (no markdown yet)
- [ ] Handle terminal events and keyboard input
- [ ] Implement basic navigation (up/down scrolling)

### Phase 3: Markdown Rendering
- [ ] Integrate md-tui for message content
- [ ] Style different content types (thinking, text, tool_use)
- [ ] Handle code blocks with syntax highlighting
- [ ] Add color scheme for message types

### Phase 4: Advanced Navigation
- [ ] Jump to first/last message
- [ ] Search functionality
- [ ] Filter by message type
- [ ] Session boundary markers
- [ ] Message detail view (expandable)

### Phase 5: Testing
- [ ] Add unit tests for all modules
- [ ] Create integration tests
- [ ] Implement snapshot tests with insta
- [ ] Test with large history files

### Phase 6: Polish
- [ ] Error handling and user feedback
- [ ] Help screen with keybindings
- [ ] Performance optimization for large files
- [ ] Configuration options (colors, keybindings)
- [ ] Documentation and README

## Code Style and Conventions

- Use descriptive variable names
- Keep functions focused and small
- Add doc comments for public APIs
- Use `Result<T>` with `anyhow` for error handling
- Prefer composition over inheritance
- Test edge cases and error paths

## Keyboard Controls (Vim-Style)

### Navigation

- `j` or `↓` - Move down one message
- `k` or `↑` - Move up one message
- `gg` - Jump to first message
- `G` - Jump to last message
- `Ctrl-d` - Scroll down half page
- `Ctrl-u` - Scroll up half page
- `Ctrl-f` - Scroll down full page
- `Ctrl-b` - Scroll up full page
- `H/M/L` - Jump to top/middle/bottom of screen

### Message Actions

- `Enter` or `Space` - Expand/collapse message
- `o` - Expand current message
- `c` - Collapse current message
- `O` - Expand all messages
- `C` - Collapse all messages

### Search and Filter

- `/` - Search forward
- `?` - Search backward
- `n` - Next search result
- `N` - Previous search result
- `f` - Filter by message type
- `F` - Clear filters

### View Modes

- `t` - Toggle thinking blocks visibility
- `s` - Show/hide system messages
- `d` - Toggle detailed view
- `:` - Command mode (for advanced features)

### General

- `q` or `ZZ` - Quit
- `?` or `h` - Help screen
- `r` - Refresh/reload file
- `Esc` - Cancel/clear current action

## Notes

- Messages can be very large (thinking blocks, tool results with lots of data)
- Consider lazy loading or pagination for performance
- The `signature` field in thinking blocks appears to be a cryptographic signature - display truncated
- Tool results may contain nested structures - flatten or indent appropriately
- Session IDs can be used to group related messages
