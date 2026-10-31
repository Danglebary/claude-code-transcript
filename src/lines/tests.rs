//! What `Lines` reads off a transcript's bytes: each line with its offset, a line too long
//! or not UTF-8 passed over without losing the lines after it, and the end. Each case
//! reads bytes held in memory through `Lines::new` and lists every piece to the end.

use std::io::Cursor;

use super::{LINE_BYTES_MAX, Lines, Piece};

fn pieces(bytes: Vec<u8>) -> Vec<Piece> {
    let mut lines = Lines::new(Cursor::new(bytes));
    let mut read = Vec::new();
    loop {
        let piece = lines.next_piece().unwrap();
        let ended = piece == Piece::End;
        read.push(piece);
        if ended {
            return read;
        }
    }
}

fn text(offset: u64, text: &str) -> Piece {
    Piece::Text {
        offset,
        text: text.to_owned(),
    }
}

mod given_lines_of_text {
    use super::*;

    #[test]
    fn when_read_then_each_comes_with_its_offset_and_without_its_end() {
        let expected = vec![text(0, "{}"), text(3, "[1]"), Piece::End];
        assert_eq!(pieces(b"{}\n[1]\n".to_vec()), expected);
    }

    #[test]
    fn when_the_last_has_no_end_then_it_is_still_read() {
        let expected = vec![text(0, "{}"), text(3, "[1]"), Piece::End];
        assert_eq!(pieces(b"{}\n[1]".to_vec()), expected);
    }

    #[test]
    fn when_a_line_is_empty_then_it_is_read_as_empty() {
        let expected = vec![text(0, ""), text(1, "{}"), Piece::End];
        assert_eq!(pieces(b"\n{}\n".to_vec()), expected);
    }
}

mod given_a_line_past_the_bound {
    use super::*;

    #[test]
    fn when_read_then_it_is_passed_over_and_the_next_line_read() {
        let mut bytes = vec![b'x'; LINE_BYTES_MAX + 1];
        bytes.extend_from_slice(b"\n{}\n");
        let next = u64::try_from(LINE_BYTES_MAX + 2).unwrap();
        let expected = vec![Piece::TooLong { offset: 0 }, text(next, "{}"), Piece::End];
        assert_eq!(pieces(bytes), expected);
    }

    #[test]
    fn when_it_is_at_the_bound_then_it_is_read() {
        let mut bytes = vec![b'x'; LINE_BYTES_MAX];
        bytes.push(b'\n');
        let read = pieces(bytes);
        let [Piece::Text { offset: 0, text }, Piece::End] = read.as_slice() else {
            panic!("a line at the bound is read whole");
        };
        assert_eq!(text.len(), LINE_BYTES_MAX);
    }
}

mod given_a_line_of_no_text {
    use super::*;

    #[test]
    fn when_read_then_it_is_passed_over_and_the_next_line_read() {
        let expected = vec![Piece::NotText { offset: 0 }, text(3, "{}"), Piece::End];
        assert_eq!(pieces(b"\xff\xfe\n{}\n".to_vec()), expected);
    }
}
