//! What a pairing makes of calls and results arriving in a transcript's order: a result
//! completing its call, a result no call awaits, a call awaited twice, and the bound on
//! calls awaiting. Each case builds calls and results by id and asserts on each outcome.

use jiff::Timestamp;
use serde_json::json;

use super::{Awaited, CALLS_AWAITING_MAX, Call, Completed, Paired, Pairing, calls_of, results_of};
use crate::record::{
    Assistant, AssistantBlock, Envelope, ResultPart, ToolResult, ToolUse, Usage, User, UserBlock,
};

fn envelope() -> Envelope {
    Envelope {
        session: "s1".to_owned(),
        timestamp: "2026-10-31T03:28:13.715Z".parse::<Timestamp>().unwrap(),
        agent_id: None,
    }
}

fn tool_use(id: &str) -> ToolUse {
    ToolUse {
        id: id.to_owned(),
        name: "Bash".to_owned(),
        input: json!({ "command": "ls" }),
    }
}

fn call(id: &str) -> Call {
    Call {
        envelope: envelope(),
        tool_use: tool_use(id),
    }
}

fn result(id: &str) -> ToolResult {
    ToolResult {
        tool_use_id: id.to_owned(),
        is_error: false,
        parts: vec![ResultPart::Text("Cargo.toml".to_owned())],
        persisted_output: None,
    }
}

mod given_records {
    use super::*;

    #[test]
    fn when_an_assistant_records_calls_are_taken_then_each_carries_its_envelope() {
        let assistant = Assistant {
            envelope: envelope(),
            message_id: "msg_01".to_owned(),
            model: "claude-opus-5-5".to_owned(),
            effort: None,
            usage: Usage::default(),
            blocks: vec![
                AssistantBlock::Text("Listing.".to_owned()),
                AssistantBlock::ToolUse(tool_use("toolu_01")),
            ],
        };
        assert_eq!(calls_of(&assistant), vec![call("toolu_01")]);
    }

    #[test]
    fn when_a_user_records_results_are_taken_then_its_text_is_left_out() {
        let user = User {
            envelope: envelope(),
            is_meta: false,
            blocks: vec![UserBlock::Image, UserBlock::ToolResult(result("toolu_01"))],
        };
        assert_eq!(results_of(&user), vec![result("toolu_01")]);
    }
}

mod given_a_call_awaited {
    use super::*;

    #[test]
    fn when_its_result_arrives_then_the_two_are_paired_and_it_awaits_no_longer() {
        let mut pairing = Pairing::new();
        assert_eq!(pairing.await_call(call("toolu_01")), Awaited::Held);
        let expected = Completed::Paired(Paired {
            call: call("toolu_01"),
            result: result("toolu_01"),
        });
        assert_eq!(pairing.complete(result("toolu_01")), expected);
        assert_eq!(pairing.awaiting(), 0);
    }

    #[test]
    fn when_another_calls_result_arrives_then_it_is_handed_back() {
        let mut pairing = Pairing::new();
        assert_eq!(pairing.await_call(call("toolu_01")), Awaited::Held);
        let expected = Completed::Unawaited(result("toolu_02"));
        assert_eq!(pairing.complete(result("toolu_02")), expected);
        assert_eq!(pairing.awaiting(), 1);
    }

    #[test]
    fn when_its_id_is_awaited_again_then_the_second_is_handed_back() {
        let mut pairing = Pairing::new();
        assert_eq!(pairing.await_call(call("toolu_01")), Awaited::Held);
        let expected = Awaited::Duplicate(call("toolu_01"));
        assert_eq!(pairing.await_call(call("toolu_01")), expected);
    }
}

mod given_the_bound_of_calls_awaiting {
    use super::*;

    #[test]
    fn when_one_more_is_awaited_then_it_is_handed_back() {
        let mut pairing = Pairing::new();
        for index in 0..CALLS_AWAITING_MAX {
            let id = format!("toolu_{index}");
            assert_eq!(pairing.await_call(call(&id)), Awaited::Held);
        }
        let expected = Awaited::Full(call("toolu_past"));
        assert_eq!(pairing.await_call(call("toolu_past")), expected);
        assert_eq!(pairing.awaiting(), CALLS_AWAITING_MAX);
    }
}
