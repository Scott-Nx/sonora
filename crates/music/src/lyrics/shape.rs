use std::time::Duration;

use crate::{Lyrics, LyricsLine, LyricsWord, Voice};

const LEAST: usize = 6;
const MATCHED: f64 = 0.7;
const KEPT: f64 = 0.5;
const SEATED: f64 = 0.9;
const LIMIT: usize = 3000;
const CELLS: usize = 2_000_000;
const TICKS: &[char] = &['\'', '\u{2019}', '\u{ff07}', '\u{2018}', '\u{00b4}', '`'];

struct Sung {
    start: Duration,
    end: Duration,
    key: String,
}

use unicode_segmentation::UnicodeSegmentation;

struct Slot {
    at: usize,
    until: usize,
    key: String,
}

pub(crate) fn conform(worded: &Lyrics, guide: &Lyrics) -> Option<Lyrics> {
    let Lyrics::Synced { lines: guide } = guide else {
        return None;
    };
    let Lyrics::Synced { lines: worded } = worded else {
        return None;
    };
    if guide.is_empty() || worded.is_empty() {
        return None;
    }

    let sung = sung(worded);
    let slotted: Vec<Vec<Slot>> = guide.iter().map(|line| tokens(&line.text)).collect();
    let places: Vec<(usize, usize)> = slotted
        .iter()
        .enumerate()
        .flat_map(|(line, slots)| (0..slots.len()).map(move |slot| (line, slot)))
        .collect();
    if sung.len() < LEAST || places.len() < LEAST || sung.len() > LIMIT || places.len() > LIMIT {
        return None;
    }
    if sung.len() * places.len() > CELLS {
        return None;
    }

    let left: Vec<&str> = sung.iter().map(|word| word.key.as_str()).collect();
    let right: Vec<&str> = places
        .iter()
        .map(|(line, slot)| slotted[*line][*slot].key.as_str())
        .collect();
    let pairs = paired(&left, &right);
    if pairs.len() < LEAST
        || (pairs.len() as f64) < MATCHED * places.len() as f64
        || (pairs.len() as f64) < KEPT * sung.len() as f64
        || !seated(&pairs, &places, &slotted)
    {
        return None;
    }

    let mut spans: Vec<Option<(Duration, Duration)>> = vec![None; places.len()];
    for (word, place) in &pairs {
        spans[*place] = Some((sung[*word].start, sung[*word].end));
    }
    let spans = filled(spans, &hints(guide, &slotted));

    let mut lines: Vec<LyricsLine> = Vec::with_capacity(guide.len());
    let mut cursor = 0;
    for (index, line) in guide.iter().enumerate() {
        let slots = &slotted[index];
        if slots.is_empty() {
            continue;
        }
        let mine = &spans[cursor..cursor + slots.len()];
        cursor += slots.len();
        let words = worded_from(&line.text, slots, mine);
        let start = words.first().map(|word| word.start)?;
        let end = words.iter().map(|word| word.end).max()?;
        lines.push(LyricsLine {
            start,
            end: Some(end.max(start)),
            text: line.text.clone(),
            romanized: None,
            words: Some(words),
            secondary: Vec::new(),
            voice: Voice::Lead,
        });
    }
    super::lrc::normalize(&mut lines);
    if lines.len() < 2 {
        return None;
    }

    Some(Lyrics::Synced {
        lines: lines.into(),
    })
}

fn seated(pairs: &[(usize, usize)], places: &[(usize, usize)], slotted: &[Vec<Slot>]) -> bool {
    let mut held = vec![false; slotted.len()];
    for (_, place) in pairs {
        held[places[*place].0] = true;
    }
    let counted = slotted.iter().filter(|slots| !slots.is_empty()).count();
    let anchored = held
        .iter()
        .zip(slotted)
        .filter(|(held, slots)| **held && !slots.is_empty())
        .count();
    (anchored as f64) >= SEATED * counted as f64
}

fn sung(lines: &[LyricsLine]) -> Vec<Sung> {
    let mut sung = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        match line.words.as_deref().filter(|words| !words.is_empty()) {
            Some(words) => sung.extend(timed(words)),
            None => {
                let until = line
                    .end
                    .or_else(|| lines.get(index + 1).map(|next| next.start))
                    .unwrap_or(line.start)
                    .max(line.start);
                sung.extend(spread(&line.text, line.start, until));
            }
        }
        for lane in &line.secondary {
            match lane.words.as_deref().filter(|words| !words.is_empty()) {
                Some(words) => sung.extend(timed(words)),
                None => {
                    let until = lane.end.unwrap_or(lane.start).max(lane.start);
                    sung.extend(spread(&lane.text, lane.start, until));
                }
            }
        }
    }
    sung
}

fn timed(words: &[LyricsWord]) -> Vec<Sung> {
    if words.is_empty() {
        return Vec::new();
    }
    let mut continuous = String::new();
    let mut slices = Vec::with_capacity(words.len());
    for word in words {
        let from = continuous.len();
        continuous.push_str(&word.text);
        let until = continuous.len();
        slices.push((from..until, word.start, word.end.max(word.start)));
    }

    let slots = tokens(&continuous);
    let mut sung = Vec::with_capacity(slots.len());

    for slot in slots {
        let overlapping: Vec<_> = slices
            .iter()
            .filter(|(range, _, _)| {
                if range.is_empty() {
                    range.start == slot.at && slot.at == slot.until
                } else {
                    range.start < slot.until && range.end > slot.at
                }
            })
            .collect();

        let (start, end) = if let Some(&(first_range, first_start, first_end)) = overlapping.first()
        {
            let &(last_range, last_start, last_end) = overlapping.last().unwrap();

            let start = if slot.at <= first_range.start || first_range.is_empty() {
                *first_start
            } else {
                let dur = first_end.saturating_sub(*first_start);
                let offset = (slot.at - first_range.start) as f64;
                let total = (first_range.end - first_range.start) as f64;
                *first_start + dur.mul_f64(offset / total)
            };

            let end = if slot.until >= last_range.end || last_range.is_empty() {
                *last_end
            } else {
                let dur = last_end.saturating_sub(*last_start);
                let offset = (slot.until - last_range.start) as f64;
                let total = (last_range.end - last_range.start) as f64;
                *last_start + dur.mul_f64(offset / total)
            };

            (start, end.max(start))
        } else {
            let (start, end) = slices
                .iter()
                .min_by_key(|(range, _, _)| {
                    range
                        .start
                        .saturating_sub(slot.at)
                        .max(slot.at.saturating_sub(range.end))
                })
                .map(|(_, start, end)| (*start, *end))
                .unwrap_or_else(|| {
                    let s = words.first().map_or(Duration::ZERO, |w| w.start);
                    (s, s)
                });
            (start, end.max(start))
        };

        sung.push(Sung {
            start,
            end,
            key: slot.key,
        });
    }

    sung
}

fn spread(text: &str, start: Duration, end: Duration) -> Vec<Sung> {
    let slots = tokens(text);
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
            start: from,
            end: to,
            key: slot.key,
        });
    }
    sung
}

fn tokens(text: &str) -> Vec<Slot> {
    let mut slots = Vec::new();
    let has_alphanumeric = text.chars().any(char::is_alphanumeric);

    for (at, segment) in text.split_word_bound_indices() {
        if segment.chars().any(wide) {
            for (g_at, grapheme) in segment.grapheme_indices(true) {
                if grapheme.chars().any(wide) {
                    slots.push(Slot {
                        at: at + g_at,
                        until: at + g_at + grapheme.len(),
                        key: grapheme.to_lowercase(),
                    });
                } else if grapheme.chars().any(char::is_alphanumeric) {
                    let key: String = grapheme
                        .chars()
                        .filter(|c| !TICKS.contains(c))
                        .flat_map(char::to_lowercase)
                        .collect();
                    if !key.is_empty() {
                        slots.push(Slot {
                            at: at + g_at,
                            until: at + g_at + grapheme.len(),
                            key,
                        });
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
                slots.push(Slot {
                    at,
                    until: at + segment.len(),
                    key,
                });
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
                slots.push(Slot {
                    at,
                    until: at + segment.len(),
                    key,
                });
            }
        }
    }
    slots
}

fn wide(letter: char) -> bool {
    matches!(letter,
        '\u{3040}'..='\u{30ff}'
        | '\u{3400}'..='\u{4dbf}'
        | '\u{4e00}'..='\u{9fff}'
        | '\u{ac00}'..='\u{d7af}'
        | '\u{f900}'..='\u{faff}')
}

fn paired(left: &[&str], right: &[&str]) -> Vec<(usize, usize)> {
    let (rows, columns) = (left.len() + 1, right.len() + 1);
    let mut table = vec![0u32; rows * columns];
    for row in (0..left.len()).rev() {
        for column in (0..right.len()).rev() {
            let at = row * columns + column;
            table[at] = match akin(left[row], right[column]) {
                true => table[at + columns + 1] + 1,
                false => table[at + columns].max(table[at + 1]),
            };
        }
    }

    let mut pairs = Vec::new();
    let (mut row, mut column) = (0, 0);
    while row < left.len() && column < right.len() {
        let at = row * columns + column;
        if akin(left[row], right[column]) {
            pairs.push((row, column));
            row += 1;
            column += 1;
            continue;
        }
        match table[at + columns] >= table[at + 1] {
            true => row += 1,
            false => column += 1,
        }
    }
    pairs
}

fn akin(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let (short, long) = match left.len() <= right.len() {
        true => (left, right),
        false => (right, left),
    };
    short.len() >= 4 && long.len() - short.len() <= 2 && long.starts_with(short)
}

fn hints(guide: &[LyricsLine], slotted: &[Vec<Slot>]) -> Vec<(Duration, Duration)> {
    let mut hints = Vec::new();
    for (index, line) in guide.iter().enumerate() {
        let slots = &slotted[index];
        if slots.is_empty() {
            continue;
        }
        let until = line
            .end
            .or_else(|| guide.get(index + 1).map(|next| next.start))
            .unwrap_or(line.start)
            .max(line.start);
        let span = until.saturating_sub(line.start);
        let total: usize = slots
            .iter()
            .map(|slot| slot.key.chars().count().max(1))
            .sum();
        let mut passed = 0usize;
        for slot in slots {
            let start = line.start + span.mul_f64(passed as f64 / total as f64);
            passed += slot.key.chars().count().max(1);
            let end = line.start + span.mul_f64(passed as f64 / total as f64);
            hints.push((start, end));
        }
    }
    hints
}

fn shared(after: Duration, before: Duration, count: usize) -> Vec<(Duration, Duration)> {
    let span = before.saturating_sub(after);
    (0..count)
        .map(|step| {
            let from = after + span.mul_f64(step as f64 / count as f64);
            let to = after + span.mul_f64((step + 1) as f64 / count as f64);
            (from, to)
        })
        .collect()
}

fn filled(
    spans: Vec<Option<(Duration, Duration)>>,
    hints: &[(Duration, Duration)],
) -> Vec<(Duration, Duration)> {
    let mut settled: Vec<(Duration, Duration)> = Vec::with_capacity(spans.len());
    let mut index = 0;
    while index < spans.len() {
        if let Some(span) = spans[index] {
            settled.push(span);
            index += 1;
            continue;
        }
        let mut until = index;
        while until < spans.len() && spans[until].is_none() {
            until += 1;
        }
        let after = settled.last().map(|(_, end)| *end);
        let before = spans
            .get(until)
            .and_then(|span| *span)
            .map(|(start, _)| start);
        match (after, before) {
            (Some(after), Some(before)) if before > after => {
                settled.extend(shared(after, before, until - index));
            }
            _ => {
                for at in index..until {
                    let (mut start, mut end) = hints
                        .get(at)
                        .copied()
                        .unwrap_or_else(|| (after.unwrap_or_default(), after.unwrap_or_default()));
                    if let Some(after) = after {
                        start = start.max(after);
                        end = end.max(start);
                    }
                    if let Some(before) = before {
                        start = start.min(before);
                        end = end.min(before).max(start);
                    }
                    let floor = settled.last().map(|(_, end)| *end).unwrap_or(start);
                    settled.push((start.max(floor), end.max(start.max(floor))));
                }
            }
        }
        index = until;
    }
    settled
}

fn worded_from(text: &str, slots: &[Slot], spans: &[(Duration, Duration)]) -> Vec<LyricsWord> {
    let mut words = Vec::with_capacity(slots.len());
    for (index, slot) in slots.iter().enumerate() {
        let from = match index {
            0 => 0,
            _ => slot.at,
        };
        let until = slots
            .get(index + 1)
            .map_or(text.len(), |next| next.at.max(from));
        let (start, end) = spans[index];
        words.push(LyricsWord {
            start,
            end: end.max(start),
            text: text[from..until].to_owned(),
        });
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combining_only_provider_chunks_are_not_silently_dropped() {
        let chunk = vec![LyricsWord {
            start: Duration::from_millis(150),
            end: Duration::from_millis(300),
            text: "้".to_owned(),
        }];
        let sung = timed(&chunk);
        assert_eq!(sung.len(), 1);
        assert_eq!(sung[0].key, "้");
        assert_eq!(sung[0].start, Duration::from_millis(150));
        assert_eq!(sung[0].end, Duration::from_millis(300));

        let multi_combining = vec![LyricsWord {
            start: Duration::from_millis(200),
            end: Duration::from_millis(450),
            text: "ี่".to_owned(),
        }];
        let sung = timed(&multi_combining);
        assert_eq!(sung.len(), 1);
        assert_eq!(sung[0].key, "ี่");
        assert_eq!(sung[0].start, Duration::from_millis(200));
        assert_eq!(sung[0].end, Duration::from_millis(450));
    }

    #[test]
    fn chunk_boundary_variants_produce_equivalent_timing_and_text() {
        let fine_chunks = vec![
            LyricsWord {
                start: Duration::from_millis(0),
                end: Duration::from_millis(100),
                text: "ไ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(100),
                end: Duration::from_millis(200),
                text: "ว".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(200),
                end: Duration::from_millis(300),
                text: "้".to_owned(),
            },
        ];

        let single_chunk = vec![LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "ไว้".to_owned(),
        }];

        let fine_sung = timed(&fine_chunks);
        let single_sung = timed(&single_chunk);

        assert_eq!(fine_sung.len(), single_sung.len());
        for (fine, single) in fine_sung.iter().zip(&single_sung) {
            assert_eq!(fine.key, single.key);
            assert_eq!(fine.start, single.start);
            assert_eq!(fine.end, single.end);
        }
        assert_eq!(fine_sung.last().unwrap().end, Duration::from_millis(300));
    }

    #[test]
    fn thai_reconstructed_text_remains_exact_and_monotonic() {
        let guide = Lyrics::Synced {
            lines: vec![
                LyricsLine {
                    start: Duration::from_millis(0),
                    end: Some(Duration::from_millis(3000)),
                    text: "ไว้ ที่ นี่ เธอ อยู่ ที่".to_owned(),
                    romanized: None,
                    words: None,
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
                LyricsLine {
                    start: Duration::from_millis(3000),
                    end: Some(Duration::from_millis(6000)),
                    text: "ฉัน ยัง รอ อยู่ ตรง นี้".to_owned(),
                    romanized: None,
                    words: None,
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
            ]
            .into(),
        };

        let split_words = vec![
            LyricsWord {
                start: Duration::from_millis(0),
                end: Duration::from_millis(100),
                text: "ไ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(100),
                end: Duration::from_millis(200),
                text: "ว".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(200),
                end: Duration::from_millis(300),
                text: "้".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(300),
                end: Duration::from_millis(400),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(400),
                end: Duration::from_millis(500),
                text: "ท".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(500),
                end: Duration::from_millis(650),
                text: "ี่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(650),
                end: Duration::from_millis(700),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(700),
                end: Duration::from_millis(800),
                text: "น".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(800),
                end: Duration::from_millis(950),
                text: "ี่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(950),
                end: Duration::from_millis(1000),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1000),
                end: Duration::from_millis(1100),
                text: "เ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1100),
                end: Duration::from_millis(1200),
                text: "ธ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1200),
                end: Duration::from_millis(1300),
                text: "อ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1300),
                end: Duration::from_millis(1400),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1400),
                end: Duration::from_millis(1500),
                text: "อ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1500),
                end: Duration::from_millis(1600),
                text: "ย".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1600),
                end: Duration::from_millis(1700),
                text: "ู".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1700),
                end: Duration::from_millis(1800),
                text: "่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1800),
                end: Duration::from_millis(1900),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1900),
                end: Duration::from_millis(2000),
                text: "ท".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(2000),
                end: Duration::from_millis(2200),
                text: "ี่".to_owned(),
            },
        ];

        let worded_split = Lyrics::Synced {
            lines: vec![
                LyricsLine {
                    start: Duration::from_millis(0),
                    end: Some(Duration::from_millis(3000)),
                    text: "ไว้ ที่ นี่ เธอ อยู่ ที่".to_owned(),
                    romanized: None,
                    words: Some(split_words),
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
                LyricsLine {
                    start: Duration::from_millis(3000),
                    end: Some(Duration::from_millis(6000)),
                    text: "ฉัน ยัง รอ อยู่ ตรง นี้".to_owned(),
                    romanized: None,
                    words: None,
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
            ]
            .into(),
        };

        let word_chunks = vec![
            LyricsWord {
                start: Duration::from_millis(0),
                end: Duration::from_millis(300),
                text: "ไว้".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(300),
                end: Duration::from_millis(400),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(400),
                end: Duration::from_millis(650),
                text: "ที่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(650),
                end: Duration::from_millis(700),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(700),
                end: Duration::from_millis(950),
                text: "นี่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(950),
                end: Duration::from_millis(1000),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1000),
                end: Duration::from_millis(1300),
                text: "เธอ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1300),
                end: Duration::from_millis(1400),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1400),
                end: Duration::from_millis(1800),
                text: "อยู่".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1800),
                end: Duration::from_millis(1900),
                text: " ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(1900),
                end: Duration::from_millis(2200),
                text: "ที่".to_owned(),
            },
        ];

        let worded_whole = Lyrics::Synced {
            lines: vec![
                LyricsLine {
                    start: Duration::from_millis(0),
                    end: Some(Duration::from_millis(3000)),
                    text: "ไว้ ที่ นี่ เธอ อยู่ ที่".to_owned(),
                    romanized: None,
                    words: Some(word_chunks),
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
                LyricsLine {
                    start: Duration::from_millis(3000),
                    end: Some(Duration::from_millis(6000)),
                    text: "ฉัน ยัง รอ อยู่ ตรง นี้".to_owned(),
                    romanized: None,
                    words: None,
                    secondary: Vec::new(),
                    voice: Voice::Lead,
                },
            ]
            .into(),
        };

        let conformed_split = conform(&worded_split, &guide).expect("split worded should conform");
        let conformed_whole = conform(&worded_whole, &guide).expect("whole worded should conform");

        let Lyrics::Synced { lines: split_lines } = conformed_split else {
            unreachable!()
        };
        let Lyrics::Synced { lines: whole_lines } = conformed_whole else {
            unreachable!()
        };

        assert_eq!(split_lines[0].text, "ไว้ ที่ นี่ เธอ อยู่ ที่");
        let split_words = split_lines[0].words.as_ref().unwrap();
        let reconstructed: String = split_words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(reconstructed, "ไว้ ที่ นี่ เธอ อยู่ ที่");

        for window in split_words.windows(2) {
            assert!(window[0].start <= window[0].end);
            assert!(window[0].end <= window[1].start);
        }

        let whole_words = whole_lines[0].words.as_ref().unwrap();
        assert_eq!(split_words.len(), whole_words.len());
        for (w_split, w_whole) in split_words.iter().zip(whole_words) {
            assert_eq!(w_split.text, w_whole.text);
            assert_eq!(w_split.start, w_whole.start);
            assert_eq!(w_split.end, w_whole.end);
        }
    }

    #[test]
    fn existing_latin_cjk_japanese_korean_behavior_remains_intact() {
        let latin_slots = tokens("Hello, world! don't stop");
        let latin_keys: Vec<&str> = latin_slots.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(latin_keys, vec!["hello", "world", "dont", "stop"]);

        let cjk_slots = tokens("你好世界");
        let cjk_keys: Vec<&str> = cjk_slots.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(cjk_keys, vec!["你", "好", "世", "界"]);

        let jpn_slots = tokens("こんにちは");
        let jpn_keys: Vec<&str> = jpn_slots.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(jpn_keys, vec!["こ", "ん", "に", "ち", "は"]);

        let kor_slots = tokens("안녕하세요");
        let kor_keys: Vec<&str> = kor_slots.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(kor_keys, vec!["안", "녕", "하", "세", "요"]);
    }
}
