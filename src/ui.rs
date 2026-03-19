use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::AppState;
use crate::message::Role;

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    let size = frame.area();

    if state.expanded {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(size);

        let list_area = chunks[0];
        let status_area = chunks[1];

        let list_height = list_area.height.saturating_sub(2) as usize;
        state.clamp_scroll(list_height);

        draw_list_expanded(frame, state, list_area);
        draw_status(frame, state, status_area);
    } else {
        // Layout: list (60%), detail (35%), status (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(size);

        let list_area = chunks[0];
        let detail_area = chunks[1];
        let status_area = chunks[2];

        let list_height = list_area.height.saturating_sub(2) as usize;
        state.clamp_scroll(list_height);

        draw_list(frame, state, list_area);
        draw_detail(frame, state, detail_area);
        draw_status(frame, state, status_area);
    }
}

fn badge_style(role: &Role) -> Style {
    match role {
        Role::System => Style::default().fg(Color::DarkGray),
        Role::Thinking => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::DIM),
        Role::Assistant => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Role::ToolUse => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Role::ToolResult => Style::default().fg(Color::Magenta),
    }
}

fn draw_list(frame: &mut Frame, state: &mut AppState, area: ratatui::layout::Rect) {
    let all = state.list_items();
    let scroll = state.list_scroll;
    let highlight_idx = state
        .selected_list_index()
        .and_then(|i| i.checked_sub(scroll));

    let items: Vec<ListItem> = all
        .iter()
        .enumerate()
        .skip(scroll)
        .take(area.height.saturating_sub(2) as usize)
        .map(|(i, item)| {
            let folded = item.role == Role::Thinking && !state.show_thinking;
            let local_idx = i - scroll;
            let is_selected = highlight_idx == Some(local_idx);

            let b_style = if folded {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                badge_style(&item.role)
            };
            let badge_span = Span::styled(format!("+ {:<8}", item.badge), b_style);

            let summary_text = if folded {
                "...".to_string()
            } else {
                item.summary.clone()
            };
            let s_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if folded {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::Gray)
            };
            let summary_span = Span::styled(summary_text, s_style);
            ListItem::new(Line::from(vec![badge_span, summary_span]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(highlight_idx);

    let thinking_hint = if state.show_thinking {
        " [thinking visible]"
    } else {
        " [thinking folded — press t]"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" vischat · {} {}", state.file_path, thinking_hint));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_list_expanded(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let all = state.list_items();
    let highlight_pos = state.selected_list_index();
    let scroll = state.list_scroll;
    let area_height = area.height.saturating_sub(2) as usize; // minus borders

    let mut lines: Vec<Line> = Vec::new();

    'outer: for (i, item) in all.iter().enumerate().skip(scroll) {
        let folded = item.role == Role::Thinking && !state.show_thinking;
        let raw_badge = format!("{:<8}", item.badge);
        let is_selected = highlight_pos == Some(i);

        if is_selected {
            let sel_style = Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD);
            let badge_span = Span::styled(format!("- {}", raw_badge), sel_style);
            let summary_span = Span::styled(item.summary.clone(), sel_style);
            lines.push(Line::from(vec![badge_span, summary_span]));
            if lines.len() >= area_height {
                break;
            }
            for (j, content_line) in item.detail.lines().enumerate() {
                if j >= state.detail_scroll {
                    lines.push(Line::from(format!("  {}", content_line)));
                    if lines.len() >= area_height {
                        break 'outer;
                    }
                }
            }
        } else if folded {
            let fold_style = Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM);
            let badge_span = Span::styled(format!("+ {}", raw_badge), fold_style);
            let summary_span = Span::styled("...".to_string(), fold_style);
            lines.push(Line::from(vec![badge_span, summary_span]));
            if lines.len() >= area_height {
                break;
            }
        } else {
            let badge_span = Span::styled(format!("+ {}", raw_badge), badge_style(&item.role));
            let summary_span = Span::styled(item.summary.clone(), Style::default().fg(Color::Gray));
            lines.push(Line::from(vec![badge_span, summary_span]));
            if lines.len() >= area_height {
                break;
            }
        }
    }

    let thinking_hint = if state.show_thinking {
        " [thinking visible]"
    } else {
        " [thinking folded — press t]"
    };

    let block = Block::default().borders(Borders::ALL).title(format!(
        " vischat · {} {} [expanded]",
        state.file_path, thinking_hint
    ));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn draw_detail(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let (title, content) = match state.selected_item() {
        None => ("(no selection)".to_string(), String::new()),
        Some(item) => {
            let title = format!(" {} {} ", item.badge, detail_title(item));
            (title, item.detail.clone())
        }
    };

    let lines: Vec<Line> = content
        .lines()
        .skip(state.detail_scroll)
        .map(|l| Line::from(l.to_string()))
        .collect();

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

fn detail_title(item: &crate::message::DisplayItem) -> String {
    match item.role {
        Role::System => "System Init".to_string(),
        Role::Thinking => "Thinking".to_string(),
        Role::Assistant => "Assistant Response".to_string(),
        Role::ToolUse => "Tool Invocation".to_string(),
        Role::ToolResult => "Tool Result".to_string(),
    }
}

fn draw_status(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let total = state.navigable_count();
    let pos = if total == 0 {
        "0/0".to_string()
    } else {
        format!("{}/{}", state.selected + 1, total)
    };
    let status = format!(
        " j/k:move  g/G:first/last  Ctrl-d/u:scroll  Enter/Space:expand  t:thinking  q:quit    {}",
        pos
    );
    let para = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}
