use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppState;

pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q'))
        | (KeyModifiers::NONE, KeyCode::Esc) => {
            state.quit = true;
        }

        // Movement
        (KeyModifiers::NONE, KeyCode::Char('j'))
        | (KeyModifiers::NONE, KeyCode::Down) => {
            state.move_down();
        }
        (KeyModifiers::NONE, KeyCode::Char('k'))
        | (KeyModifiers::NONE, KeyCode::Up) => {
            state.move_up();
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            state.selected = 0;
            state.list_scroll = 0;
            state.detail_scroll = 0;
        }
        (KeyModifiers::NONE, KeyCode::Char('G')) => {
            let last = state.visible_count().saturating_sub(1);
            state.selected = last;
            state.detail_scroll = 0;
        }

        // Detail pane scroll
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            state.detail_scroll = state.detail_scroll.saturating_add(10);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            state.detail_scroll = state.detail_scroll.saturating_sub(10);
        }

        // Toggle thinking
        (KeyModifiers::NONE, KeyCode::Char('t')) => {
            state.show_thinking = !state.show_thinking;
            // Clamp selection if items changed
            let count = state.visible_count();
            if count > 0 && state.selected >= count {
                state.selected = count - 1;
            }
            state.detail_scroll = 0;
        }

        _ => {}
    }
}
