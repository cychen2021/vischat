use crate::message::{DisplayItem, Role};

pub struct AppState {
    pub all_items: Vec<DisplayItem>,
    pub selected: usize,
    pub list_scroll: usize,
    pub detail_scroll: usize,
    pub show_thinking: bool,
    pub quit: bool,
    pub file_path: String,
}

impl AppState {
    pub fn new(all_items: Vec<DisplayItem>, file_path: String) -> Self {
        AppState {
            all_items,
            selected: 0,
            list_scroll: 0,
            detail_scroll: 0,
            show_thinking: false,
            quit: false,
            file_path,
        }
    }

    /// Returns items j/k navigation can land on (excludes thinking when hidden).
    pub fn navigable_items(&self) -> Vec<&DisplayItem> {
        self.all_items
            .iter()
            .filter(|item| self.show_thinking || item.role != Role::Thinking)
            .collect()
    }

    /// Returns all items for display (thinking shown folded when hidden).
    pub fn list_items(&self) -> Vec<&DisplayItem> {
        self.all_items.iter().collect()
    }

    pub fn navigable_count(&self) -> usize {
        self.navigable_items().len()
    }

    pub fn selected_item(&self) -> Option<&DisplayItem> {
        self.navigable_items().into_iter().nth(self.selected)
    }

    /// Position of the selected navigable item within `all_items`.
    pub fn selected_list_index(&self) -> Option<usize> {
        let sel = self.selected_item()?;
        self.all_items
            .iter()
            .position(|item| std::ptr::eq(item, sel))
    }

    pub fn move_down(&mut self) {
        let count = self.navigable_count();
        if count == 0 {
            return;
        }
        if self.selected + 1 < count {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    /// Ensure list_scroll keeps selected item visible given the list pane height.
    pub fn clamp_scroll(&mut self, list_height: usize) {
        if list_height == 0 {
            return;
        }
        let list_pos = self.selected_list_index().unwrap_or(0);
        if list_pos < self.list_scroll {
            self.list_scroll = list_pos;
        }
        if list_pos >= self.list_scroll + list_height {
            self.list_scroll = list_pos - list_height + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{DisplayItem, Role};

    fn make_item(role: Role) -> DisplayItem {
        DisplayItem {
            role,
            badge: "[T]",
            summary: "test".to_string(),
            detail: "detail".to_string(),
        }
    }

    fn make_state(n: usize) -> AppState {
        let items = (0..n).map(|_| make_item(Role::Assistant)).collect();
        AppState::new(items, "test.jsonl".to_string())
    }

    #[test]
    fn test_new_defaults() {
        let state = AppState::new(vec![], "file.jsonl".to_string());
        assert_eq!(state.selected, 0);
        assert_eq!(state.list_scroll, 0);
        assert_eq!(state.detail_scroll, 0);
        assert!(!state.show_thinking);
        assert!(!state.quit);
    }

    #[test]
    fn test_navigable_items_hides_thinking_by_default() {
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::ToolUse),
        ];
        let state = AppState::new(items, "f".to_string());
        let nav = state.navigable_items();
        assert_eq!(nav.len(), 2);
        assert!(nav.iter().all(|i| i.role != Role::Thinking));
    }

    #[test]
    fn test_navigable_items_shows_thinking_when_enabled() {
        let items = vec![make_item(Role::Assistant), make_item(Role::Thinking)];
        let mut state = AppState::new(items, "f".to_string());
        state.show_thinking = true;
        assert_eq!(state.navigable_items().len(), 2);
    }

    #[test]
    fn test_navigable_count_excludes_thinking() {
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::Assistant),
        ];
        let state = AppState::new(items, "f".to_string());
        assert_eq!(state.navigable_count(), 2);
    }

    #[test]
    fn test_list_items_always_includes_thinking() {
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::ToolUse),
        ];
        let state = AppState::new(items, "f".to_string());
        assert_eq!(state.list_items().len(), 3);
        let mut state2 = AppState::new(
            vec![make_item(Role::Assistant), make_item(Role::Thinking)],
            "f".to_string(),
        );
        state2.show_thinking = true;
        assert_eq!(state2.list_items().len(), 2);
    }

    #[test]
    fn test_selected_list_index_skips_folded_thinking() {
        // items: Assistant, Thinking, Assistant
        // show_thinking=false → navigable: [0, 2]; selected=1 → all_items index 2
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::Assistant),
        ];
        let mut state = AppState::new(items, "f".to_string());
        state.selected = 1; // second navigable = third all_items
        assert_eq!(state.selected_list_index(), Some(2));
    }

    #[test]
    fn test_selected_list_index_none_on_empty() {
        let state = make_state(0);
        assert!(state.selected_list_index().is_none());
    }

    #[test]
    fn test_clamp_scroll_with_folded_thinking_ahead() {
        // items: Assistant, Thinking, Assistant, Assistant, Assistant
        // show_thinking=false → navigable indices in all_items: [0,2,3,4]
        // selected navigable=3 → all_items index 4
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::Assistant),
            make_item(Role::Assistant),
            make_item(Role::Assistant),
        ];
        let mut state = AppState::new(items, "f".to_string());
        state.selected = 3;
        state.clamp_scroll(3); // list_pos=4, need scroll so 4 < scroll+3 → scroll=2
        assert_eq!(state.list_scroll, 2);
    }

    #[test]
    fn test_move_down_advances_selection() {
        let mut state = make_state(3);
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_down_stops_at_end() {
        let mut state = make_state(2);
        state.move_down();
        state.move_down(); // at last item, no-op
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_down_resets_detail_scroll() {
        let mut state = make_state(2);
        state.detail_scroll = 5;
        state.move_down();
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_move_up_retreats_selection() {
        let mut state = make_state(3);
        state.selected = 2;
        state.move_up();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_move_up_stops_at_start() {
        let mut state = make_state(3);
        state.move_up(); // already at 0, no-op
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_move_up_resets_detail_scroll() {
        let mut state = make_state(3);
        state.selected = 1;
        state.detail_scroll = 7;
        state.move_up();
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_move_on_empty_list_does_not_panic() {
        let mut state = make_state(0);
        state.move_down();
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_selected_item_returns_first_by_default() {
        let items = vec![make_item(Role::Assistant), make_item(Role::ToolUse)];
        let mut state = AppState::new(items, "f".to_string());
        state.all_items[0].summary = "first".to_string();
        let item = state.selected_item().unwrap();
        assert_eq!(item.summary, "first");
    }

    #[test]
    fn test_selected_item_none_when_empty() {
        let state = make_state(0);
        assert!(state.selected_item().is_none());
    }

    #[test]
    fn test_clamp_scroll_scrolls_down_when_selected_ahead() {
        let mut state = make_state(5);
        state.selected = 4;
        state.clamp_scroll(3); // height 3, selected=4 → scroll to 4-3+1=2
        assert_eq!(state.list_scroll, 2);
    }

    #[test]
    fn test_clamp_scroll_scrolls_up_when_selected_behind() {
        let mut state = make_state(5);
        state.selected = 0;
        state.list_scroll = 3;
        state.clamp_scroll(3);
        assert_eq!(state.list_scroll, 0);
    }

    #[test]
    fn test_clamp_scroll_zero_height_no_change() {
        let mut state = make_state(3);
        state.list_scroll = 5;
        state.clamp_scroll(0);
        assert_eq!(state.list_scroll, 5);
    }

    #[test]
    fn test_clamp_scroll_within_window_unchanged() {
        let mut state = make_state(5);
        state.selected = 2;
        state.list_scroll = 1;
        state.clamp_scroll(5); // window big enough
        assert_eq!(state.list_scroll, 1);
    }
}
