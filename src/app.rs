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
        if self.selected < self.list_scroll {
            self.list_scroll = self.selected;
        }
        if self.selected >= self.list_scroll + list_height {
            self.list_scroll = self.selected - list_height + 1;
        }
    }
}
