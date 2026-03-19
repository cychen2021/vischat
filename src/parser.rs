use anyhow::{Context, Result};
use std::fs;

use crate::message::{ContentBlock, LogicalMessage, RawRecord};

pub fn parse_file(path: &str) -> Result<Vec<LogicalMessage>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))?;
    parse_str(&content)
}

pub fn parse_str(content: &str) -> Result<Vec<LogicalMessage>> {
    let mut messages: Vec<LogicalMessage> = Vec::new();

    // For grouping assistant blocks by message id
    let mut current_assistant_id: Option<String> = None;
    let mut current_assistant_blocks: Vec<ContentBlock> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let record: RawRecord = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON on line {}", line_num + 1))?;

        match record.record_type.as_str() {
            "system" => {
                // Flush any pending assistant turn
                flush_assistant(
                    &mut current_assistant_id,
                    &mut current_assistant_blocks,
                    &mut messages,
                );

                if record.subtype.as_deref() == Some("init") {
                    messages.push(LogicalMessage::SystemInit {
                        session_id: record.session_id.unwrap_or_default(),
                        model: record.model.unwrap_or_default(),
                        cwd: record.cwd.unwrap_or_default(),
                        tools: record.tools.unwrap_or_default(),
                    });
                }
            }
            "assistant" => {
                if let Some(msg) = &record.message {
                    let msg_id = msg.id.clone().unwrap_or_default();

                    // If the message id changed, flush the previous turn
                    if current_assistant_id.as_deref() != Some(&msg_id) {
                        flush_assistant(
                            &mut current_assistant_id,
                            &mut current_assistant_blocks,
                            &mut messages,
                        );
                        current_assistant_id = Some(msg_id);
                    }

                    // Parse content blocks from this record
                    if let Some(content_val) = &msg.content {
                        if let Some(arr) = content_val.as_array() {
                            for block_val in arr {
                                if let Some(block) = ContentBlock::from_value(block_val) {
                                    current_assistant_blocks.push(block);
                                }
                            }
                        }
                    }
                }
            }
            "user" => {
                // Flush any pending assistant turn before user turn
                flush_assistant(
                    &mut current_assistant_id,
                    &mut current_assistant_blocks,
                    &mut messages,
                );

                if let Some(msg) = &record.message {
                    if let Some(content_val) = &msg.content {
                        let mut blocks = Vec::new();
                        if let Some(arr) = content_val.as_array() {
                            for block_val in arr {
                                if let Some(block) = ContentBlock::from_value(block_val) {
                                    blocks.push(block);
                                }
                            }
                        }
                        if !blocks.is_empty() {
                            messages.push(LogicalMessage::UserTurn { blocks });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Flush trailing assistant turn
    flush_assistant(
        &mut current_assistant_id,
        &mut current_assistant_blocks,
        &mut messages,
    );

    Ok(messages)
}

fn flush_assistant(
    id: &mut Option<String>,
    blocks: &mut Vec<ContentBlock>,
    messages: &mut Vec<LogicalMessage>,
) {
    if let Some(msg_id) = id.take() {
        if !blocks.is_empty() {
            messages.push(LogicalMessage::AssistantTurn {
                id: msg_id,
                blocks: std::mem::take(blocks),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ContentBlock;

    const SYSTEM_LINE: &str = r#"{"type":"system","subtype":"init","cwd":"/app","session_id":"sess-001","tools":["Bash","Read"],"model":"claude-test","uuid":"uuid-sys"}"#;
    const ASSISTANT_LINE: &str = r#"{"type":"assistant","message":{"id":"msg-001","role":"assistant","content":[{"type":"text","text":"Hello!"}]},"session_id":"sess-001","uuid":"uuid-asst"}"#;
    const USER_LINE: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tool-001","content":"done","is_error":false}]},"session_id":"sess-001","uuid":"uuid-user"}"#;

    #[test]
    fn test_parse_empty_string() {
        let messages = parse_str("").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let messages = parse_str("   \n\n  ").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_parse_system_init() {
        let messages = parse_str(SYSTEM_LINE).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LogicalMessage::SystemInit {
                session_id,
                model,
                cwd,
                tools,
            } => {
                assert_eq!(session_id, "sess-001");
                assert_eq!(model, "claude-test");
                assert_eq!(cwd, "/app");
                assert_eq!(tools, &["Bash", "Read"]);
            }
            _ => panic!("Expected SystemInit"),
        }
    }

    #[test]
    fn test_parse_system_non_init_subtype_ignored() {
        let line = r#"{"type":"system","subtype":"other","session_id":"s","uuid":"u"}"#;
        let messages = parse_str(line).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_parse_assistant_turn() {
        let messages = parse_str(ASSISTANT_LINE).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LogicalMessage::AssistantTurn { id, blocks } => {
                assert_eq!(id, "msg-001");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], ContentBlock::Text { text } if text == "Hello!"));
            }
            _ => panic!("Expected AssistantTurn"),
        }
    }

    #[test]
    fn test_parse_user_turn() {
        let messages = parse_str(USER_LINE).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LogicalMessage::UserTurn { blocks } => assert_eq!(blocks.len(), 1),
            _ => panic!("Expected UserTurn"),
        }
    }

    #[test]
    fn test_parse_user_empty_blocks_not_emitted() {
        // User message with content that parses to zero valid blocks → no UserTurn
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"unknown"}]},"session_id":"s","uuid":"u"}"#;
        let messages = parse_str(line).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_parse_full_conversation() {
        let content = format!("{}\n{}\n{}", SYSTEM_LINE, ASSISTANT_LINE, USER_LINE);
        let messages = parse_str(&content).unwrap();
        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0], LogicalMessage::SystemInit { .. }));
        assert!(matches!(messages[1], LogicalMessage::AssistantTurn { .. }));
        assert!(matches!(messages[2], LogicalMessage::UserTurn { .. }));
    }

    #[test]
    fn test_parse_assistant_blocks_grouped_by_same_id() {
        let line1 = r#"{"type":"assistant","message":{"id":"msg-001","role":"assistant","content":[{"type":"thinking","thinking":"hmm","signature":"s"}]},"session_id":"s","uuid":"u1"}"#;
        let line2 = r#"{"type":"assistant","message":{"id":"msg-001","role":"assistant","content":[{"type":"text","text":"result"}]},"session_id":"s","uuid":"u2"}"#;
        let messages = parse_str(&format!("{}\n{}", line1, line2)).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LogicalMessage::AssistantTurn { blocks, .. } => assert_eq!(blocks.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn test_parse_assistant_blocks_split_by_different_id() {
        let line1 = r#"{"type":"assistant","message":{"id":"msg-001","role":"assistant","content":[{"type":"text","text":"first"}]},"session_id":"s","uuid":"u1"}"#;
        let line2 = r#"{"type":"assistant","message":{"id":"msg-002","role":"assistant","content":[{"type":"text","text":"second"}]},"session_id":"s","uuid":"u2"}"#;
        let messages = parse_str(&format!("{}\n{}", line1, line2)).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_parse_skips_blank_lines() {
        let content = format!("\n{}\n\n{}\n", SYSTEM_LINE, ASSISTANT_LINE);
        let messages = parse_str(&content).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        let result = parse_str("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_trailing_assistant_turn_flushed() {
        // Assistant turn at end of file (no following user/system) should still be emitted
        let messages = parse_str(ASSISTANT_LINE).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0], LogicalMessage::AssistantTurn { .. }));
    }

    #[test]
    fn test_parse_unknown_record_type_ignored() {
        let line = r#"{"type":"summary","content":"some summary","session_id":"s","uuid":"u"}"#;
        let messages = parse_str(line).unwrap();
        assert!(messages.is_empty());
    }
}
