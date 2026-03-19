use crate::message::{DisplayItem, Role};

pub struct AppState {
    pub all_items: Vec<DisplayItem>,
    pub selected: usize,
    pub list_scroll: usize,
    pub detail_scroll: usize,
    pub show_thinking: bool,
    pub expanded: bool,
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
            expanded: false,
            quit: false,
            file_path,
        }
    }

    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
        self.detail_scroll = 0;
    }

    /// Returns items filtered by show_thinking
    pub fn visible_items(&self) -> Vec<&DisplayItem> {
        self.all_items
            .iter()
            .filter(|item| self.show_thinking || item.role != Role::Thinking)
            .collect()
    }

    pub fn visible_count(&self) -> usize {
        self.visible_items().len()
    }

    pub fn selected_item(&self) -> Option<&DisplayItem> {
        self.visible_items().into_iter().nth(self.selected)
    }

    pub fn move_down(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            return;
        }
        if self.selected + 1 < count {
            self.selected += 1;
            self.detail_scroll = 0;
            self.expanded = false;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
            self.expanded = false;
        }
    }

    /// Ensure list_scroll keeps selected item visible given the list pane height.
    pub fn clamp_scroll(&mut self, list_height: usize) {
        if list_height == 0 {
            return;
        }
        if self.selected < self.list_scroll {
            self.list_scroll = self.selected;
        }
        if self.selected >= self.list_scroll + list_height {
            self.list_scroll = self.selected - list_height + 1;
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
    fn test_visible_items_hides_thinking_by_default() {
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::ToolUse),
        ];
        let state = AppState::new(items, "f".to_string());
        let visible = state.visible_items();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|i| i.role != Role::Thinking));
    }

    #[test]
    fn test_visible_items_shows_thinking_when_enabled() {
        let items = vec![make_item(Role::Assistant), make_item(Role::Thinking)];
        let mut state = AppState::new(items, "f".to_string());
        state.show_thinking = true;
        assert_eq!(state.visible_items().len(), 2);
    }

    #[test]
    fn test_visible_count_excludes_thinking() {
        let items = vec![
            make_item(Role::Assistant),
            make_item(Role::Thinking),
            make_item(Role::Assistant),
        ];
        let state = AppState::new(items, "f".to_string());
        assert_eq!(state.visible_count(), 2);
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
