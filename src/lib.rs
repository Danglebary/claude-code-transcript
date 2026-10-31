//! Reads the transcripts Claude Code keeps of its sessions: `store` finds them and says
//! whose each one is, `lines` reads one a line at a time under bounds, `record` reads a
//! line into what it records, and `calls` pairs each tool call with its result.
//!
//! `record` and `calls` do no IO, which is what lets a line be replayed against a
//! fixture; `store` and `lines` hold every read of disk. Claude Code documents no schema
//! for these files, so a line outside the shape read here comes back as an outcome the
//! caller branches on, never as an error.

pub mod calls;
pub mod lines;
pub mod record;
pub mod store;
