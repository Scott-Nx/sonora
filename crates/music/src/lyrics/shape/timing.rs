use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use super::CELLS;
#[cfg(test)]
use super::LIMIT;
use crate::{LyricsLine, LyricsWord};

const TICKS: &[char] = &['\'', '\u{2019}', '\u{ff07}', '\u{2018}', '\u{00b4}', '`'];

pub(super) struct Sung {
    pub(super) source_line: usize,
    pub(super) start: Duration,
    pub(super) end: Duration,
    pub(super) key: String,
    pub(super) timing: Timing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Timing {
    Provider,
    Synthesized,
}

pub(super) struct Slot {
    pub(super) at: usize,
    pub(super) until: usize,
    pub(super) key: String,
}

pub(super) enum CanonicalOutcome {
    Mapped(Vec<Sung>),
    Unsafe(usize),
    LimitExceeded,
}

pub(super) fn canonical(
    line: &LyricsLine,
    next: Option<&LyricsLine>,
    limit: usize,
    work: &mut usize,
) -> CanonicalOutcome {
    if *work >= CELLS {
        return CanonicalOutcome::LimitExceeded;
    }
    let Some(slots) = bounded_tokens_to(&line.text, limit) else {
        return CanonicalOutcome::LimitExceeded;
    };
    let slot_count = slots.len();
    if slot_count == 0 {
        return CanonicalOutcome::Unsafe(0);
    }

    let tokens = match line.words.as_deref().filter(|words| !words.is_empty()) {
        Some(words) => match timed_with_limit_and_slots(&line.text, words, &slots, work) {
            Some(sung) => sung,
            None => {
                if *work >= CELLS {
                    return CanonicalOutcome::LimitExceeded;
                }
                return CanonicalOutcome::Unsafe(slot_count);
            }
        },
        None => {
            let until = line
                .end
                .or_else(|| next.map(|next| next.start))
                .unwrap_or(line.start)
                .max(line.start);
            spread(&slots, line.start, until)
        }
    };
    if tokens.is_empty() {
        if *work >= CELLS {
            CanonicalOutcome::LimitExceeded
        } else {
            CanonicalOutcome::Unsafe(slot_count)
        }
    } else {
        CanonicalOutcome::Mapped(tokens)
    }
}

#[cfg(test)]
pub(super) fn timed(text: &str, words: &[LyricsWord]) -> Option<Vec<Sung>> {
    let mut work = 0;
    timed_with_limit(text, words, LIMIT, &mut work)
}

#[cfg(test)]
pub(super) fn timed_with_limit(
    text: &str,
    words: &[LyricsWord],
    limit: usize,
    work: &mut usize,
) -> Option<Vec<Sung>> {
    let slots = bounded_tokens_to(text, limit)?;
    timed_with_limit_and_slots(text, words, &slots, work)
}

fn timed_with_limit_and_slots(
    text: &str,
    words: &[LyricsWord],
    slots: &[Slot],
    work: &mut usize,
) -> Option<Vec<Sung>> {
    if words.is_empty() || text.is_empty() {
        return None;
    }
    let effective_text = text.to_owned();

    let slices = map_words_to_text_with_limit(&effective_text, words, slots, work)?;
    let mut sung = Vec::with_capacity(slots.len());
    let mut slice_cursor = 0;

    for slot in slots {
        while slice_cursor < slices.len() && slices[slice_cursor].0.end <= slot.at {
            slice_cursor += 1;
        }

        let mut first: Option<&(std::ops::Range<usize>, Duration, Duration)> = None;
        let mut last: Option<&(std::ops::Range<usize>, Duration, Duration)> = None;

        for slice in &slices[slice_cursor..] {
            if slice.0.start >= slot.until {
                break;
            }
            if slice.0.end > slot.at {
                if first.is_none() {
                    first = Some(slice);
                }
                last = Some(slice);
            }
        }

        let (first_slice, last_slice) = match (first, last) {
            (Some(f), Some(l)) => (f, l),
            _ => return None,
        };

        let (first_range, first_start, first_end) = (&first_slice.0, first_slice.1, first_slice.2);
        let (last_range, last_start, last_end) = (&last_slice.0, last_slice.1, last_slice.2);

        let mut start = if slot.at <= first_range.start
            || first_range.is_empty()
            || effective_text[first_range.start..slot.at]
                .chars()
                .all(char::is_whitespace)
        {
            first_start
        } else {
            let dur = first_end.saturating_sub(first_start);
            let offset = (slot.at - first_range.start) as f64;
            let total = (first_range.end - first_range.start) as f64;
            first_start + dur.mul_f64(offset / total)
        };

        let mut end = if slot.until >= last_range.end
            || last_range.is_empty()
            || effective_text[slot.until..last_range.end]
                .chars()
                .all(char::is_whitespace)
        {
            last_end
        } else {
            let dur = last_end.saturating_sub(last_start);
            let offset = (slot.until - last_range.start) as f64;
            let total = (last_range.end - last_range.start) as f64;
            last_start + dur.mul_f64(offset / total)
        };

        for slice in &slices[slice_cursor..] {
            if slice.0.start >= slot.until {
                break;
            }
            if slice.0.end > slot.at {
                if slot.at <= slice.0.start {
                    start = start.min(slice.1);
                }
                if slot.until >= slice.0.end {
                    end = end.max(slice.2);
                }
            }
        }

        sung.push(Sung {
            source_line: 0,
            start,
            end: end.max(start),
            key: slot.key.clone(),
            timing: Timing::Provider,
        });
    }

    Some(sung)
}

#[cfg(test)]
pub(super) fn map_words_to_text(
    text: &str,
    words: &[LyricsWord],
) -> Option<Vec<(std::ops::Range<usize>, Duration, Duration)>> {
    let mut work = 0;
    let slots = bounded_tokens_to(text, LIMIT)?;
    map_words_to_text_with_limit(text, words, &slots, &mut work)
}

pub(super) fn map_words_to_text_with_limit(
    text: &str,
    words: &[LyricsWord],
    canonical_slots: &[Slot],
    work: &mut usize,
) -> Option<Vec<(std::ops::Range<usize>, Duration, Duration)>> {
    let total_word_len: usize = words.iter().map(|w| w.text.len()).sum();
    if total_word_len == text.len()
        && words.iter().map(|w| w.text.as_str()).collect::<String>() == text
    {
        let mut slice_ranges = Vec::with_capacity(words.len());
        let mut from = 0;
        for word in words {
            let until = from + word.text.len();
            slice_ranges.push(from..until);
            from = until;
        }
        if !validate_bidirectional_coverage(words, &slice_ranges, canonical_slots) {
            return None;
        }
        return Some(
            words
                .iter()
                .zip(slice_ranges)
                .map(|(word, range)| (range, word.start, word.end.max(word.start)))
                .collect(),
        );
    }

    let mut anchors: Vec<Option<std::ops::Range<usize>>> = vec![None; words.len()];
    let mut search_idx = 0;

    for (w_idx, word) in words.iter().enumerate() {
        let w_trimmed = word
            .text
            .trim_matches(|c: char| c.is_whitespace() || TICKS.contains(&c));
        if w_trimmed.is_empty() {
            continue;
        }
        let w_chars: Vec<char> = w_trimmed.chars().collect();

        for slot in &canonical_slots[search_idx..] {
            *work = work.saturating_add(1);
            if *work >= CELLS {
                return None;
            }
            let start_byte = slot.at;
            if let Some(matched_len) = match_prefix(&w_chars, &text[start_byte..], work) {
                let match_end = start_byte + matched_len;
                anchors[w_idx] = Some(start_byte..match_end);
                while search_idx < canonical_slots.len()
                    && canonical_slots[search_idx].at < match_end
                {
                    search_idx += 1;
                }
                break;
            }
        }
    }

    // Require at least one trustworthy anchor; never fabricate mapping across unrelated text
    if *work >= CELLS || anchors.iter().all(Option::is_none) {
        return None;
    }

    let mut slice_ranges: Vec<std::ops::Range<usize>> = vec![0..0; words.len()];
    let mut i = 0;
    while i < words.len() {
        if let Some(range) = &anchors[i] {
            slice_ranges[i] = range.clone();
            i += 1;
        } else {
            let run_start = i;
            while i < words.len() && anchors[i].is_none() {
                i += 1;
            }
            let run_end = i;
            let gap_start = if run_start == 0 {
                0
            } else {
                anchors[run_start - 1].as_ref().unwrap().end
            };

            let gap_end = if run_end == words.len() {
                text.len()
            } else {
                anchors[run_end].as_ref().unwrap().start
            };

            let meaningful: Vec<usize> = (run_start..run_end)
                .filter(|index| meaningful_cue(&words[*index].text))
                .collect();
            if !meaningful.is_empty() && (run_start == 0 || run_end == words.len()) {
                return None;
            }
            let partitioned = partition_gap(gap_start..gap_end, meaningful.len(), canonical_slots)?;
            for (index, range) in meaningful.into_iter().zip(partitioned) {
                slice_ranges[index] = range;
            }
        }
    }

    if !validate_bidirectional_coverage(words, &slice_ranges, canonical_slots) {
        return None;
    }

    Some(
        words
            .iter()
            .zip(slice_ranges)
            .map(|(word, range)| (range, word.start, word.end.max(word.start)))
            .collect(),
    )
}

fn validate_bidirectional_coverage(
    words: &[LyricsWord],
    ranges: &[std::ops::Range<usize>],
    slots: &[Slot],
) -> bool {
    if slots.is_empty() {
        return words.iter().all(|w| !meaningful_cue(&w.text));
    }

    let mut slot_idx = 0;
    for (word, range) in words.iter().zip(ranges) {
        if !meaningful_cue(&word.text) {
            continue;
        }
        while slot_idx < slots.len() && slots[slot_idx].until <= range.start {
            slot_idx += 1;
        }
        if slot_idx == slots.len() || slots[slot_idx].at >= range.end {
            return false;
        }
    }

    let mut cue_idx = 0;
    for slot in slots {
        while cue_idx < ranges.len() && ranges[cue_idx].end <= slot.at {
            cue_idx += 1;
        }
        let mut covered = false;
        let mut k = cue_idx;
        while k < ranges.len() && ranges[k].start < slot.until {
            if meaningful_cue(&words[k].text) {
                covered = true;
                break;
            }
            k += 1;
        }
        if !covered {
            return false;
        }
    }

    true
}

fn meaningful_cue(text: &str) -> bool {
    !text
        .trim_matches(|letter: char| letter.is_whitespace() || TICKS.contains(&letter))
        .is_empty()
}

fn partition_gap(
    gap: std::ops::Range<usize>,
    count: usize,
    slots: &[Slot],
) -> Option<Vec<std::ops::Range<usize>>> {
    if count == 0 {
        return Some(Vec::new());
    }
    let available: Vec<&Slot> = slots
        .iter()
        .filter(|slot| slot.at >= gap.start && slot.until <= gap.end)
        .collect();
    if available.len() < count {
        return None;
    }
    let mut ranges = Vec::with_capacity(count);
    for i in 0..count {
        let first = i * available.len() / count;
        let last = ((i + 1) * available.len() / count).saturating_sub(1);
        ranges.push(available[first].at..available[last].until);
    }
    Some(ranges)
}

fn match_prefix(w_chars: &[char], text_rem: &str, work: &mut usize) -> Option<usize> {
    let mut w_idx = 0;
    let mut last_matched_byte = 0;

    for (t_byte, t_char) in text_rem.char_indices() {
        *work = work.saturating_add(1);
        if *work >= CELLS {
            return None;
        }
        while w_idx < w_chars.len() && TICKS.contains(&w_chars[w_idx]) {
            w_idx += 1;
        }
        if w_idx == w_chars.len() {
            return Some(last_matched_byte);
        }

        if TICKS.contains(&t_char) {
            last_matched_byte = t_byte + t_char.len_utf8();
            continue;
        }

        let w_char = w_chars[w_idx];
        if char_alike(w_char, t_char) {
            w_idx += 1;
            last_matched_byte = t_byte + t_char.len_utf8();
        } else if w_idx > 0 && !t_char.is_alphanumeric() && !wide(t_char) {
            last_matched_byte = t_byte + t_char.len_utf8();
        } else {
            return None;
        }
    }

    while w_idx < w_chars.len()
        && (TICKS.contains(&w_chars[w_idx]) || w_chars[w_idx].is_whitespace())
    {
        w_idx += 1;
    }
    if w_idx == w_chars.len() {
        Some(last_matched_byte.max(text_rem.len()))
    } else {
        None
    }
}

fn char_alike(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    a.to_lowercase().eq(b.to_lowercase())
}

fn spread(slots: &[Slot], start: Duration, end: Duration) -> Vec<Sung> {
    let total: usize = slots.iter().map(|slot| slot.key.chars().count()).sum();
    if total == 0 {
        return Vec::new();
    }
    let span = end.saturating_sub(start);
    let mut sung = Vec::with_capacity(slots.len());
    let mut passed = 0usize;
    for slot in slots {
        let length = slot.key.chars().count();
        let from = start + span.mul_f64(passed as f64 / total as f64);
        passed += length;
        let to = start + span.mul_f64(passed as f64 / total as f64);
        sung.push(Sung {
            source_line: 0,
            start: from,
            end: to,
            key: slot.key.clone(),
            timing: Timing::Synthesized,
        });
    }
    sung
}

pub(super) fn tokens(text: &str) -> Vec<Slot> {
    tokenize(text, usize::MAX).unwrap()
}

pub(super) fn bounded_tokens_to(text: &str, limit: usize) -> Option<Vec<Slot>> {
    tokenize(text, limit)
}

fn tokenize(text: &str, limit: usize) -> Option<Vec<Slot>> {
    let mut slots = Vec::new();
    let has_alphanumeric = text.chars().any(char::is_alphanumeric);

    for (at, segment) in text.split_word_bound_indices() {
        if segment.chars().any(wide) {
            for (g_at, grapheme) in segment.grapheme_indices(true) {
                if grapheme.chars().any(wide) {
                    push_slot(
                        &mut slots,
                        limit,
                        Slot {
                            at: at + g_at,
                            until: at + g_at + grapheme.len(),
                            key: grapheme.to_lowercase(),
                        },
                    )?;
                } else if grapheme.chars().any(char::is_alphanumeric) {
                    let key: String = grapheme
                        .chars()
                        .filter(|c| !TICKS.contains(c))
                        .flat_map(char::to_lowercase)
                        .collect();
                    if !key.is_empty() {
                        push_slot(
                            &mut slots,
                            limit,
                            Slot {
                                at: at + g_at,
                                until: at + g_at + grapheme.len(),
                                key,
                            },
                        )?;
                    }
                }
            }
            continue;
        }

        if segment.chars().any(char::is_alphanumeric) {
            let key: String = segment
                .chars()
                .filter(|c| !TICKS.contains(c))
                .flat_map(char::to_lowercase)
                .collect();
            if !key.is_empty() {
                push_slot(
                    &mut slots,
                    limit,
                    Slot {
                        at,
                        until: at + segment.len(),
                        key,
                    },
                )?;
            }
            continue;
        }

        if !has_alphanumeric && !segment.chars().all(char::is_whitespace) {
            let key: String = segment
                .chars()
                .filter(|c| !TICKS.contains(c))
                .flat_map(char::to_lowercase)
                .collect();
            if !key.is_empty() {
                push_slot(
                    &mut slots,
                    limit,
                    Slot {
                        at,
                        until: at + segment.len(),
                        key,
                    },
                )?;
            }
        }
    }
    Some(slots)
}

fn push_slot(slots: &mut Vec<Slot>, limit: usize, slot: Slot) -> Option<()> {
    if slots.len() >= limit {
        None
    } else {
        slots.push(slot);
        Some(())
    }
}

fn wide(letter: char) -> bool {
    matches!(letter,
        '\u{3040}'..='\u{30ff}'
        | '\u{3400}'..='\u{4dbf}'
        | '\u{4e00}'..='\u{9fff}'
        | '\u{ac00}'..='\u{d7af}'
        | '\u{f900}'..='\u{faff}')
}
