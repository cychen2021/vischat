use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use vischat::app::AppState;
use vischat::message::DisplayItem;

#[derive(Parser)]
#[command(
    name = "vischat",
    about = "Browse AI agent chat history in JSONL format"
)]
struct Cli {
    /// Path to the JSONL chat history file
    file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let messages = vischat::parser::parse_file(&cli.file)?;

    let all_items: Vec<DisplayItem> = messages
        .iter()
        .flat_map(|msg| DisplayItem::from_logical(msg))
        .collect();

    if all_items.is_empty() {
        eprintln!("No displayable items found in {}", cli.file);
        return Ok(());
    }

    run_tui(all_items, cli.file)
}

fn run_tui(items: Vec<DisplayItem>, file_path: String) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new(items, file_path);

    loop {
        terminal.draw(|f| vischat::ui::draw(f, &mut state))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                vischat::navigation::handle_key(&mut state, key);
            }
        }

        if state.quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
