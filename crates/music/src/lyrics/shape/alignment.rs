use std::time::Duration;

use super::CELLS;
use super::timing::{Slot, Sung, Timing, tokens};
use crate::{LyricsLine, LyricsWord};

pub(super) struct SourceLine {
    pub(super) index: usize,
    pub(super) tokens: Option<Vec<Sung>>,
}

#[derive(Clone, Copy)]
pub(super) struct Group {
    pub(super) sources: usize,
    pub(super) guides: usize,
    pub(super) pairs: usize,
    pub(super) temporal_distance: u128,
}

pub(super) struct Alignment {
    pub(super) pairs: Vec<(usize, usize)>,
    pub(super) spans: Vec<(Duration, Duration)>,
}

#[derive(Clone, Copy)]
pub(super) enum Step {
    Source,
    Guide,
    Match(Group),
    Done,
}

#[derive(Clone, Copy)]
pub(super) struct Path {
    pub(super) score: usize,
    pub(super) temporal_distance: u128,
    pub(super) step: Step,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GuideHint {
    Bounded { start: Duration, end: Duration },
    Simultaneous { at: Duration },
    Terminal { start: Duration },
}

pub(super) enum GroupOutcome {
    Matched(Group),
    Incompatible,
    LimitExceeded,
}

pub(super) fn guide_hint(lines: &[LyricsLine], index: usize) -> GuideHint {
    let line = &lines[index];
    let following = || {
        lines[index + 1..]
            .iter()
            .map(|next| next.start)
            .find(|start| *start > line.start)
    };
    match line.end.or_else(following) {
        Some(end) if end > line.start => GuideHint::Bounded {
            start: line.start,
            end,
        },
        Some(_) => GuideHint::Simultaneous { at: line.start },
        None => GuideHint::Terminal { start: line.start },
    }
}

pub(super) fn group(
    sources: &[SourceLine],
    guides: &[Vec<Slot>],
    hints: &[GuideHint],
    work: &mut usize,
) -> GroupOutcome {
    if *work >= CELLS {
        return GroupOutcome::LimitExceeded;
    }
    if !group_temporally_compatible(sources, hints) {
        return GroupOutcome::Incompatible;
    }
    let source_len: usize = sources
        .iter()
        .map(|line| line.tokens.as_ref().map_or(0, Vec::len))
        .sum();
    let guide_len: usize = guides.iter().map(Vec::len).sum();
    if source_len == 0 || guide_len == 0 {
        return GroupOutcome::Incompatible;
    }
    let Some(cells) = source_len.checked_mul(guide_len) else {
        return GroupOutcome::LimitExceeded;
    };
    let Some(next_work) = work.checked_add(cells) else {
        return GroupOutcome::LimitExceeded;
    };
    *work = next_work;
    if *work > CELLS {
        return GroupOutcome::LimitExceeded;
    }
    let Some(source_tokens) = sources
        .iter()
        .map(|line| line.tokens.as_ref())
        .collect::<Option<Vec<_>>>()
    else {
        return GroupOutcome::Incompatible;
    };
    let source_tokens: Vec<&Sung> = source_tokens.into_iter().flatten().collect();
    let guide_tokens: Vec<(usize, &Slot)> = guides
        .iter()
        .enumerate()
        .flat_map(|(line, slots)| slots.iter().map(move |slot| (line, slot)))
        .collect();
    let Some(alignment) = local_alignment(&source_tokens, &guide_tokens, hints) else {
        return GroupOutcome::Incompatible;
    };
    let pairs = &alignment.pairs;
    for source in sources {
        let Some(first) = source_tokens
            .iter()
            .position(|token| token.source_line == source.index)
        else {
            return GroupOutcome::Incompatible;
        };
        let Some(last) = source_tokens
            .iter()
            .rposition(|token| token.source_line == source.index)
        else {
            return GroupOutcome::Incompatible;
        };
        if !pairs.iter().any(|(at, _)| (first..=last).contains(at)) {
            return GroupOutcome::Incompatible;
        }
    }
    let mut guide_cursor = 0;
    for slots in guides {
        let end = guide_cursor + slots.len();
        if !pairs.iter().any(|(_, at)| (guide_cursor..end).contains(at)) {
            return GroupOutcome::Incompatible;
        }
        guide_cursor = end;
    }
    let Some(temporal_distance) = group_temporal_distance(sources, hints) else {
        return GroupOutcome::Incompatible;
    };
    GroupOutcome::Matched(Group {
        sources: sources.len(),
        guides: guides.len(),
        pairs: alignment.pairs.len(),
        temporal_distance,
    })
}

pub(super) fn represented(pairs: &[(usize, usize)], source_len: usize, guide_len: usize) -> bool {
    let boundary =
        pairs.first() == Some(&(0, 0)) && pairs.last() == Some(&(source_len - 1, guide_len - 1));
    boundary
        && pairs
            .windows(2)
            .all(|pair| pair[1].0 == pair[0].0 + 1 || pair[1].1 > pair[0].1 + 1)
}

pub(super) fn build_group(
    sources: &[SourceLine],
    guides: &[LyricsLine],
    guide_slots: &[Vec<Slot>],
    guide_hints: &[GuideHint],
    original: &[LyricsLine],
) -> Option<Vec<LyricsLine>> {
    let source_tokens: Vec<&Sung> = sources
        .iter()
        .flat_map(|line| line.tokens.as_ref().into_iter().flatten())
        .collect();
    let guide_tokens: Vec<(usize, &Slot)> = guide_slots
        .iter()
        .enumerate()
        .flat_map(|(line, slots)| slots.iter().map(move |slot| (line, slot)))
        .collect();
    let spans = local_alignment(&source_tokens, &guide_tokens, guide_hints)?.spans;

    let owner = sources
        .iter()
        .max_by_key(|source| {
            let line = &original[source.index];
            guide_hints
                .iter()
                .map(|hint| {
                    line_temporal_overlap(
                        line.start,
                        line.sung_end().unwrap_or(line.start).max(line.start),
                        *hint,
                    )
                })
                .max()
                .unwrap_or_default()
        })?
        .index;
    let owner = &original[owner];
    let owner_keys: Vec<_> = tokens(&owner.text)
        .into_iter()
        .map(|slot| slot.key)
        .collect();

    let mut output = Vec::with_capacity(guides.len());
    let mut cursor = 0;
    for (index, (guide, slots)) in guides.iter().zip(guide_slots).enumerate() {
        let end = cursor + slots.len();
        let words = worded_from(&guide.text, slots, &spans[cursor..end]);
        cursor = end;
        let start = words.first()?.start;
        let finish = words.iter().map(|word| word.end).max()?.max(start);
        let guide_keys: Vec<_> = slots.iter().map(|slot| slot.key.clone()).collect();
        let mut line = LyricsLine {
            start,
            end: Some(finish),
            text: guide.text.clone(),
            romanized: guide.romanized.clone().or_else(|| {
                (sources.len() == 1 && guides.len() == 1 && owner_keys == guide_keys)
                    .then(|| owner.romanized.clone())
                    .flatten()
            }),
            words: Some(words),
            secondary: if index == 0 {
                owner.secondary.clone()
            } else {
                Vec::new()
            },
            voice: owner.voice,
        };
        crate::lyrics::lrc::normalize_conformed_background(&mut line);
        output.push(line);
    }
    Some(output)
}

pub(super) fn token_temporally_compatible(
    t_start: Duration,
    t_end: Duration,
    hint: GuideHint,
) -> bool {
    if t_start == t_end {
        return match hint {
            GuideHint::Bounded { start, end } => t_start >= start && t_start < end,
            GuideHint::Simultaneous { at } => t_start == at,
            GuideHint::Terminal { start } => t_start >= start,
        };
    }
    match hint {
        GuideHint::Bounded { start, end } => t_end.min(end) > t_start.max(start),
        GuideHint::Simultaneous { at } => t_start <= at && t_end > at,
        GuideHint::Terminal { start } => t_end > start,
    }
}

pub(super) fn line_temporally_compatible(
    s_start: Duration,
    s_end: Duration,
    hint: GuideHint,
) -> bool {
    if s_start == s_end {
        return token_temporally_compatible(s_start, s_end, hint);
    }
    match hint {
        GuideHint::Bounded { start, end } => s_end.min(end) > s_start.max(start),
        GuideHint::Simultaneous { at } => s_start <= at && s_end > at,
        GuideHint::Terminal { start } => s_end > start,
    }
}

fn source_extent(source: &SourceLine) -> Option<(Duration, Duration)> {
    let tokens = source.tokens.as_ref()?;
    let start = tokens.iter().map(|token| token.start).min()?;
    let end = tokens.iter().map(|token| token.end).max()?.max(start);
    Some((start, end))
}

pub(super) fn source_temporally_compatible(source: &SourceLine, hint: GuideHint) -> bool {
    source_extent(source).is_some_and(|(start, end)| line_temporally_compatible(start, end, hint))
}

fn group_temporally_compatible(sources: &[SourceLine], hints: &[GuideHint]) -> bool {
    sources.iter().all(|source| {
        hints
            .iter()
            .any(|hint| source_temporally_compatible(source, *hint))
    }) && hints.iter().all(|hint| {
        sources
            .iter()
            .any(|source| source_temporally_compatible(source, *hint))
    })
}

fn guide_start(hint: GuideHint) -> Duration {
    match hint {
        GuideHint::Bounded { start, .. } | GuideHint::Terminal { start } => start,
        GuideHint::Simultaneous { at } => at,
    }
}

fn start_distance(left: Duration, right: Duration) -> u128 {
    left.abs_diff(right).as_nanos()
}

fn group_temporal_distance(sources: &[SourceLine], hints: &[GuideHint]) -> Option<u128> {
    let source_starts: Vec<_> = sources
        .iter()
        .map(|source| source_extent(source).map(|extent| extent.0))
        .collect::<Option<_>>()?;
    let source_distance = source_starts.iter().map(|source| {
        hints
            .iter()
            .map(|hint| start_distance(*source, guide_start(*hint)))
            .min()
            .unwrap_or_default()
    });
    let guide_distance = hints.iter().map(|hint| {
        source_starts
            .iter()
            .map(|source| start_distance(*source, guide_start(*hint)))
            .min()
            .unwrap_or_default()
    });
    Some(
        source_distance
            .chain(guide_distance)
            .fold(0u128, u128::saturating_add),
    )
}

fn line_temporal_overlap(s_start: Duration, s_end: Duration, hint: GuideHint) -> Duration {
    match hint {
        GuideHint::Bounded { start, end } => s_end.min(end).saturating_sub(s_start.max(start)),
        GuideHint::Simultaneous { .. } => Duration::ZERO,
        GuideHint::Terminal { start } => {
            if s_end > start {
                s_end.saturating_sub(s_start.max(start))
            } else {
                Duration::ZERO
            }
        }
    }
}

pub(super) fn local_pairs(
    source: &[&Sung],
    guide: &[(usize, &Slot)],
    hints: &[GuideHint],
) -> Vec<(usize, usize)> {
    paired_by(source.len(), guide.len(), |row, column| {
        akin(&source[row].key, &guide[column].1.key)
            && (source[row].timing == Timing::Synthesized
                || token_temporally_compatible(
                    source[row].start,
                    source[row].end,
                    hints[guide[column].0],
                ))
    })
}

pub(super) fn local_alignment(
    source: &[&Sung],
    guide: &[(usize, &Slot)],
    hints: &[GuideHint],
) -> Option<Alignment> {
    let pairs = local_pairs(source, guide, hints);
    if pairs.is_empty() || !represented(&pairs, source.len(), guide.len()) {
        return None;
    }
    if pairs
        .windows(2)
        .any(|pair| source[pair[0].0].start > source[pair[1].0].start)
    {
        return None;
    }
    let mut spans = vec![None; guide.len()];
    for &(source_at, guide_at) in &pairs {
        let token = source[source_at];
        spans[guide_at] = Some((token.start, token.end));
    }
    Some(Alignment {
        pairs,
        spans: filled(spans)?,
    })
}

fn paired_by(
    left: usize,
    right: usize,
    matches: impl Fn(usize, usize) -> bool,
) -> Vec<(usize, usize)> {
    let (rows, columns) = (left + 1, right + 1);
    let mut table = vec![0u32; rows * columns];

    for row in (0..left).rev() {
        for column in (0..right).rev() {
            let at = row * columns + column;
            table[at] = match matches(row, column) {
                true => table[at + columns + 1] + 1,
                false => table[at + columns].max(table[at + 1]),
            };
        }
    }

    let mut pairs = Vec::new();
    let (mut row, mut column) = (0, 0);
    while row < left && column < right {
        let at = row * columns + column;
        if matches(row, column) {
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

fn filled(spans: Vec<Option<(Duration, Duration)>>) -> Option<Vec<(Duration, Duration)>> {
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
        let (after, before) = (after?, before?);
        let span = before.checked_sub(after)?;
        let count = until - index;
        settled.extend((0..count).map(|step| {
            (
                after + span.mul_f64(step as f64 / count as f64),
                after + span.mul_f64((step + 1) as f64 / count as f64),
            )
        }));
        index = until;
    }
    Some(settled)
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
