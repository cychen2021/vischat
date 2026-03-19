use vischat::app::AppState;
use vischat::message::{DisplayItem, LogicalMessage};
use vischat::parser;

const FIXTURE: &str = include_str!("fixtures/simple.jsonl");

// ── Parsing ──────────────────────────────────────────────────────────────────

#[test]
fn test_fixture_parses_three_messages() {
    let messages = parser::parse_str(FIXTURE).unwrap();
    // system init + assistant turn (grouped) + user turn
    assert_eq!(messages.len(), 3);
}

#[test]
fn test_fixture_first_message_is_system_init() {
    let messages = parser::parse_str(FIXTURE).unwrap();
    match &messages[0] {
        LogicalMessage::SystemInit {
            session_id,
            model,
            cwd,
            tools,
        } => {
            assert_eq!(session_id, "test-session");
            assert_eq!(model, "claude-test");
            assert_eq!(cwd, "/test");
            assert_eq!(tools, &["Bash", "Read"]);
        }
        _ => panic!("Expected SystemInit as first message"),
    }
}

#[test]
fn test_fixture_assistant_turn_has_three_blocks() {
    let messages = parser::parse_str(FIXTURE).unwrap();
    match &messages[1] {
        LogicalMessage::AssistantTurn { id, blocks } => {
            assert_eq!(id, "msg-001");
            assert_eq!(blocks.len(), 3); // thinking + text + tool_use
        }
        _ => panic!("Expected AssistantTurn"),
    }
}

#[test]
fn test_fixture_user_turn_has_tool_result() {
    let messages = parser::parse_str(FIXTURE).unwrap();
    match &messages[2] {
        LogicalMessage::UserTurn { blocks } => assert_eq!(blocks.len(), 1),
        _ => panic!("Expected UserTurn"),
    }
}

// ── Display items ─────────────────────────────────────────────────────────────

fn fixture_display_items() -> Vec<DisplayItem> {
    let messages = parser::parse_str(FIXTURE).unwrap();
    messages
        .iter()
        .flat_map(|m| DisplayItem::from_logical(m))
        .collect()
}

#[test]
fn test_fixture_produces_five_display_items() {
    let items = fixture_display_items();
    // [SYS] + [THINK] + [ASST] + [TOOL>] + [TOOL<]
    assert_eq!(items.len(), 5);
}

#[test]
fn test_fixture_display_item_roles_in_order() {
    use vischat::message::Role;
    let items = fixture_display_items();
    assert_eq!(items[0].role, Role::System);
    assert_eq!(items[1].role, Role::Thinking);
    assert_eq!(items[2].role, Role::Assistant);
    assert_eq!(items[3].role, Role::ToolUse);
    assert_eq!(items[4].role, Role::ToolResult);
}

#[test]
fn test_fixture_display_item_badges() {
    let items = fixture_display_items();
    assert_eq!(items[0].badge, "[SYS]");
    assert_eq!(items[1].badge, "[THINK]");
    assert_eq!(items[2].badge, "[ASST]");
    assert_eq!(items[3].badge, "[TOOL>]");
    assert_eq!(items[4].badge, "[TOOL<]");
}

#[test]
fn test_fixture_system_item_summary_contains_model() {
    let items = fixture_display_items();
    assert!(
        items[0].summary.contains("claude-test"),
        "summary: {}",
        items[0].summary
    );
}

#[test]
fn test_fixture_assistant_item_summary() {
    let items = fixture_display_items();
    assert_eq!(items[2].summary, "Hello, world!");
}

#[test]
fn test_fixture_tool_use_summary_contains_name() {
    let items = fixture_display_items();
    assert!(
        items[3].summary.contains("Bash"),
        "summary: {}",
        items[3].summary
    );
}

// ── App state navigation ──────────────────────────────────────────────────────

fn make_app() -> AppState {
    let items = fixture_display_items();
    AppState::new(items, "simple.jsonl".to_string())
}

#[test]
fn test_app_visible_count_hides_thinking_by_default() {
    let app = make_app();
    // 5 items total, 1 thinking hidden → 4 visible
    assert_eq!(app.visible_count(), 4);
}

#[test]
fn test_app_visible_count_shows_all_with_thinking() {
    let mut app = make_app();
    app.show_thinking = true;
    assert_eq!(app.visible_count(), 5);
}

#[test]
fn test_app_navigate_to_last() {
    let mut app = make_app();
    let last = app.visible_count() - 1;
    for _ in 0..last {
        app.move_down();
    }
    assert_eq!(app.selected, last);
    assert!(app.selected_item().is_some());
}

#[test]
fn test_app_navigate_back_to_first() {
    let mut app = make_app();
    app.selected = 3;
    for _ in 0..3 {
        app.move_up();
    }
    assert_eq!(app.selected, 0);
}

#[test]
fn test_app_clamp_scroll_keeps_selection_visible() {
    let mut app = make_app();
    app.selected = 3;
    app.clamp_scroll(2);
    // selected=3, height=2 → scroll = 3-2+1 = 2
    assert_eq!(app.list_scroll, 2);
}

// ── Snapshot tests ────────────────────────────────────────────────────────────

#[test]
fn snapshot_system_item_summary() {
    let items = fixture_display_items();
    insta::assert_snapshot!("system_item_summary", items[0].summary);
}

#[test]
fn snapshot_system_item_detail() {
    let items = fixture_display_items();
    insta::assert_snapshot!("system_item_detail", items[0].detail);
}

#[test]
fn snapshot_assistant_text_item() {
    let items = fixture_display_items();
    insta::assert_snapshot!("assistant_text_summary", items[2].summary);
    insta::assert_snapshot!("assistant_text_detail", items[2].detail);
}

#[test]
fn snapshot_tool_use_item() {
    let items = fixture_display_items();
    insta::assert_snapshot!("tool_use_summary", items[3].summary);
    insta::assert_snapshot!("tool_use_detail", items[3].detail);
}

#[test]
fn snapshot_tool_result_item() {
    let items = fixture_display_items();
    insta::assert_snapshot!("tool_result_summary", items[4].summary);
    insta::assert_snapshot!("tool_result_detail", items[4].detail);
}

#[test]
fn snapshot_thinking_item() {
    let items = fixture_display_items();
    insta::assert_snapshot!("thinking_summary", items[1].summary);
    insta::assert_snapshot!("thinking_detail", items[1].detail);
}
