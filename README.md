# claude-code-transcript

Reads the transcripts Claude Code keeps under `~/.claude/projects`: which file is whose session and thread, each file's lines under bounds, each line as a typed record, and each tool call paired with its result.

```rust
use claude_code_transcript::lines::{Lines, Piece};
use claude_code_transcript::record::{Line, read_line};
use claude_code_transcript::store::{folders, projects_folder, transcripts_in};

let projects = projects_folder(home);
for folder in folders(&projects)? {
    for found in transcripts_in(&folder)? {
        let mut lines = Lines::open(&found.path)?;
        loop {
            match lines.next_piece()? {
                Piece::Text { text, .. } => {
                    if let Line::Assistant(record) = read_line(&text) {
                        // record.usage, record.blocks, …
                    }
                }
                Piece::TooLong { .. } | Piece::NotText { .. } => {}
                Piece::End | Piece::PastTheBound => break,
            }
        }
    }
}
```

Claude Code documents no schema for these files. The shapes read here were read off real transcripts, and a line outside them comes back as an outcome the caller branches on: `Line::Other` for a kind of record not read here, and `Line::Malformed` with the JSON pointer of the field that failed its check.

## License

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
