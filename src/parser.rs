use anyhow::{Context, Result};
use std::fs;

use crate::message::{ContentBlock, LogicalMessage, RawRecord};

pub fn parse_file(path: &str) -> Result<Vec<LogicalMessage>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;
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
                flush_assistant(&mut current_assistant_id, &mut current_assistant_blocks, &mut messages);

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
                        flush_assistant(&mut current_assistant_id, &mut current_assistant_blocks, &mut messages);
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
                flush_assistant(&mut current_assistant_id, &mut current_assistant_blocks, &mut messages);

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
    flush_assistant(&mut current_assistant_id, &mut current_assistant_blocks, &mut messages);

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
