//! What `read_line` makes of each kind of record, `read_usage` of a usage object, and
//! `class_of` of each kind of text. Each case builds a line or a usage object from a valid
//! one of its kind, written as Claude Code writes it, with one field overridden where the
//! case breaks it, and asserts on the whole outcome.

use jiff::Timestamp;
use serde_json::{Value, json};

use super::{
    Assistant, AssistantBlock, Attachment, Commit, CommitKind, CommitReport, Compaction, Envelope,
    Line, ResultPart, TextClass, ToolResult, ToolUse, Usage, UsageField, User, UserBlock,
    WHOLE_LINE, class_of, read_line, read_usage,
};

const SESSION: &str = "9d5fe9d0-d389-44a2-a3ba-a29db8bbb544";
const STAMP: &str = "2026-10-31T03:28:13.715Z";
const DIRECTORY: &str = "/home/admin/projects/pirr";
const BRANCH: &str = "main";
const RELEASE: &str = "2.1.292";
const SHA: &str = "1a2b3c4";

fn envelope() -> Envelope {
    Envelope {
        session: SESSION.to_owned(),
        timestamp: STAMP.parse::<Timestamp>().unwrap(),
        agent_id: None,
        cwd: Some(DIRECTORY.to_owned()),
        git_branch: Some(BRANCH.to_owned()),
        version: Some(RELEASE.to_owned()),
    }
}

fn assistant(content: &Value) -> Value {
    json!({
        "type": "assistant",
        "sessionId": SESSION,
        "timestamp": STAMP,
        "cwd": DIRECTORY,
        "gitBranch": BRANCH,
        "version": RELEASE,
        "effort": "high",
        "message": {
            "id": "msg_01",
            "model": "claude-opus-5-5",
            "content": content,
            "usage": {
                "input_tokens": 3,
                "cache_read_input_tokens": 40,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 0,
                    "ephemeral_1h_input_tokens": 7
                },
                "output_tokens": 12
            }
        }
    })
}

fn user(content: &Value) -> Value {
    json!({
        "type": "user",
        "sessionId": SESSION,
        "timestamp": STAMP,
        "cwd": DIRECTORY,
        "gitBranch": BRANCH,
        "version": RELEASE,
        "message": { "role": "user", "content": content }
    })
}

fn read(record: &Value) -> Line {
    read_line(&record.to_string())
}

fn set(record: &mut Value, pointer: &str, field: Value) {
    *record.pointer_mut(pointer).unwrap() = field;
}

/// Removes the field `pointer` names from the object holding it.
fn remove(record: &mut Value, pointer: &str) {
    let (holder, key) = pointer.rsplit_once('/').unwrap();
    record
        .pointer_mut(holder)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove(key)
        .unwrap();
}

/// A usage object as Claude Code 2.1.300 writes it in a print-mode result, read
/// 2026-11-26, each count distinct so a count read from the wrong field shows.
fn usage() -> Value {
    json!({
        "input_tokens": 2,
        "cache_creation_input_tokens": 48_109,
        "cache_read_input_tokens": 10_698,
        "output_tokens": 40,
        "output_tokens_details": { "thinking_tokens": 9 },
        "server_tool_use": { "web_search_requests": 0, "web_fetch_requests": 0 },
        "service_tier": "standard",
        "cache_creation": {
            "ephemeral_1h_input_tokens": 48_078,
            "ephemeral_5m_input_tokens": 31
        },
        "inference_geo": "global",
        "speed": "standard",
        "fallback_credit": null
    })
}

/// The counts [`usage`] holds.
const USAGE_READ: Usage = Usage {
    input: 2,
    cache_read: 10_698,
    cache_write_5m: 31,
    cache_write_1h: 48_078,
    output: 40,
    thinking: 9,
};

mod given_an_assistant_record {
    use super::*;

    #[test]
    fn when_read_then_every_field_and_block_is_kept() {
        let content = json!([
            { "type": "tool_use", "id": "toolu_01", "name": "Bash", "input": { "command": "ls" } }
        ]);
        let expected = Line::Assistant(Assistant {
            envelope: envelope(),
            message_id: "msg_01".to_owned(),
            model: "claude-opus-5-5".to_owned(),
            effort: Some("high".to_owned()),
            usage: Usage {
                input: 3,
                cache_read: 40,
                cache_write_5m: 0,
                cache_write_1h: 7,
                output: 12,
                thinking: 0,
            },
            blocks: vec![AssistantBlock::ToolUse(ToolUse {
                id: "toolu_01".to_owned(),
                name: "Bash".to_owned(),
                input: json!({ "command": "ls" }),
            })],
        });
        assert_eq!(read(&assistant(&content)), expected);
    }

    #[test]
    fn when_read_then_each_block_kind_is_named() {
        let content = json!([
            { "type": "text", "text": "Done." },
            { "type": "thinking", "thinking": "…", "signature": "…" },
            { "type": "server_tool_use", "id": "srvtoolu_01" }
        ]);
        let Line::Assistant(read) = read(&assistant(&content)) else {
            panic!("an assistant record reads as one");
        };
        let expected = vec![
            AssistantBlock::Text("Done.".to_owned()),
            AssistantBlock::Thinking,
            AssistantBlock::Other,
        ];
        assert_eq!(read.blocks, expected);
    }

    #[test]
    fn when_read_in_a_subagents_transcript_then_its_agent_is_kept() {
        let mut record = assistant(&json!([]));
        record["agentId"] = json!("a9491ebeefe851f49");
        let Line::Assistant(read) = read(&record) else {
            panic!("an assistant record reads as one");
        };
        assert_eq!(read.envelope.agent_id.as_deref(), Some("a9491ebeefe851f49"));
    }

    #[test]
    fn when_it_names_no_directory_then_none_is_kept() {
        for absent in [None, Some(json!(null))] {
            let mut record = assistant(&json!([]));
            match absent {
                None => {
                    record.as_object_mut().unwrap().remove("cwd");
                }
                Some(field) => set(&mut record, "/cwd", field),
            }
            let Line::Assistant(read) = read(&record) else {
                panic!("an assistant record reads as one");
            };
            assert_eq!(read.envelope.cwd, None);
        }
    }

    #[test]
    fn when_it_names_no_branch_then_none_is_kept() {
        for absent in [None, Some(json!(null))] {
            let mut record = assistant(&json!([]));
            match absent {
                None => {
                    record.as_object_mut().unwrap().remove("gitBranch");
                }
                Some(field) => set(&mut record, "/gitBranch", field),
            }
            let Line::Assistant(read) = read(&record) else {
                panic!("an assistant record reads as one");
            };
            assert_eq!(read.envelope.git_branch, None);
        }
    }

    #[test]
    fn when_it_names_no_release_then_none_is_kept() {
        for absent in [None, Some(json!(null))] {
            let mut record = assistant(&json!([]));
            match absent {
                None => {
                    record.as_object_mut().unwrap().remove("version");
                }
                Some(field) => set(&mut record, "/version", field),
            }
            let Line::Assistant(read) = read(&record) else {
                panic!("an assistant record reads as one");
            };
            assert_eq!(read.envelope.version, None);
        }
    }

    #[test]
    fn when_a_field_fails_its_check_then_the_pointer_names_it() {
        let cases: [(&str, Value, &str); 11] = [
            ("/sessionId", json!(""), "/sessionId"),
            ("/timestamp", json!("yesterday"), "/timestamp"),
            ("/cwd", json!(""), "/cwd"),
            ("/gitBranch", json!(7), "/gitBranch"),
            ("/version", json!(2.1), "/version"),
            ("/version", json!(""), "/version"),
            ("/message/id", json!(null), "/message/id"),
            ("/message/model", json!(7), "/message/model"),
            ("/message/usage", json!(null), "/message/usage"),
            (
                "/message/usage/output_tokens",
                json!(-1),
                "/message/usage/output_tokens",
            ),
            ("/message/content", json!("text"), "/message/content"),
        ];
        for (pointer, field, named) in cases {
            let mut record = assistant(&json!([]));
            set(&mut record, pointer, field);
            assert_eq!(read(&record), Line::Malformed(named), "{pointer}");
        }
    }

    #[test]
    fn when_a_count_passes_its_bound_then_it_is_malformed() {
        let mut record = assistant(&json!([]));
        set(
            &mut record,
            "/message/usage/input_tokens",
            json!(10_000_001),
        );
        let expected = Line::Malformed("/message/usage/input_tokens");
        assert_eq!(read(&record), expected);
    }

    #[test]
    fn when_its_input_or_output_count_is_absent_or_null_then_the_pointer_names_it() {
        let pointers = [
            "/message/usage/input_tokens",
            "/message/usage/output_tokens",
        ];
        for pointer in pointers {
            let mut absent = assistant(&json!([]));
            remove(&mut absent, pointer);
            assert_eq!(read(&absent), Line::Malformed(pointer), "{pointer} absent");
            let mut null = assistant(&json!([]));
            set(&mut null, pointer, json!(null));
            assert_eq!(read(&null), Line::Malformed(pointer), "{pointer} null");
        }
    }

    #[test]
    fn when_its_cache_counts_are_absent_then_it_reads_counting_none_cached() {
        let mut record = assistant(&json!([]));
        remove(&mut record, "/message/usage/cache_read_input_tokens");
        remove(&mut record, "/message/usage/cache_creation");
        let Line::Assistant(read) = read(&record) else {
            panic!("an assistant record reads as one");
        };
        let expected = Usage {
            input: 3,
            output: 12,
            ..Usage::default()
        };
        assert_eq!(read.usage, expected);
    }

    #[test]
    fn when_a_usage_count_fails_its_check_then_the_pointer_names_it_within_the_record() {
        let pointers = [
            "/message/usage/input_tokens",
            "/message/usage/cache_read_input_tokens",
            "/message/usage/cache_creation/ephemeral_5m_input_tokens",
            "/message/usage/cache_creation/ephemeral_1h_input_tokens",
            "/message/usage/output_tokens",
            "/message/usage/output_tokens_details/thinking_tokens",
        ];
        for pointer in pointers {
            let mut record = assistant(&json!([]));
            set(&mut record, "/message/usage", usage());
            set(&mut record, pointer, json!("many"));
            assert_eq!(read(&record), Line::Malformed(pointer), "{pointer}");
        }
    }

    #[test]
    fn when_a_tool_call_names_no_id_then_the_block_pointer_names_it() {
        let content = json!([{ "type": "tool_use", "name": "Bash", "input": {} }]);
        let expected = Line::Malformed("/message/content/*/id");
        assert_eq!(read(&assistant(&content)), expected);
    }
}

mod given_a_usage_object {
    use super::*;

    #[test]
    fn when_read_then_every_count_is_kept() {
        assert_eq!(read_usage(&usage()), Ok(USAGE_READ));
    }

    #[test]
    fn when_its_input_or_output_count_is_absent_or_null_then_the_field_is_named() {
        let cases = [
            ("/input_tokens", UsageField::Input),
            ("/output_tokens", UsageField::Output),
        ];
        for (pointer, field) in cases {
            let mut absent = usage();
            remove(&mut absent, pointer);
            assert_eq!(read_usage(&absent), Err(field), "{pointer} absent");
            let mut null = usage();
            set(&mut null, pointer, json!(null));
            assert_eq!(read_usage(&null), Err(field), "{pointer} null");
        }
    }

    #[test]
    fn when_a_cache_or_thinking_count_is_absent_or_null_then_it_counts_none() {
        let cases = [
            (
                "/cache_read_input_tokens",
                Usage {
                    cache_read: 0,
                    ..USAGE_READ
                },
            ),
            (
                "/cache_creation/ephemeral_5m_input_tokens",
                Usage {
                    cache_write_5m: 0,
                    ..USAGE_READ
                },
            ),
            (
                "/cache_creation/ephemeral_1h_input_tokens",
                Usage {
                    cache_write_1h: 0,
                    ..USAGE_READ
                },
            ),
            (
                "/output_tokens_details/thinking_tokens",
                Usage {
                    thinking: 0,
                    ..USAGE_READ
                },
            ),
        ];
        for (pointer, expected) in cases {
            let mut absent = usage();
            remove(&mut absent, pointer);
            assert_eq!(read_usage(&absent), Ok(expected), "{pointer} absent");
            let mut null = usage();
            set(&mut null, pointer, json!(null));
            assert_eq!(read_usage(&null), Ok(expected), "{pointer} null");
        }
    }

    #[test]
    fn when_a_count_is_no_whole_number_within_its_bound_then_the_field_is_named() {
        let cases = [
            ("/input_tokens", UsageField::Input),
            ("/cache_read_input_tokens", UsageField::CacheRead),
            (
                "/cache_creation/ephemeral_5m_input_tokens",
                UsageField::CacheWrite5m,
            ),
            (
                "/cache_creation/ephemeral_1h_input_tokens",
                UsageField::CacheWrite1h,
            ),
            ("/output_tokens", UsageField::Output),
            (
                "/output_tokens_details/thinking_tokens",
                UsageField::Thinking,
            ),
        ];
        let broken = [json!(-1), json!(2.5), json!("many"), json!(10_000_001)];
        for (pointer, field) in cases {
            for count in &broken {
                let mut read = usage();
                set(&mut read, pointer, count.clone());
                assert_eq!(read_usage(&read), Err(field), "{pointer} as {count}");
            }
        }
    }

    #[test]
    fn when_it_is_no_object_then_the_whole_is_named() {
        for whole in [json!(null), json!([]), json!(7)] {
            assert_eq!(read_usage(&whole), Err(UsageField::Whole), "{whole}");
        }
    }

    #[test]
    fn when_a_field_is_named_then_its_pointer_is_where_the_object_holds_it() {
        let cases = [
            (UsageField::Whole, ""),
            (UsageField::Input, "/input_tokens"),
            (UsageField::CacheRead, "/cache_read_input_tokens"),
            (
                UsageField::CacheWrite5m,
                "/cache_creation/ephemeral_5m_input_tokens",
            ),
            (
                UsageField::CacheWrite1h,
                "/cache_creation/ephemeral_1h_input_tokens",
            ),
            (UsageField::Output, "/output_tokens"),
            (
                UsageField::Thinking,
                "/output_tokens_details/thinking_tokens",
            ),
        ];
        for (field, pointer) in cases {
            assert_eq!(field.pointer(), pointer, "{field:?}");
        }
    }
}

mod given_a_user_record {
    use super::*;

    #[test]
    fn when_its_content_is_a_string_then_it_is_one_text_block() {
        let expected = Line::User(User {
            envelope: envelope(),
            is_meta: false,
            blocks: vec![UserBlock::Text {
                text: "carry on".to_owned(),
                class: TextClass::Typed,
            }],
        });
        assert_eq!(read(&user(&json!("carry on"))), expected);
    }

    #[test]
    fn when_claude_code_wrote_it_then_it_is_marked_meta() {
        let mut record = user(&json!("Base directory for this skill: /skills/pickup"));
        record["isMeta"] = json!(true);
        let Line::User(read) = read(&record) else {
            panic!("a user record reads as one");
        };
        assert!(read.is_meta);
    }

    #[test]
    fn when_it_holds_a_tool_result_then_the_result_and_its_parts_are_kept() {
        let content = json!([{
            "type": "tool_result",
            "tool_use_id": "toolu_01",
            "is_error": true,
            "content": [
                { "type": "text", "text": "refused" },
                { "type": "image", "source": { "data": "…" } },
                { "type": "tool_reference", "tool_name": "Read" }
            ]
        }]);
        let expected = vec![UserBlock::ToolResult(ToolResult {
            tool_use_id: "toolu_01".to_owned(),
            is_error: true,
            parts: vec![
                ResultPart::Text("refused".to_owned()),
                ResultPart::Image,
                ResultPart::Other(json!({ "type": "tool_reference", "tool_name": "Read" })),
            ],
            persisted_output: None,
            commit: CommitReport::Unreported,
        })];
        let Line::User(read) = read(&user(&content)) else {
            panic!("a user record reads as one");
        };
        assert_eq!(read.blocks, expected);
    }

    #[test]
    fn when_one_result_was_saved_then_the_saved_path_is_its() {
        let content = json!([{ "type": "tool_result", "tool_use_id": "toolu_01", "content": "…" }]);
        let mut record = user(&content);
        record["toolUseResult"] = json!({ "persistedOutputPath": "/saved/toolu_01.txt" });
        let Line::User(read) = read(&record) else {
            panic!("a user record reads as one");
        };
        let [UserBlock::ToolResult(result)] = read.blocks.as_slice() else {
            panic!("the record holds one result");
        };
        assert_eq!(
            result.persisted_output.as_deref(),
            Some("/saved/toolu_01.txt")
        );
    }

    #[test]
    fn when_two_results_share_a_saved_path_then_neither_takes_it() {
        let content = json!([
            { "type": "tool_result", "tool_use_id": "toolu_01", "content": "…" },
            { "type": "tool_result", "tool_use_id": "toolu_02", "content": "…" }
        ]);
        let mut record = user(&content);
        record["toolUseResult"] = json!({ "persistedOutputPath": "/saved/toolu_01.txt" });
        let Line::User(read) = read(&record) else {
            panic!("a user record reads as one");
        };
        for block in read.blocks {
            let UserBlock::ToolResult(result) = block else {
                panic!("the record holds results alone");
            };
            assert_eq!(result.persisted_output, None);
        }
    }

    #[test]
    fn when_a_field_fails_its_check_then_the_pointer_names_it() {
        let result =
            json!([{ "type": "tool_result", "tool_use_id": "toolu_01", "is_error": false }]);
        let cases: [(&str, Value, &str); 4] = [
            ("/isMeta", json!("yes"), "/isMeta"),
            ("/message/content", json!(null), "/message/content"),
            (
                "/message/content/0/tool_use_id",
                json!(""),
                "/message/content/*/tool_use_id",
            ),
            (
                "/message/content/0/is_error",
                json!(1),
                "/message/content/*/is_error",
            ),
        ];
        for (pointer, field, named) in cases {
            let mut record = user(&result);
            if pointer == "/isMeta" {
                record["isMeta"] = field;
            } else {
                set(&mut record, pointer, field);
            }
            assert_eq!(read(&record), Line::Malformed(named), "{pointer}");
        }
    }
}

mod given_a_user_record_reporting_a_commit {
    use super::*;

    const COMMIT_BRANCH: &str = "feature";
    const OUTPUT: &str = "[feature 1a2b3c4] Add a file";

    fn bash_result(tool_use_result: Value) -> Value {
        let content =
            json!([{ "type": "tool_result", "tool_use_id": "toolu_01", "content": OUTPUT }]);
        let mut record = user(&content);
        record["toolUseResult"] = tool_use_result;
        record
    }

    fn reporting(git_operation: &Value) -> Value {
        json!({ "stdout": OUTPUT, "stderr": "", "gitOperation": git_operation })
    }

    fn commit_operation(kind: &str) -> Value {
        json!({ "commit": { "sha": SHA, "kind": kind, "branch": COMMIT_BRANCH } })
    }

    fn only_result(record: &Value) -> ToolResult {
        let Line::User(read) = read(record) else {
            panic!("a user record reads as one");
        };
        let [UserBlock::ToolResult(result)] = read.blocks.as_slice() else {
            panic!("the record holds one result");
        };
        result.clone()
    }

    fn results(record: &Value) -> Vec<ToolResult> {
        let Line::User(read) = read(record) else {
            panic!("a user record reads as one");
        };
        let mut results = Vec::new();
        for block in read.blocks {
            let UserBlock::ToolResult(result) = block else {
                panic!("the record holds results alone");
            };
            results.push(result);
        }
        results
    }

    fn two_results(tool_use_result: Value) -> Value {
        let content = json!([
            { "type": "tool_result", "tool_use_id": "toolu_01", "content": OUTPUT },
            { "type": "tool_result", "tool_use_id": "toolu_02", "content": OUTPUT }
        ]);
        let mut record = user(&content);
        record["toolUseResult"] = tool_use_result;
        record
    }

    #[test]
    fn when_one_result_reports_a_commit_then_its_commit_is_read() {
        let cases: [(&str, CommitKind); 2] = [
            ("committed", CommitKind::Committed),
            ("amended", CommitKind::Amended),
        ];
        for (kind, read_kind) in cases {
            let operation = commit_operation(kind);
            let tool_use_result = reporting(&operation);
            let record = bash_result(tool_use_result);
            let expected = CommitReport::Commit(Commit {
                sha: SHA.to_owned(),
                kind: read_kind,
                branch: COMMIT_BRANCH.to_owned(),
            });
            assert_eq!(only_result(&record).commit, expected, "{kind}");
        }
    }

    #[test]
    fn when_it_reports_no_commit_then_none_is_reported() {
        let cases: [Value; 6] = [
            json!({ "stdout": OUTPUT, "stderr": "" }),
            json!("Error: Exit code 1"),
            reporting(&json!(null)),
            reporting(&json!({ "push": { "branch": COMMIT_BRANCH } })),
            reporting(&json!({
                "pr": { "number": 7, "action": "created", "url": "https://example.com/pull/7" },
                "push": { "branch": COMMIT_BRANCH }
            })),
            reporting(
                &json!({ "commit": null, "branch": { "ref": COMMIT_BRANCH, "action": "created" } }),
            ),
        ];
        for tool_use_result in cases {
            let record = bash_result(tool_use_result.clone());
            let read = only_result(&record);
            assert_eq!(read.commit, CommitReport::Unreported, "{tool_use_result}");
        }
    }

    #[test]
    fn when_its_commit_is_in_a_shape_not_read_then_the_pointer_names_it_and_the_result_is_kept() {
        let operation = "/toolUseResult/gitOperation";
        let commit = "/toolUseResult/gitOperation/commit";
        let sha = "/toolUseResult/gitOperation/commit/sha";
        let kind = "/toolUseResult/gitOperation/commit/kind";
        let branch = "/toolUseResult/gitOperation/commit/branch";
        let cases: [(&str, Value, &str); 11] = [
            (operation, json!(["commit"]), operation),
            (commit, json!(SHA), commit),
            (sha, json!(7), sha),
            (sha, json!(""), sha),
            (sha, json!("1A2B3C4"), sha),
            (sha, json!("a".repeat(65)), sha),
            (kind, json!("reverted"), kind),
            (kind, json!(null), kind),
            (branch, json!(""), branch),
            (branch, json!(null), branch),
            (branch, json!([COMMIT_BRANCH]), branch),
        ];
        for (pointer, field, named) in cases {
            let operation = commit_operation("committed");
            let tool_use_result = reporting(&operation);
            let mut record = bash_result(tool_use_result);
            set(&mut record, pointer, field);
            let expected = ToolResult {
                tool_use_id: "toolu_01".to_owned(),
                is_error: false,
                parts: vec![ResultPart::Text(OUTPUT.to_owned())],
                persisted_output: None,
                commit: CommitReport::Unread(named),
            };
            assert_eq!(only_result(&record), expected, "{pointer}");
        }
    }

    #[test]
    fn when_two_results_share_a_commit_then_neither_takes_it() {
        let operation = commit_operation("committed");
        let tool_use_result = reporting(&operation);
        let record = two_results(tool_use_result);
        let read = results(&record);
        assert_eq!(read.len(), 2);
        for result in read {
            assert_eq!(result.commit, CommitReport::Unattributed);
        }
    }

    #[test]
    fn when_two_results_report_no_commit_then_neither_has_one() {
        let operation = json!({ "push": { "branch": COMMIT_BRANCH } });
        let tool_use_result = reporting(&operation);
        let record = two_results(tool_use_result);
        let read = results(&record);
        assert_eq!(read.len(), 2);
        for result in read {
            assert_eq!(result.commit, CommitReport::Unreported);
        }
    }
}

mod given_an_attachment_record {
    use super::*;

    #[test]
    fn when_read_then_its_kind_and_body_are_kept() {
        let body = json!({ "type": "hook_success", "content": "ok" });
        let record = json!({
            "type": "attachment",
            "sessionId": SESSION,
            "timestamp": STAMP,
            "cwd": DIRECTORY,
            "gitBranch": BRANCH,
            "version": RELEASE,
            "attachment": body
        });
        let expected = Line::Attachment(Attachment {
            envelope: envelope(),
            kind: "hook_success".to_owned(),
            body,
        });
        assert_eq!(read(&record), expected);
    }

    #[test]
    fn when_it_names_no_kind_then_the_pointer_names_it() {
        let record = json!({
            "type": "attachment",
            "sessionId": SESSION,
            "timestamp": STAMP,
            "cwd": DIRECTORY,
            "gitBranch": BRANCH,
            "version": RELEASE,
            "attachment": { "content": "ok" }
        });
        assert_eq!(read(&record), Line::Malformed("/attachment/type"));
    }
}

mod given_a_system_record {
    use super::*;

    fn compaction() -> Value {
        json!({
            "type": "system",
            "subtype": "compact_boundary",
            "sessionId": SESSION,
            "timestamp": STAMP,
            "cwd": DIRECTORY,
            "gitBranch": BRANCH,
            "version": RELEASE,
            "level": "info",
            "compactMetadata": { "trigger": "auto", "preTokens": 967_763, "postTokens": 8_614 }
        })
    }

    #[test]
    fn when_it_marks_a_compaction_boundary_then_its_envelope_is_kept() {
        let expected = Line::Compaction(Compaction {
            envelope: envelope(),
        });
        assert_eq!(read(&compaction()), expected);
    }

    #[test]
    fn when_it_marks_anything_else_then_it_is_other() {
        for subtype in ["turn_duration", "local_command", "away_summary"] {
            let mut record = compaction();
            set(&mut record, "/subtype", json!(subtype));
            assert_eq!(read(&record), Line::Other, "{subtype}");
        }
    }

    #[test]
    fn when_its_timestamp_fails_its_check_then_the_pointer_names_it() {
        let mut record = compaction();
        set(&mut record, "/timestamp", json!("yesterday"));
        assert_eq!(read(&record), Line::Malformed("/timestamp"));
    }
}

mod given_any_other_line {
    use super::*;

    #[test]
    fn when_read_then_it_is_other_or_malformed() {
        let cases: [(&str, Line); 4] = [
            (
                r#"{"type":"queue-operation","operation":"enqueue"}"#,
                Line::Other,
            ),
            (r#"{"type":"cost-state","totalCostUSD":1.5}"#, Line::Other),
            (
                r#"{"type":"user","sessionId":"#,
                Line::Malformed(WHOLE_LINE),
            ),
            (r#"["type":"user"]"#, Line::Malformed(WHOLE_LINE)),
        ];
        for (text, expected) in cases {
            assert_eq!(read_line(text), expected, "{text}");
        }
    }

    #[test]
    #[should_panic(expected = "a line holds no line end")]
    fn when_it_keeps_its_line_end_then_the_read_crashes() {
        let _ = read_line("{\"type\":\"user\"}\n");
    }
}

mod given_text_in_the_users_turn {
    use super::*;

    #[test]
    fn when_classed_then_its_opening_names_who_put_it_there() {
        let cases: [(&str, TextClass); 7] = [
            ("fix the test", TextClass::Typed),
            ("< is less than", TextClass::Typed),
            (
                "Base directory for this skill: /skills/x",
                TextClass::SkillBody,
            ),
            (
                "<command-message>pickup</command-message>\n<command-name>/pickup</command-name>",
                TextClass::SlashCommand,
            ),
            ("[Request interrupted by user]", TextClass::Interruption),
            (
                "<task-notification>\n<task-id>1</task-id>",
                TextClass::Tagged("task-notification".to_owned()),
            ),
            (
                "<bash-input>ls</bash-input>",
                TextClass::Tagged("bash-input".to_owned()),
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(class_of(text), expected, "{text}");
        }
    }
}
