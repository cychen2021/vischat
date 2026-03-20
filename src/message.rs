use serde::Deserialize;
use serde_json::Value;

// ── Raw JSONL deserialization ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RawRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    pub subtype: Option<String>,
    pub session_id: Option<String>,
    pub message: Option<RawMessage>,
    pub uuid: Option<String>,
    pub timestamp: Option<String>,
    // system init fields
    pub cwd: Option<String>,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub id: Option<String>,
    pub role: Option<String>,
    pub content: Option<Value>,
}

// ── Parsed content blocks ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: ToolResultContent,
        is_error: bool,
    },
}

#[derive(Debug, Clone)]
pub enum ToolResultContent {
    References(Vec<String>),
    Text(String),
    Raw(Value),
}

impl ContentBlock {
    pub fn from_value(v: &Value) -> Option<ContentBlock> {
        let block_type = v.get("type")?.as_str()?;
        match block_type {
            "thinking" => {
                let thinking = v.get("thinking")?.as_str()?.to_string();
                let signature = v
                    .get("signature")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                Some(ContentBlock::Thinking {
                    thinking,
                    signature,
                })
            }
            "text" => {
                let text = v.get("text")?.as_str()?.to_string();
                Some(ContentBlock::Text { text })
            }
            "tool_use" => {
                let id = v.get("id")?.as_str()?.to_string();
                let name = v.get("name")?.as_str()?.to_string();
                let input = v.get("input").cloned().unwrap_or(Value::Null);
                Some(ContentBlock::ToolUse { id, name, input })
            }
            "tool_result" => {
                let tool_use_id = v.get("tool_use_id")?.as_str()?.to_string();
                let is_error = v.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                let content_val = v.get("content");
                let content = parse_tool_result_content(content_val);
                Some(ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                })
            }
            _ => None,
        }
    }
}

fn parse_tool_result_content(val: Option<&Value>) -> ToolResultContent {
    match val {
        None => ToolResultContent::Text(String::new()),
        Some(Value::String(s)) => ToolResultContent::Text(s.clone()),
        Some(Value::Array(arr)) => {
            // Check if all items are tool_reference
            let refs: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "tool_reference" {
                        item.get("tool_name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            if !refs.is_empty() {
                ToolResultContent::References(refs)
            } else {
                // Could be text content items
                let texts: Vec<String> = arr
                    .iter()
                    .filter_map(|item| {
                        if item.get("type")?.as_str()? == "text" {
                            item.get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !texts.is_empty() {
                    ToolResultContent::Text(texts.join("\n"))
                } else {
                    ToolResultContent::Raw(Value::Array(arr.clone()))
                }
            }
        }
        Some(other) => ToolResultContent::Raw(other.clone()),
    }
}

// ── Logical messages (grouped) ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LogicalMessage {
    SystemInit {
        session_id: String,
        model: String,
        cwd: String,
        tools: Vec<String>,
    },
    AssistantTurn {
        id: String,
        blocks: Vec<ContentBlock>,
    },
    UserTurn {
        blocks: Vec<ContentBlock>,
    },
}

// ── Display role ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    System,
    Thinking,
    Assistant,
    ToolUse,
    ToolResult,
}

// ── DisplayItem: one row in the list pane ───────────────────────────────────

#[derive(Debug, Clone)]
pub struct DisplayItem {
    pub role: Role,
    pub badge: &'static str,
    pub summary: String,
    pub detail: String,
}

impl DisplayItem {
    pub fn from_logical(msg: &LogicalMessage) -> Vec<DisplayItem> {
        match msg {
            LogicalMessage::SystemInit {
                session_id,
                model,
                cwd,
                tools,
            } => {
                let summary = format!(
                    "session={} model={} cwd={} tools={}",
                    &session_id[..session_id.len().min(8)],
                    model.split('/').next_back().unwrap_or(model),
                    cwd,
                    tools.len()
                );
                let detail = format!(
                    "Session:  {}\nModel:    {}\nCwd:      {}\nTools ({}):\n  {}",
                    session_id,
                    model,
                    cwd,
                    tools.len(),
                    tools.join(", ")
                );
                vec![DisplayItem {
                    role: Role::System,
                    badge: "[SYS]",
                    summary,
                    detail,
                }]
            }
            LogicalMessage::AssistantTurn { id: _, blocks } => {
                blocks.iter().map(block_to_display_item).collect()
            }
            LogicalMessage::UserTurn { blocks } => {
                blocks.iter().map(block_to_display_item).collect()
            }
        }
    }
}

fn block_to_display_item(block: &ContentBlock) -> DisplayItem {
    match block {
        ContentBlock::Thinking { thinking, .. } => {
            let summary = first_line(thinking, 80);
            DisplayItem {
                role: Role::Thinking,
                badge: "[THINK]",
                summary,
                detail: thinking.clone(),
            }
        }
        ContentBlock::Text { text } => {
            let summary = first_line(text, 80);
            DisplayItem {
                role: Role::Assistant,
                badge: "[ASST]",
                summary,
                detail: text.clone(),
            }
        }
        ContentBlock::ToolUse { name, input, .. } => {
            let input_str = serde_json::to_string_pretty(input).unwrap_or_default();
            let summary = format!("{} · {}", name, first_line(&input_str, 60));
            let detail = format!("[TOOL USE] {}\n{}", name, input_str);
            DisplayItem {
                role: Role::ToolUse,
                badge: "[TOOL>]",
                summary,
                detail,
            }
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            let (summary_text, detail_text) = match content {
                ToolResultContent::References(refs) => {
                    let s = refs.join(", ");
                    (s.clone(), format!("Tools: {}", s))
                }
                ToolResultContent::Text(t) => (first_line(t, 80), t.clone()),
                ToolResultContent::Raw(v) => {
                    let s = serde_json::to_string_pretty(v).unwrap_or_default();
                    (first_line(&s, 80), s)
                }
            };
            let err_mark = if *is_error { " [ERROR]" } else { "" };
            DisplayItem {
                role: Role::ToolResult,
                badge: "[TOOL<]",
                summary: format!(
                    "{}{} · {}",
                    tool_use_id.get(..8).unwrap_or(tool_use_id),
                    err_mark,
                    summary_text
                ),
                detail: detail_text,
            }
        }
    }
}

fn first_line(s: &str, max_len: usize) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    if line.chars().count() > max_len {
        let truncated: String = line.chars().take(max_len).collect();
        format!("{}…", truncated)
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_first_line_normal() {
        assert_eq!(first_line("hello world", 80), "hello world");
    }

    #[test]
    fn test_first_line_truncate() {
        let long = "a".repeat(100);
        let result = first_line(&long, 80);
        assert!(result.ends_with('…'));
        // 80 ASCII chars + 1 ellipsis char
        assert_eq!(result.chars().count(), 81);
    }

    #[test]
    fn test_first_line_multiline() {
        assert_eq!(first_line("first\nsecond\nthird", 80), "first");
    }

    #[test]
    fn test_first_line_trims_whitespace() {
        assert_eq!(first_line("  hello  ", 80), "hello");
    }

    #[test]
    fn test_first_line_empty() {
        assert_eq!(first_line("", 80), "");
    }

    #[test]
    fn test_content_block_thinking() {
        let v = json!({"type": "thinking", "thinking": "Let me think", "signature": "sig123"});
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me think");
                assert_eq!(signature, Some("sig123".to_string()));
            }
            _ => panic!("Expected Thinking"),
        }
    }

    #[test]
    fn test_content_block_thinking_no_signature() {
        let v = json!({"type": "thinking", "thinking": "hmm"});
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::Thinking { signature, .. } => assert!(signature.is_none()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_text() {
        let v = json!({"type": "text", "text": "Hello!"});
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::Text { text } => assert_eq!(text, "Hello!"),
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_use() {
        let v = json!({
            "type": "tool_use",
            "id": "tool-001",
            "name": "Bash",
            "input": {"command": "ls"}
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tool-001");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_use_missing_input_uses_null() {
        let v = json!({"type": "tool_use", "id": "t1", "name": "Read"});
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolUse { input, .. } => assert!(input.is_null()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_result_string() {
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "tool-001",
            "content": "result text",
            "is_error": false
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tool-001");
                assert!(!is_error);
                match content {
                    ToolResultContent::Text(t) => assert_eq!(t, "result text"),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_result_error_flag() {
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "t2",
            "content": "error msg",
            "is_error": true
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_result_references() {
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "t3",
            "content": [
                {"type": "tool_reference", "tool_name": "Bash"},
                {"type": "tool_reference", "tool_name": "Read"}
            ]
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::References(refs) => {
                    assert_eq!(refs, vec!["Bash", "Read"]);
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_result_text_array() {
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "t4",
            "content": [
                {"type": "text", "text": "line one"},
                {"type": "text", "text": "line two"}
            ]
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::Text(t) => {
                    assert!(t.contains("line one") && t.contains("line two"))
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_tool_result_no_content() {
        let v = json!({"type": "tool_result", "tool_use_id": "t5"});
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::Text(t) => assert!(t.is_empty()),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_content_block_unknown_type_returns_none() {
        let v = json!({"type": "unknown_block"});
        assert!(ContentBlock::from_value(&v).is_none());
    }

    #[test]
    fn test_content_block_missing_type_returns_none() {
        let v = json!({"text": "no type field"});
        assert!(ContentBlock::from_value(&v).is_none());
    }

    #[test]
    fn test_display_item_system_init() {
        let msg = LogicalMessage::SystemInit {
            session_id: "abc12345def".to_string(),
            model: "anthropic/claude-test".to_string(),
            cwd: "/app".to_string(),
            tools: vec!["Bash".to_string(), "Read".to_string()],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].role, Role::System);
        assert_eq!(items[0].badge, "[SYS]");
        assert!(items[0].summary.contains("claude-test"));
        assert!(items[0].detail.contains("Bash"));
        assert!(items[0].detail.contains("Read"));
    }

    #[test]
    fn test_display_item_system_init_empty_tools() {
        let msg = LogicalMessage::SystemInit {
            session_id: "s".to_string(),
            model: "m".to_string(),
            cwd: "/".to_string(),
            tools: vec![],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("tools=0"));
    }

    #[test]
    fn test_display_item_assistant_turn_blocks() {
        let msg = LogicalMessage::AssistantTurn {
            id: "msg-001".to_string(),
            blocks: vec![
                ContentBlock::Thinking {
                    thinking: "hmm".to_string(),
                    signature: None,
                },
                ContentBlock::Text {
                    text: "response".to_string(),
                },
            ],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].role, Role::Thinking);
        assert_eq!(items[0].badge, "[THINK]");
        assert_eq!(items[1].role, Role::Assistant);
        assert_eq!(items[1].badge, "[ASST]");
    }

    #[test]
    fn test_display_item_tool_use() {
        let msg = LogicalMessage::AssistantTurn {
            id: "msg-002".to_string(),
            blocks: vec![ContentBlock::ToolUse {
                id: "toolu-001".to_string(),
                name: "Bash".to_string(),
                input: json!({"command": "ls"}),
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].role, Role::ToolUse);
        assert_eq!(items[0].badge, "[TOOL>]");
        assert!(items[0].summary.contains("Bash"));
        assert!(items[0].detail.contains("ls"));
    }

    #[test]
    fn test_display_item_tool_result_error_mark() {
        let msg = LogicalMessage::UserTurn {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu-001".to_string(),
                content: ToolResultContent::Text("fail".to_string()),
                is_error: true,
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].role, Role::ToolResult);
        assert!(items[0].summary.contains("[ERROR]"));
    }

    #[test]
    fn test_display_item_tool_result_ok() {
        let msg = LogicalMessage::UserTurn {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu-abcdefgh".to_string(),
                content: ToolResultContent::Text("success output".to_string()),
                is_error: false,
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert!(!items[0].summary.contains("[ERROR]"));
        // Summary contains first 8 chars of tool_use_id
        assert!(items[0].summary.contains("toolu-ab"));
    }

    #[test]
    fn test_tool_result_content_raw_from_non_array_value() {
        // content is a JSON object (not string or array) → Raw variant
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "t6",
            "content": {"key": "value"}
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::Raw(_) => {}
                _ => panic!("Expected Raw"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_tool_result_content_raw_from_mixed_array() {
        // array with items that are neither tool_reference nor text → Raw(Array)
        let v = json!({
            "type": "tool_result",
            "tool_use_id": "t7",
            "content": [
                {"type": "image", "url": "http://example.com/img.png"}
            ]
        });
        let block = ContentBlock::from_value(&v).unwrap();
        match block {
            ContentBlock::ToolResult { content, .. } => match content {
                ToolResultContent::Raw(_) => {}
                _ => panic!("Expected Raw"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_display_item_tool_result_raw_content() {
        let msg = LogicalMessage::UserTurn {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu-raw12".to_string(),
                content: ToolResultContent::Raw(json!({"status": "ok"})),
                is_error: false,
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].role, Role::ToolResult);
        assert!(items[0].detail.contains("status"));
    }

    #[test]
    fn test_display_item_tool_result_references_content() {
        let msg = LogicalMessage::UserTurn {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu-ref123".to_string(),
                content: ToolResultContent::References(vec!["Bash".to_string(), "Read".to_string()]),
                is_error: false,
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("Bash"));
        assert!(items[0].detail.contains("Read"));
    }

    #[test]
    fn test_tool_use_id_shorter_than_8_chars() {
        // tool_use_id shorter than 8 chars — uses the full id in summary
        let msg = LogicalMessage::UserTurn {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "short".to_string(),
                content: ToolResultContent::Text("ok".to_string()),
                is_error: false,
            }],
        };
        let items = DisplayItem::from_logical(&msg);
        assert!(items[0].summary.contains("short"));
    }
}
