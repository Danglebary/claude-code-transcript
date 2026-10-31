//! A transcript read a line at a time under bounds.
//!
//! Each line comes with the byte offset it starts at. A line too long to read or holding
//! no UTF-8 text comes back as an outcome and the read goes on past it, since one bad line
//! says nothing of the rest.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

/// The longest line read, its line end left out; a longer one is passed over.
pub const LINE_BYTES_MAX: usize = 64 * 1024 * 1024;

/// The most lines one transcript is read for.
pub const LINES_PER_TRANSCRIPT_MAX: u64 = 10_000_000;

/// The buffer a transcript file is read through.
const BUFFER_BYTES: usize = 1024 * 1024;

const LINE_END: u8 = b'\n';

/// What reading the next line came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// A line, its line end left off.
    Text {
        /// The byte the line starts at.
        offset: u64,
        /// The line's text.
        text: String,
    },
    /// A line longer than [`LINE_BYTES_MAX`], passed over through its end.
    TooLong {
        /// The byte the line starts at.
        offset: u64,
    },
    /// A line whose bytes are no UTF-8 text.
    NotText {
        /// The byte the line starts at.
        offset: u64,
    },
    /// The transcript ended.
    End,
    /// The transcript holds more than [`LINES_PER_TRANSCRIPT_MAX`] lines, and nothing past
    /// them is read.
    PastTheBound,
}

/// A transcript's lines, read in order.
#[derive(Debug)]
pub struct Lines<R> {
    reader: R,
    offset: u64,
    count: u64,
}

impl Lines<BufReader<File>> {
    /// The lines of the transcript at `path`.
    ///
    /// # Errors
    ///
    /// The error opening the file.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(BUFFER_BYTES, file);
        Ok(Self::new(reader))
    }
}

impl<R: BufRead> Lines<R> {
    /// The lines `reader` holds, from its current position.
    pub const fn new(reader: R) -> Self {
        Self {
            reader,
            offset: 0,
            count: 0,
        }
    }

    /// The next line, or why there is none.
    ///
    /// # Errors
    ///
    /// The error reading the transcript, after which nothing more is read from it.
    ///
    /// # Panics
    ///
    /// When the read passes its own bounds, which no transcript can cause.
    pub fn next_piece(&mut self) -> io::Result<Piece> {
        assert!(
            self.count <= LINES_PER_TRANSCRIPT_MAX,
            "the read stops at its bound"
        );
        if self.count == LINES_PER_TRANSCRIPT_MAX {
            return Ok(Piece::PastTheBound);
        }
        let offset = self.offset;
        let Some(take_bytes) = u64::try_from(LINE_BYTES_MAX)
            .ok()
            .and_then(|bytes| bytes.checked_add(1))
        else {
            unreachable!("the line bound and its line end fit a u64")
        };
        let mut bytes = Vec::new();
        let read = self
            .reader
            .by_ref()
            .take(take_bytes)
            .read_until(LINE_END, &mut bytes)?;
        if read == 0 {
            return Ok(Piece::End);
        }
        let ended = bytes.last() == Some(&LINE_END);
        let too_long = !ended && read > LINE_BYTES_MAX;
        let skipped = if too_long {
            self.reader.skip_until(LINE_END)?
        } else {
            0
        };
        self.advance(read, skipped);
        assert!(self.offset > offset, "a line read moves the offset on");
        if too_long {
            return Ok(Piece::TooLong { offset });
        }
        if ended {
            bytes.pop();
        }
        assert!(
            bytes.len() <= LINE_BYTES_MAX,
            "a line kept is within its bound"
        );
        match String::from_utf8(bytes) {
            Ok(text) => Ok(Piece::Text { offset, text }),
            Err(_) => Ok(Piece::NotText { offset }),
        }
    }

    /// Moves the offset past a line read in `read` bytes and skipped through `skipped` more,
    /// and counts the line.
    fn advance(&mut self, read: usize, skipped: usize) {
        let Some(length) = read.checked_add(skipped) else {
            unreachable!("a line's bytes fit a usize")
        };
        let Ok(length) = u64::try_from(length) else {
            unreachable!("a line's length fits a u64")
        };
        let Some(offset) = self.offset.checked_add(length) else {
            unreachable!("a transcript's bytes fit a u64")
        };
        let Some(count) = self.count.checked_add(1) else {
            unreachable!("the lines read stop at their bound")
        };
        self.offset = offset;
        self.count = count;
    }
}

#[cfg(test)]
mod tests;
