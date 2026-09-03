use std::time::Duration;

use crate::{LyricsLine, LyricsWord, Voice};

pub(super) use crate::lyrics::shape::alignment::*;
pub(super) use crate::lyrics::shape::timing::*;
pub(super) use crate::lyrics::shape::*;

mod alignment;
mod regressions;
mod timing;

pub(super) fn thai_worded_line(words: &[&str], start_ms: u64, dur_per_word_ms: u64) -> LyricsLine {
    let mut word_items = Vec::new();
    let mut curr = start_ms;
    for (i, &w) in words.iter().enumerate() {
        if i > 0 {
            word_items.push(LyricsWord {
                start: Duration::from_millis(curr),
                end: Duration::from_millis(curr + 50),
                text: " ".to_owned(),
            });
            curr += 50;
        }
        word_items.push(LyricsWord {
            start: Duration::from_millis(curr),
            end: Duration::from_millis(curr + dur_per_word_ms),
            text: w.to_owned(),
        });
        curr += dur_per_word_ms;
    }
    let text = words.join(" ");
    LyricsLine {
        start: Duration::from_millis(start_ms),
        end: Some(Duration::from_millis(curr)),
        text,
        romanized: None,
        words: Some(word_items),
        secondary: Vec::new(),
        voice: Voice::Lead,
    }
}

pub(super) fn guide_line(text: &str, start_ms: u64, end_ms: u64) -> LyricsLine {
    LyricsLine {
        start: Duration::from_millis(start_ms),
        end: Some(Duration::from_millis(end_ms)),
        text: text.to_owned(),
        romanized: None,
        words: None,
        secondary: Vec::new(),
        voice: Voice::Lead,
    }
}
