# claude-code-transcript

A Rust library that reads the transcripts Claude Code keeps of its sessions, so the Rust tools reading them (meter, pirr, and excerpt's measurements) follow each change to the format once rather than each on its own.

The engineering style guide at `~/.claude/CLAUDE.md` governs. This file states only what is specific to this repository.

## The format it reads

Claude Code documents no schema for its transcripts. Every shape this crate reads was read off this machine's store, and each module's header states the shape it reads with the date it was last checked; when a module's reading changes, recheck the shape against current transcripts and refresh that date. A line outside the shape comes back as `Line::Other` or `Line::Malformed` with the pointer of the field that failed, never as an error, so each tool decides how much fails.

The store stands at `~/.claude/projects`. A folder per path a session ran in holds each session's main thread as `<session>.jsonl` and its subagents' beneath `<session>/subagents/`, each beside an `agent-<id>.meta.json` naming its type. Each line is one JSON object, written without spaces, its kind under `type`; `record` reads the `assistant`, `user`, and `attachment` kinds and the `system` record marking a compaction boundary, and passes every other over.

## Shape

`src/lib.rs` declares four modules. `record` reads one line into a typed `Line` and `calls` pairs each tool call with its result; both do no IO, and their tests replay fixture lines. `store` lists the store's folders and each folder's transcripts, and `lines` reads one transcript a line at a time; both read disk under the bounds their constants name, and their tests read real files in a scratch directory. A tool call's input stays JSON, since each tool reads fields of its own.

The lint table in `Cargo.toml` denies `unwrap`, `expect`, `panic!`, indexing, slicing, string slicing, `as` casts, and unchecked arithmetic. A programmer error crashes through `assert!` or `unreachable!` with a message; arithmetic goes through `checked_*` with a `let ... else { unreachable!(..) }`.

## Running things

`just check` is the gate and must stay green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`. The recipes enter `nix develop` themselves unless `CLAUDE_CODE_TRANSCRIPT_SHELL` shows they already stand in its shell. The toolchain is pinned once, in `rust-toolchain.toml`, which both the flake and CI's rustup read.

## Tests

A test is a function named `when_<operation>_then_<result>` in a module named `given_<input>`, and each test file opens with its goal and method as a `//!` comment. Each case asserts on a returned value; an invalid record is a valid one with the field the case breaks overridden.

## Taking the crate

A tool takes the crate as a git dependency pinned to a release tag:

```toml
claude-code-transcript = { git = "https://github.com/Danglebary/claude-code-transcript", tag = "v0.2.0" }
```

A tool built by nixpkgs' `buildRustPackage` names the dependency's hash in its `cargoLock`, keyed by the crate's name and version, and refreshes it whenever the pin moves; the first build after a move fails and prints the hash to write:

```nix
cargoLock = {
  lockFile = ./Cargo.lock;
  outputHashes."claude-code-transcript-0.2.0" = "sha256-…";
};
```

## Releases

Raise `version` in `Cargo.toml` and merge to `main`. `.github/workflows/release.yml` publishes a release tagged `v<version>` when the version has no release yet, and publishes nothing otherwise.
