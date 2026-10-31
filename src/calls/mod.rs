//! Each tool call paired with its result.
//!
//! A call's result arrives in a later record of the same transcript under the call's
//! id, so a pairing holds each call its reader asks it to await, under a bound, until
//! that result arrives. Everything here is pure.

use std::collections::HashMap;

use crate::record::{Assistant, AssistantBlock, Envelope, ToolResult, ToolUse, User, UserBlock};

/// The most calls one pairing holds awaiting their results at once.
pub const CALLS_AWAITING_MAX: usize = 1_000;

/// A tool call, with the record that made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The session, instant, and subagent of the record that made the call.
    pub envelope: Envelope,
    /// The call.
    pub tool_use: ToolUse,
}

/// A call and what it returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paired {
    /// The call.
    pub call: Call,
    /// What it returned.
    pub result: ToolResult,
}

/// What awaiting a call came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Awaited {
    /// The call awaits its result.
    Held,
    /// A call of the same id already awaits its result, and this one is handed back.
    Duplicate(Call),
    /// [`CALLS_AWAITING_MAX`] calls await their results, and this one is handed back.
    Full(Call),
}

/// What a result came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completed {
    /// The result completes an awaited call.
    Paired(Paired),
    /// No awaited call has the result's id, and the result is handed back.
    Unawaited(ToolResult),
}

/// The calls awaiting their results.
#[derive(Debug, Default)]
pub struct Pairing {
    awaiting: HashMap<String, Call>,
}

/// The tool calls an assistant record makes, each with the record's envelope.
///
/// # Panics
///
/// When it finds more calls than the record holds blocks, which no record can cause.
#[must_use]
pub fn calls_of(assistant: &Assistant) -> Vec<Call> {
    let mut calls = Vec::new();
    for block in &assistant.blocks {
        if let AssistantBlock::ToolUse(tool_use) = block {
            calls.push(Call {
                envelope: assistant.envelope.clone(),
                tool_use: tool_use.clone(),
            });
        }
    }
    assert!(
        calls.len() <= assistant.blocks.len(),
        "each call is one block"
    );
    calls
}

/// The tool results a user record holds.
///
/// # Panics
///
/// When it finds more results than the record holds blocks, which no record can cause.
#[must_use]
pub fn results_of(user: &User) -> Vec<ToolResult> {
    let mut results = Vec::new();
    for block in &user.blocks {
        if let UserBlock::ToolResult(result) = block {
            results.push(result.clone());
        }
    }
    assert!(
        results.len() <= user.blocks.len(),
        "each result is one block"
    );
    results
}

impl Pairing {
    /// A pairing awaiting no call.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Holds `call` until its result arrives.
    ///
    /// # Panics
    ///
    /// When more calls await than [`CALLS_AWAITING_MAX`], which no sequence of calls can
    /// cause.
    pub fn await_call(&mut self, call: Call) -> Awaited {
        assert!(
            self.awaiting.len() <= CALLS_AWAITING_MAX,
            "the calls awaiting stop at their bound"
        );
        if self.awaiting.contains_key(&call.tool_use.id) {
            return Awaited::Duplicate(call);
        }
        if self.awaiting.len() == CALLS_AWAITING_MAX {
            return Awaited::Full(call);
        }
        let id = call.tool_use.id.clone();
        let held = self.awaiting.insert(id, call);
        assert!(held.is_none(), "a call awaits once");
        Awaited::Held
    }

    /// Pairs `result` with the call it returns from, which then awaits no longer.
    ///
    /// # Panics
    ///
    /// When the call held under the result's id is another's, which no sequence of
    /// calls can cause.
    pub fn complete(&mut self, result: ToolResult) -> Completed {
        let Some(call) = self.awaiting.remove(&result.tool_use_id) else {
            return Completed::Unawaited(result);
        };
        assert_eq!(
            call.tool_use.id, result.tool_use_id,
            "a result pairs with its call"
        );
        Completed::Paired(Paired { call, result })
    }

    /// The calls awaiting their results.
    ///
    /// # Panics
    ///
    /// When more calls await than [`CALLS_AWAITING_MAX`], which no sequence of calls can
    /// cause.
    #[must_use]
    pub fn awaiting(&self) -> usize {
        let count = self.awaiting.len();
        assert!(
            count <= CALLS_AWAITING_MAX,
            "the calls awaiting stop at their bound"
        );
        count
    }
}

#[cfg(test)]
mod tests;
