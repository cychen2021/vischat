use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppState;

pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            state.quit = true;
        }

        // Movement
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            state.move_down();
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            state.move_up();
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            state.selected = 0;
            state.list_scroll = 0;
            state.detail_scroll = 0;
        }
        (KeyModifiers::NONE, KeyCode::Char('G')) => {
            let last = state.navigable_count().saturating_sub(1);
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

        // Expand/collapse inline
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char(' ')) => {
            state.toggle_expand();
        }

        // Toggle thinking
        (KeyModifiers::NONE, KeyCode::Char('t')) => {
            // Remember which item is selected before the navigable set changes
            let current_ptr = state.selected_item().map(|item| item as *const _);
            state.show_thinking = !state.show_thinking;
            if let Some(ptr) = current_ptr {
                let new_idx = state
                    .navigable_items()
                    .iter()
                    .position(|item| std::ptr::eq(*item, ptr));
                if let Some(idx) = new_idx {
                    state.selected = idx;
                } else {
                    // Selected item is no longer navigable (was a thinking block, now hidden)
                    let count = state.navigable_count();
                    if count > 0 && state.selected >= count {
                        state.selected = count - 1;
                    }
                }
            }
            state.detail_scroll = 0;
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::message::{DisplayItem, Role};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_item() -> DisplayItem {
        DisplayItem {
            role: Role::Assistant,
            badge: "[T]",
            summary: "test".to_string(),
            detail: "detail".to_string(),
        }
    }

    fn make_state(n: usize) -> AppState {
        let items = (0..n).map(|_| make_item()).collect();
        AppState::new(items, "test.jsonl".to_string())
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn test_quit_on_q() {
        let mut state = make_state(2);
        handle_key(&mut state, key(KeyCode::Char('q')));
        assert!(state.quit);
    }

    #[test]
    fn test_quit_on_esc() {
        let mut state = make_state(2);
        handle_key(&mut state, key(KeyCode::Esc));
        assert!(state.quit);
    }

    #[test]
    fn test_move_down_j() {
        let mut state = make_state(3);
        handle_key(&mut state, key(KeyCode::Char('j')));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_down_arrow() {
        let mut state = make_state(3);
        handle_key(&mut state, key(KeyCode::Down));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_up_k() {
        let mut state = make_state(3);
        state.selected = 2;
        handle_key(&mut state, key(KeyCode::Char('k')));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_up_arrow() {
        let mut state = make_state(3);
        state.selected = 2;
        handle_key(&mut state, key(KeyCode::Up));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_jump_to_first_g() {
        let mut state = make_state(5);
        state.selected = 4;
        state.list_scroll = 3;
        state.detail_scroll = 5;
        handle_key(&mut state, key(KeyCode::Char('g')));
        assert_eq!(state.selected, 0);
        assert_eq!(state.list_scroll, 0);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_jump_to_last_g_uppercase() {
        let mut state = make_state(5);
        handle_key(&mut state, key(KeyCode::Char('G')));
        assert_eq!(state.selected, 4);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_ctrl_d_scrolls_detail_down() {
        let mut state = make_state(2);
        handle_key(&mut state, ctrl_key(KeyCode::Char('d')));
        assert_eq!(state.detail_scroll, 10);
    }

    #[test]
    fn test_ctrl_d_accumulates() {
        let mut state = make_state(2);
        handle_key(&mut state, ctrl_key(KeyCode::Char('d')));
        handle_key(&mut state, ctrl_key(KeyCode::Char('d')));
        assert_eq!(state.detail_scroll, 20);
    }

    #[test]
    fn test_ctrl_u_scrolls_detail_up() {
        let mut state = make_state(2);
        state.detail_scroll = 20;
        handle_key(&mut state, ctrl_key(KeyCode::Char('u')));
        assert_eq!(state.detail_scroll, 10);
    }

    #[test]
    fn test_ctrl_u_saturates_at_zero() {
        let mut state = make_state(2);
        state.detail_scroll = 5;
        handle_key(&mut state, ctrl_key(KeyCode::Char('u')));
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_toggle_thinking_t() {
        let mut state = make_state(2);
        assert!(!state.show_thinking);
        handle_key(&mut state, key(KeyCode::Char('t')));
        assert!(state.show_thinking);
        handle_key(&mut state, key(KeyCode::Char('t')));
        assert!(!state.show_thinking);
    }

    #[test]
    fn test_toggle_thinking_clamps_selection() {
        // Add a thinking item so visible count changes when toggled
        let mut state = make_state(0);
        state.all_items = vec![
            make_item(), // Assistant
            DisplayItem {
                role: Role::Thinking,
                badge: "[THINK]",
                summary: "t".to_string(),
                detail: "d".to_string(),
            },
        ];
        state.show_thinking = true;
        state.selected = 1; // pointing at thinking item
        handle_key(&mut state, key(KeyCode::Char('t'))); // hide thinking → count becomes 1
        assert_eq!(state.selected, 0); // clamped to last visible
    }

    #[test]
    fn test_toggle_thinking_preserves_selected_item() {
        // items: Assistant(0), Thinking(1), Assistant(2), Thinking(3), Assistant(4)
        // show_thinking=false → navigable = [0,2,4], so selected=2 points to Assistant(4)
        // After toggling on, selected should still point to Assistant(4), i.e. navigable index 4
        let mut state = make_state(0);
        state.all_items = vec![
            make_item(), // 0 Assistant
            DisplayItem { role: Role::Thinking, badge: "[T]", summary: "t".to_string(), detail: "d".to_string() },
            make_item(), // 2 Assistant
            DisplayItem { role: Role::Thinking, badge: "[T]", summary: "t".to_string(), detail: "d".to_string() },
            make_item(), // 4 Assistant
        ];
        state.show_thinking = false;
        state.selected = 2; // third non-thinking = all_items[4]
        handle_key(&mut state, key(KeyCode::Char('t'))); // show thinking
        assert_eq!(state.selected, 4); // all_items[4] is now navigable index 4
        assert_eq!(
            state.selected_item().unwrap().role,
            Role::Assistant,
            "should remain on the same Assistant item"
        );
    }

    #[test]
    fn test_enter_toggles_expanded() {
        let mut state = make_state(2);
        assert!(!state.expanded);
        handle_key(&mut state, key(KeyCode::Enter));
        assert!(state.expanded);
        handle_key(&mut state, key(KeyCode::Enter));
        assert!(!state.expanded);
    }

    #[test]
    fn test_space_toggles_expanded() {
        let mut state = make_state(2);
        assert!(!state.expanded);
        handle_key(&mut state, key(KeyCode::Char(' ')));
        assert!(state.expanded);
        handle_key(&mut state, key(KeyCode::Char(' ')));
        assert!(!state.expanded);
    }

    #[test]
    fn test_enter_resets_detail_scroll() {
        let mut state = make_state(2);
        state.detail_scroll = 10;
        handle_key(&mut state, key(KeyCode::Enter));
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_unknown_key_does_nothing() {
        let mut state = make_state(2);
        handle_key(&mut state, key(KeyCode::Char('z')));
        assert!(!state.quit);
        assert_eq!(state.selected, 0);
    }
}
