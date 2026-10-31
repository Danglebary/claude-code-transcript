//! One line of a transcript read into what it records. Everything here is pure.
//!
//! The shape is the one Claude Code's files hold, read off this machine's transcripts
//! (checked 2026-10-30): one JSON object per line, written without spaces; the record's
//! kind under `type`; the session under `sessionId`, the instant under `timestamp`, and a
//! subagent's id under `agentId`; an assistant's or user's message under `message`, whose
//! `content` is a string or a list of blocks; an assistant message's id, model, and token
//! counts under `id`, `model`, and `usage`, with the effort under the record's `effort`;
//! a tool result's call under `tool_use_id` and its output under `content`, as a string
//! or a list of blocks; an output too large to inline saved to a file the record names
//! under `toolUseResult.persistedOutputPath`; an attachment under `attachment`, its kind
//! under `type`; and a system record's kind under `subtype`, a compaction boundary's
//! being `compact_boundary`.

use jiff::Timestamp;
use serde_json::Value;

/// The most tokens one usage field is read as holding; a field above it is malformed. A
/// context window holds a small fraction of this.
pub const TOKENS_PER_FIELD_MAX: u64 = 10_000_000;

/// The pointer a malformed record names where the line is no JSON object.
pub const WHOLE_LINE: &str = "";

const ASSISTANT_MARK: &str = "\"type\":\"assistant\"";
const USER_MARK: &str = "\"type\":\"user\"";
const ATTACHMENT_MARK: &str = "\"type\":\"attachment\"";
const COMPACTION_MARK: &str = "\"subtype\":\"compact_boundary\"";
const COMPACT_BOUNDARY_SUBTYPE: &str = "compact_boundary";
const SKILL_BODY_OPENING: &str = "Base directory for this skill";
const SLASH_COMMAND_MARK: &str = "<command-name>";
const INTERRUPTION_OPENING: &str = "[Request interrupted";
const TAG_OPENING: char = '<';
const LINE_END: char = '\n';

/// What one line of a transcript records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A model call's record.
    Assistant(Assistant),
    /// The user's turn: what they typed, or the results of the model's tool calls.
    User(User),
    /// Something Claude Code attached to the context, such as a hook's output.
    Attachment(Attachment),
    /// A compaction boundary, where Claude Code replaced the context with a summary of it.
    Compaction(Compaction),
    /// A record of a kind read here none of, such as a queue operation or a cost summary.
    Other,
    /// A record whose field fails its check. The JSON pointer names the field, a block's
    /// index written as `*`, or is [`WHOLE_LINE`] where the line is no JSON object.
    Malformed(&'static str),
}

/// What every record read here carries beside its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// The session the record belongs to.
    pub session: String,
    /// When Claude Code wrote the record.
    pub timestamp: Timestamp,
    /// The subagent whose transcript holds the record; none on a main thread.
    pub agent_id: Option<String>,
}

/// A model call's record. Claude Code writes a call's record once for each of its
/// content blocks, every copy carrying the call's message id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assistant {
    /// The session, instant, and subagent the record carries.
    pub envelope: Envelope,
    /// The id every record of the call carries.
    pub message_id: String,
    /// The model the call reached.
    pub model: String,
    /// The effort the call ran at, where the record names one.
    pub effort: Option<String>,
    /// The tokens the call reports, as of this record.
    pub usage: Usage,
    /// The blocks this record holds of the call's content.
    pub blocks: Vec<AssistantBlock>,
}

/// The tokens one call reports.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// Input read uncached.
    pub input: u64,
    /// Input read from the cache.
    pub cache_read: u64,
    /// Input written to the cache that lives five minutes.
    pub cache_write_5m: u64,
    /// Input written to the cache that lives an hour.
    pub cache_write_1h: u64,
    /// Output written, thinking included.
    pub output: u64,
    /// The part of `output` the model spent thinking.
    pub thinking: u64,
}

/// One block of a model call's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistantBlock {
    /// Text the model wrote.
    Text(String),
    /// A tool the model called.
    ToolUse(ToolUse),
    /// The model's thinking, whose text is not kept.
    Thinking,
    /// A block of a kind read here none of.
    Other,
}

/// A tool call a model makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUse {
    /// The id the call's result names.
    pub id: String,
    /// The tool called, such as `Bash` or `Read`.
    pub name: String,
    /// The call's input whole, its fields each tool's own.
    pub input: Value,
}

/// The user's turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The session, instant, and subagent the record carries.
    pub envelope: Envelope,
    /// Whether Claude Code wrote the record into the session itself, such as a skill's
    /// body, rather than anyone typing it.
    pub is_meta: bool,
    /// The blocks of the turn's content, a string content read as one text block.
    pub blocks: Vec<UserBlock>,
}

/// One block of the user's turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserBlock {
    /// Text in the user's turn, with who put it there.
    Text {
        /// The text whole.
        text: String,
        /// Whether the user typed it or Claude Code put it there.
        class: TextClass,
    },
    /// What a tool call returned.
    ToolResult(ToolResult),
    /// An image in the turn, whose data is not kept.
    Image,
    /// A block of a kind read here none of.
    Other,
}

/// Who put a piece of text in the user's turn.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextClass {
    /// Text the user typed.
    Typed,
    /// The body of a skill Claude Code loads into the conversation.
    SkillBody,
    /// A slash command the user ran, with its expansion.
    SlashCommand,
    /// The note Claude Code leaves where the user interrupted a turn.
    Interruption,
    /// Text Claude Code wraps in a tag, named here, such as `task-notification` or
    /// `bash-input`.
    Tagged(String),
}

/// What a tool call returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    /// The id of the call returning it.
    pub tool_use_id: String,
    /// Whether Claude Code marked the result an error, a hook's refusal among them.
    pub is_error: bool,
    /// The output's parts, a string output read as one text part.
    pub parts: Vec<ResultPart>,
    /// The file Claude Code saved an output too large to inline to. A record names one
    /// saved path, so it is the result's only where the record holds one result.
    pub persisted_output: Option<String>,
}

/// One part of a tool's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultPart {
    /// Text the tool returned.
    Text(String),
    /// An image the tool returned, whose data is not kept.
    Image,
    /// A part of another kind, such as a document or a tool reference, kept whole.
    Other(Value),
}

/// Something Claude Code attached to the context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// The session, instant, and subagent the record carries.
    pub envelope: Envelope,
    /// The attachment's kind, such as `hook_success`.
    pub kind: String,
    /// The attachment whole.
    pub body: Value,
}

/// Where Claude Code replaced a thread's context with a summary of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compaction {
    /// The session, instant, and subagent the record carries.
    pub envelope: Envelope,
}

/// Reads one line of a transcript, its line end left off.
///
/// # Panics
///
/// When `text` holds a line end, which a line [`crate::lines::Lines`] reads does not.
#[must_use]
pub fn read_line(text: &str) -> Line {
    assert!(!text.contains(LINE_END), "a line holds no line end");
    // A record read here holds its kind in one of these forms, so a line holding none is
    // passed over unparsed.
    let assistant = text.contains(ASSISTANT_MARK);
    let user = text.contains(USER_MARK);
    let attachment = text.contains(ATTACHMENT_MARK);
    let compaction = text.contains(COMPACTION_MARK);
    let marked = assistant || user || attachment || compaction;
    if !marked {
        return Line::Other;
    }
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Line::Malformed(WHOLE_LINE);
    };
    if !value.is_object() {
        return Line::Malformed(WHOLE_LINE);
    }
    let read = match value.get("type").and_then(Value::as_str) {
        Some("assistant") => assistant_of(&value).map(Line::Assistant),
        Some("user") => user_of(&value).map(Line::User),
        Some("attachment") => attachment_of(&value).map(Line::Attachment),
        Some("system") => system_of(&value),
        Some(_) | None => Ok(Line::Other),
    };
    match read {
        Ok(line) => line,
        Err(pointer) => Line::Malformed(pointer),
    }
}

/// Who put a piece of text in the user's turn, by how it opens or what it holds.
#[must_use]
pub fn class_of(text: &str) -> TextClass {
    if text.starts_with(SKILL_BODY_OPENING) {
        return TextClass::SkillBody;
    }
    if text.contains(SLASH_COMMAND_MARK) {
        return TextClass::SlashCommand;
    }
    if text.starts_with(INTERRUPTION_OPENING) {
        return TextClass::Interruption;
    }
    let Some(rest) = text.strip_prefix(TAG_OPENING) else {
        return TextClass::Typed;
    };
    let tag: String = rest
        .chars()
        .take_while(|&letter| is_tag_letter(letter))
        .collect();
    if tag.is_empty() {
        TextClass::Typed
    } else {
        TextClass::Tagged(tag)
    }
}

const fn is_tag_letter(letter: char) -> bool {
    if letter.is_ascii_alphanumeric() {
        return true;
    }
    letter == '-' || letter == '_'
}

fn envelope_of(value: &Value) -> Result<Envelope, &'static str> {
    let session = name_at(value, "/sessionId", "/sessionId")?;
    let stamp = string_at(value, "/timestamp", "/timestamp")?;
    let Ok(timestamp) = stamp.parse::<Timestamp>() else {
        return Err("/timestamp");
    };
    let agent_id = match value.get("agentId") {
        None | Some(Value::Null) => None,
        Some(_) => Some(name_at(value, "/agentId", "/agentId")?),
    };
    Ok(Envelope {
        session,
        timestamp,
        agent_id,
    })
}

fn assistant_of(value: &Value) -> Result<Assistant, &'static str> {
    let envelope = envelope_of(value)?;
    let message_id = name_at(value, "/message/id", "/message/id")?;
    let model = name_at(value, "/message/model", "/message/model")?;
    let effort = match value.get("effort") {
        None | Some(Value::Null) => None,
        Some(_) => Some(name_at(value, "/effort", "/effort")?),
    };
    let usage = usage_of(value)?;
    let Some(Value::Array(content)) = value.pointer("/message/content") else {
        return Err("/message/content");
    };
    let mut blocks = Vec::with_capacity(content.len());
    for block in content {
        let read = assistant_block_of(block)?;
        blocks.push(read);
    }
    assert_eq!(
        blocks.len(),
        content.len(),
        "every block is read or refused"
    );
    Ok(Assistant {
        envelope,
        message_id,
        model,
        effort,
        usage,
        blocks,
    })
}

fn usage_of(value: &Value) -> Result<Usage, &'static str> {
    let Some(Value::Object(_)) = value.pointer("/message/usage") else {
        return Err("/message/usage");
    };
    let usage = Usage {
        input: tokens_at(value, "/message/usage/input_tokens")?,
        cache_read: tokens_at(value, "/message/usage/cache_read_input_tokens")?,
        cache_write_5m: tokens_at(
            value,
            "/message/usage/cache_creation/ephemeral_5m_input_tokens",
        )?,
        cache_write_1h: tokens_at(
            value,
            "/message/usage/cache_creation/ephemeral_1h_input_tokens",
        )?,
        output: tokens_at(value, "/message/usage/output_tokens")?,
        thinking: tokens_at(
            value,
            "/message/usage/output_tokens_details/thinking_tokens",
        )?,
    };
    assert!(
        usage.output <= TOKENS_PER_FIELD_MAX,
        "the output is bounded"
    );
    Ok(usage)
}

fn assistant_block_of(block: &Value) -> Result<AssistantBlock, &'static str> {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => {
            let text = string_at(block, "/text", "/message/content/*/text")?;
            Ok(AssistantBlock::Text(text))
        }
        Some("tool_use") => {
            let id = name_at(block, "/id", "/message/content/*/id")?;
            let name = name_at(block, "/name", "/message/content/*/name")?;
            let Some(input) = block.get("input") else {
                return Err("/message/content/*/input");
            };
            Ok(AssistantBlock::ToolUse(ToolUse {
                id,
                name,
                input: input.clone(),
            }))
        }
        Some("thinking" | "redacted_thinking") => Ok(AssistantBlock::Thinking),
        Some(_) => Ok(AssistantBlock::Other),
        None => Err("/message/content/*/type"),
    }
}

fn user_of(value: &Value) -> Result<User, &'static str> {
    let envelope = envelope_of(value)?;
    let is_meta = flag_at(value, "/isMeta", "/isMeta")?;
    let blocks = match value.pointer("/message/content") {
        Some(Value::String(text)) => vec![text_block(text)],
        Some(Value::Array(content)) => user_blocks_of(value, content)?,
        _ => return Err("/message/content"),
    };
    Ok(User {
        envelope,
        is_meta,
        blocks,
    })
}

fn user_blocks_of(value: &Value, content: &[Value]) -> Result<Vec<UserBlock>, &'static str> {
    let mut results: usize = 0;
    for block in content {
        if block.get("type").and_then(Value::as_str) == Some("tool_result") {
            let Some(more) = results.checked_add(1) else {
                unreachable!("the blocks of a bounded line fit a usize")
            };
            results = more;
        }
    }
    // A record names a single saved path, which can belong to a single result alone.
    let persisted_output = if results == 1 {
        match value.pointer("/toolUseResult/persistedOutputPath") {
            None | Some(Value::Null) => None,
            Some(_) => Some(name_at(
                value,
                "/toolUseResult/persistedOutputPath",
                "/toolUseResult/persistedOutputPath",
            )?),
        }
    } else {
        None
    };
    let mut blocks = Vec::with_capacity(content.len());
    for block in content {
        let read = user_block_of(block, persisted_output.as_deref())?;
        blocks.push(read);
    }
    assert_eq!(
        blocks.len(),
        content.len(),
        "every block is read or refused"
    );
    Ok(blocks)
}

fn user_block_of(block: &Value, persisted: Option<&str>) -> Result<UserBlock, &'static str> {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => {
            let text = string_at(block, "/text", "/message/content/*/text")?;
            Ok(text_block(&text))
        }
        Some("tool_result") => {
            let result = tool_result_of(block, persisted)?;
            Ok(UserBlock::ToolResult(result))
        }
        Some("image") => Ok(UserBlock::Image),
        Some(_) => Ok(UserBlock::Other),
        None => Err("/message/content/*/type"),
    }
}

fn text_block(text: &str) -> UserBlock {
    UserBlock::Text {
        text: text.to_owned(),
        class: class_of(text),
    }
}

fn tool_result_of(block: &Value, persisted: Option<&str>) -> Result<ToolResult, &'static str> {
    let tool_use_id = name_at(block, "/tool_use_id", "/message/content/*/tool_use_id")?;
    let is_error = flag_at(block, "/is_error", "/message/content/*/is_error")?;
    let parts = match block.get("content") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(text)) => vec![ResultPart::Text(text.clone())],
        Some(Value::Array(content)) => {
            let mut parts = Vec::with_capacity(content.len());
            for part in content {
                let read = result_part_of(part)?;
                parts.push(read);
            }
            parts
        }
        Some(_) => return Err("/message/content/*/content"),
    };
    Ok(ToolResult {
        tool_use_id,
        is_error,
        parts,
        persisted_output: persisted.map(str::to_owned),
    })
}

fn result_part_of(part: &Value) -> Result<ResultPart, &'static str> {
    match part.get("type").and_then(Value::as_str) {
        Some("text") => {
            let text = string_at(part, "/text", "/message/content/*/content/*/text")?;
            Ok(ResultPart::Text(text))
        }
        Some("image") => Ok(ResultPart::Image),
        Some(_) => Ok(ResultPart::Other(part.clone())),
        None => Err("/message/content/*/content/*/type"),
    }
}

fn attachment_of(value: &Value) -> Result<Attachment, &'static str> {
    let envelope = envelope_of(value)?;
    let Some(attachment) = value.get("attachment") else {
        return Err("/attachment");
    };
    let kind = name_at(value, "/attachment/type", "/attachment/type")?;
    Ok(Attachment {
        envelope,
        kind,
        body: attachment.clone(),
    })
}

/// A system record: a compaction boundary, or a kind read here none of.
fn system_of(value: &Value) -> Result<Line, &'static str> {
    let subtype = value.get("subtype").and_then(Value::as_str);
    if subtype == Some(COMPACT_BOUNDARY_SUBTYPE) {
        let envelope = envelope_of(value)?;
        return Ok(Line::Compaction(Compaction { envelope }));
    }
    Ok(Line::Other)
}

/// The string at `pointer`, or `reported` where it is absent or no string.
fn string_at(value: &Value, pointer: &str, reported: &'static str) -> Result<String, &'static str> {
    match value.pointer(pointer) {
        Some(Value::String(text)) => Ok(text.clone()),
        _ => Err(reported),
    }
}

/// The string at `pointer`, refusing an empty one, for an id or a name.
fn name_at(value: &Value, pointer: &str, reported: &'static str) -> Result<String, &'static str> {
    let name = string_at(value, pointer, reported)?;
    if name.is_empty() {
        return Err(reported);
    }
    Ok(name)
}

/// The flag at `pointer`, an absent one false.
fn flag_at(value: &Value, pointer: &str, reported: &'static str) -> Result<bool, &'static str> {
    match value.pointer(pointer) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(reported),
    }
}

/// A count of tokens, an absent field counting none.
fn tokens_at(value: &Value, pointer: &'static str) -> Result<u64, &'static str> {
    match value.pointer(pointer) {
        None | Some(Value::Null) => Ok(0),
        Some(field) => match field.as_u64() {
            Some(count) if count <= TOKENS_PER_FIELD_MAX => Ok(count),
            Some(_) | None => Err(pointer),
        },
    }
}

#[cfg(test)]
mod tests;
