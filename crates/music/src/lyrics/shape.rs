use crate::Lyrics;

mod alignment;
#[cfg(test)]
mod tests;
mod timing;

use self::alignment::{
    GroupOutcome, GuideHint, Path, SourceLine, Step, build_group, group, guide_hint,
    source_temporally_compatible,
};
use self::timing::{CanonicalOutcome, bounded_tokens_to, canonical};

pub(super) const LEAST: usize = 6;
pub(super) const MATCHED: f64 = 0.7;
pub(super) const KEPT: f64 = 0.5;
pub(super) const SEATED: f64 = 0.9;
pub(super) const LIMIT: usize = 3000;
pub(super) const CELLS: usize = 2_000_000;

pub(crate) fn conform(worded: &Lyrics, guide: &Lyrics) -> Option<Lyrics> {
    let Lyrics::Synced { lines: guide } = guide else {
        return None;
    };
    let Lyrics::Synced { lines: worded } = worded else {
        return None;
    };
    if guide.is_empty() || worded.is_empty() || worded.len() > LIMIT || guide.len() > LIMIT {
        return None;
    }
    if worded
        .iter()
        .chain(guide.iter())
        .any(|line| line.text.len() > CELLS)
        || worded.iter().any(|line| {
            line.words
                .iter()
                .flatten()
                .any(|word| word.text.len() > CELLS)
        })
    {
        return None;
    }

    let raw_cues: usize = worded
        .iter()
        .map(|line| line.words.as_ref().map_or(1, |words| words.len().max(1)))
        .sum();
    if raw_cues > LIMIT || worded.len().checked_mul(guide.len())? > CELLS {
        return None;
    }

    let mut source_tokens = 0usize;
    let mut sources = Vec::with_capacity(worded.len());
    let mut work = 0usize;
    for (index, line) in worded.iter().enumerate() {
        let remaining_tokens = LIMIT.checked_sub(source_tokens)?;
        let outcome = canonical(line, worded.get(index + 1), remaining_tokens, &mut work);
        let tokens = match outcome {
            CanonicalOutcome::LimitExceeded => return None,
            CanonicalOutcome::Unsafe(count) => {
                source_tokens = source_tokens.checked_add(count)?;
                if source_tokens > LIMIT {
                    return None;
                }
                None
            }
            CanonicalOutcome::Mapped(mut tokens) => {
                source_tokens = source_tokens.checked_add(tokens.len())?;
                if source_tokens > LIMIT {
                    return None;
                }
                for token in &mut tokens {
                    token.source_line = index;
                }
                Some(tokens)
            }
        };
        sources.push(SourceLine { index, tokens });
    }
    let mut guide_tokens = 0usize;
    let mut guide_slots = Vec::with_capacity(guide.len());
    for line in guide.iter() {
        let slots = bounded_tokens_to(&line.text, LIMIT - guide_tokens)?;
        guide_tokens = guide_tokens.checked_add(slots.len())?;
        if guide_tokens > LIMIT {
            return None;
        }
        guide_slots.push(slots);
    }
    if !(LEAST..=LIMIT).contains(&source_tokens) || !(LEAST..=LIMIT).contains(&guide_tokens) {
        return None;
    }
    let guide_hints: Vec<GuideHint> = (0..guide.len())
        .map(|index| guide_hint(guide, index))
        .collect();

    let columns = guide.len() + 1;
    let mut paths = vec![
        Path {
            score: 0,
            temporal_distance: 0,
            step: Step::Done,
        };
        (worded.len() + 1) * columns
    ];

    for source in (0..=worded.len()).rev() {
        for guide_at in (0..=guide.len()).rev() {
            if source == worded.len() && guide_at == guide.len() {
                continue;
            }
            let at = source * columns + guide_at;
            let mut best = if source < worded.len() {
                Path {
                    score: paths[(source + 1) * columns + guide_at].score,
                    temporal_distance: paths[(source + 1) * columns + guide_at].temporal_distance,
                    step: Step::Source,
                }
            } else {
                Path {
                    score: paths[source * columns + guide_at + 1].score,
                    temporal_distance: paths[source * columns + guide_at + 1].temporal_distance,
                    step: Step::Guide,
                }
            };

            if guide_at < guide.len() {
                let skipped = paths[source * columns + guide_at + 1].score;
                let skipped_distance = paths[source * columns + guide_at + 1].temporal_distance;
                if skipped > best.score
                    || (skipped == best.score && skipped_distance < best.temporal_distance)
                {
                    best = Path {
                        score: skipped,
                        temporal_distance: skipped_distance,
                        step: Step::Guide,
                    };
                }
            }

            if source < worded.len()
                && guide_at < guide.len()
                && sources[source].tokens.is_some()
                && !guide_slots[guide_at].is_empty()
            {
                if work >= CELLS {
                    return None;
                }
                let remaining_sources = worded.len() - source;
                let remaining_guides = guide.len() - guide_at;
                let consider = |source_count: usize,
                                guide_count: usize,
                                best: &mut Path,
                                work: &mut usize|
                 -> Result<(), ()> {
                    match group(
                        &sources[source..source + source_count],
                        &guide_slots[guide_at..guide_at + guide_count],
                        &guide_hints[guide_at..guide_at + guide_count],
                        work,
                    ) {
                        GroupOutcome::LimitExceeded => Err(()),
                        GroupOutcome::Incompatible => Ok(()),
                        GroupOutcome::Matched(group) => {
                            let tail =
                                paths[(source + source_count) * columns + guide_at + guide_count];
                            let score = group.pairs + tail.score;
                            let temporal_distance = group
                                .temporal_distance
                                .saturating_add(tail.temporal_distance);
                            if score > best.score
                                || (score == best.score
                                    && (temporal_distance < best.temporal_distance
                                        || (temporal_distance == best.temporal_distance
                                            && matches!(best.step, Step::Guide))))
                            {
                                *best = Path {
                                    score,
                                    temporal_distance,
                                    step: Step::Match(group),
                                };
                            }
                            Ok(())
                        }
                    }
                };
                if consider(1, 1, &mut best, &mut work).is_err() {
                    return None;
                }
                for source_count in 2..=remaining_sources {
                    let added = &sources[source + source_count - 1];
                    if added.tokens.is_none()
                        || !source_temporally_compatible(added, guide_hints[guide_at])
                    {
                        break;
                    }
                    if consider(source_count, 1, &mut best, &mut work).is_err() {
                        return None;
                    }
                }
                for guide_count in 2..=remaining_guides {
                    let added = guide_at + guide_count - 1;
                    if guide_slots[added].is_empty()
                        || !source_temporally_compatible(&sources[source], guide_hints[added])
                    {
                        break;
                    }
                    if consider(1, guide_count, &mut best, &mut work).is_err() {
                        return None;
                    }
                }
            }
            paths[at] = best;
        }
    }

    let mut source = 0;
    let mut guide_at = 0;
    let mut matched = 0;
    let mut matched_guide_lines = 0;
    let mut output = Vec::with_capacity(worded.len() + guide.len());
    while source < worded.len() || guide_at < guide.len() {
        match paths[source * columns + guide_at].step {
            Step::Source => {
                output.push((worded[source].clone(), source, 0));
                source += 1;
            }
            Step::Guide => guide_at += 1,
            Step::Match(group) => {
                let source_group = &sources[source..source + group.sources];
                let guide_group = &guide[guide_at..guide_at + group.guides];
                let slots = &guide_slots[guide_at..guide_at + group.guides];
                let hints = &guide_hints[guide_at..guide_at + group.guides];
                let built = build_group(source_group, guide_group, slots, hints, worded)?;
                matched += group.pairs;
                matched_guide_lines += group.guides;
                output.extend(
                    built
                        .into_iter()
                        .enumerate()
                        .map(|(split, line)| (line, source, split)),
                );
                source += group.sources;
                guide_at += group.guides;
            }
            Step::Done => break,
        }
    }

    let safe_source_tokens: usize = sources
        .iter()
        .filter_map(|line| line.tokens.as_ref())
        .map(Vec::len)
        .sum();
    let guide_lines = guide_slots.iter().filter(|slots| !slots.is_empty()).count();
    if matched < LEAST
        || (matched as f64) < MATCHED * guide_tokens as f64
        || (matched as f64) < KEPT * safe_source_tokens as f64
        || (matched_guide_lines as f64) < SEATED * guide_lines as f64
    {
        return None;
    }

    output.sort_by(
        |(left, left_source, left_split), (right, right_source, right_split)| {
            left.start
                .cmp(&right.start)
                .then_with(|| left_source.cmp(right_source))
                .then_with(|| left_split.cmp(right_split))
        },
    );
    let output: Vec<_> = output.into_iter().map(|(line, _, _)| line).collect();
    (output.len() >= 2).then(|| Lyrics::Synced {
        lines: output.into(),
    })
}
