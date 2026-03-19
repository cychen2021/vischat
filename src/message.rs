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
                Some(ContentBlock::Thinking { thinking, signature })
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
                let is_error = v
                    .get("is_error")
                    .and_then(|e| e.as_bool())
                    .unwrap_or(false);
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
                            item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
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
                    model.split('/').last().unwrap_or(model),
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
            let summary = format!(
                "{} · {}",
                name,
                first_line(&input_str, 60)
            );
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
                ToolResultContent::Text(t) => {
                    (first_line(t, 80), t.clone())
                }
                ToolResultContent::Raw(v) => {
                    let s = serde_json::to_string_pretty(v).unwrap_or_default();
                    (first_line(&s, 80), s)
                }
            };
            let err_mark = if *is_error { " [ERROR]" } else { "" };
            DisplayItem {
                role: Role::ToolResult,
                badge: "[TOOL<]",
                summary: format!("{}{} · {}", tool_use_id.get(..8).unwrap_or(tool_use_id), err_mark, summary_text),
                detail: detail_text,
            }
        }
    }
}

fn first_line(s: &str, max_len: usize) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    if line.len() > max_len {
        format!("{}…", &line[..max_len])
    } else {
        line.to_string()
    }
}
