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
        .flat_map(DisplayItem::from_logical)
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

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            vischat::navigation::handle_key(&mut state, key);
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parses_file_arg() {
        let cli = Cli::try_parse_from(["vischat", "some/file.jsonl"]).unwrap();
        assert_eq!(cli.file, "some/file.jsonl");
    }

    #[test]
    fn test_cli_missing_file_arg_errors() {
        assert!(Cli::try_parse_from(["vischat"]).is_err());
    }

    #[test]
    fn test_cli_help_flag_errors_with_help_exit() {
        // --help causes clap to surface Err with kind DisplayHelp
        match Cli::try_parse_from(["vischat", "--help"]) {
            Ok(_) => panic!("expected error"),
            Err(e) => assert_eq!(e.kind(), clap::error::ErrorKind::DisplayHelp),
        }
    }
}
