use std::ops::Range;

use gpui::{FontWeight, Pixels, ShapedLine, SharedString, Window, px};
use icu_segmenter::{LineSegmenter, options::LineBreakOptions};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(in crate::chrome::aside) struct TimingPart {
    pub(in crate::chrome::aside) word: usize,
    pub(in crate::chrome::aside) offset: Pixels,
    pub(in crate::chrome::aside) before: Pixels,
    pub(in crate::chrome::aside) width: Pixels,
}

#[derive(Clone)]
pub(in crate::chrome::aside) struct VisualUnit {
    pub(in crate::chrome::aside) range: Range<usize>,
    pub(in crate::chrome::aside) width: Pixels,
    pub(in crate::chrome::aside) parts: Vec<TimingPart>,
}

// Original text remains source of truth; units and rows only hold byte ranges into it.
#[derive(Clone)]
pub(in crate::chrome::aside) struct Wrapped {
    pub(in crate::chrome::aside) units: Vec<VisualUnit>,
    pub(in crate::chrome::aside) rows: Vec<Range<usize>>,
    pub(in crate::chrome::aside) word_widths: Vec<Pixels>,
    pub(in crate::chrome::aside) text: Vec<SharedString>,
    pub(in crate::chrome::aside) shapes: Vec<ShapedLine>,
}

pub(in crate::chrome::aside) fn lyrics_wrap_rows(
    line: &str,
    words: Option<&[music::LyricsWord]>,
    font_size: Pixels,
    width: Pixels,
    window: &mut Window,
) -> Option<Wrapped> {
    (width > px(0.)).then(|| lyrics_plan(line, words, font_size, Some(width), window))
}

pub(in crate::chrome::aside) fn lyrics_plan(
    line: &str,
    words: Option<&[music::LyricsWord]>,
    font_size: Pixels,
    width: Option<Pixels>,
    window: &mut Window,
) -> Wrapped {
    let mut style = window.text_style();
    style.font_weight = FontWeight::SEMIBOLD;
    let source = SharedString::from(line.to_owned());
    let run = style.to_run(source.len());
    let shaped = window
        .text_system()
        .shape_line(source.clone(), font_size, &[run], None);
    let timing = words
        .filter(|words| !words.is_empty())
        .map(|words| timing_spans(line, words))
        .unwrap_or_default();
    let normal = normal_break_ranges(line);
    let ranges = match width {
        Some(width) => emergency_ranges(line, normal, width, |index| shaped.x_for_index(index)),
        None => normal,
    };
    let measure = |index| shaped.x_for_index(index);
    let (units, word_widths) = measured_units(ranges, &timing, &measure);
    let rows = width
        .map(|width| wrap_unit_widths(&units, width))
        .unwrap_or_default();
    let text = rows
        .iter()
        .map(|row| {
            let start = units[row.start].range.start;
            let end = units[row.end - 1].range.end;
            SharedString::from(source.as_ref()[start..end].to_owned())
        })
        .collect::<Vec<_>>();
    let shapes = match (words.is_some(), rows.is_empty()) {
        (false, _) => Vec::new(),
        (true, true) => units
            .iter()
            .map(|unit| shaped_range(&shaped, unit.range.clone()))
            .collect(),
        (true, false) => rows
            .iter()
            .map(|row| {
                let start = units[row.start].range.start;
                let end = units[row.end - 1].range.end;
                shaped_range(&shaped, start..end)
            })
            .collect(),
    };

    Wrapped {
        units,
        rows,
        word_widths,
        text,
        shapes,
    }
}

fn shaped_range(line: &ShapedLine, range: Range<usize>) -> ShapedLine {
    let (_, suffix) = line.split_at(range.start);
    let (slice, _) = suffix.split_at(range.end - range.start);
    slice
}

pub(in crate::chrome::aside) fn measured_units(
    ranges: Vec<Range<usize>>,
    timing: &[Range<usize>],
    x_for_index: &impl Fn(usize) -> Pixels,
) -> (Vec<VisualUnit>, Vec<Pixels>) {
    let word_widths = timing
        .iter()
        .map(|span| x_for_index(span.end) - x_for_index(span.start))
        .collect::<Vec<_>>();
    let units = ranges
        .into_iter()
        .map(|range| {
            let start_x = x_for_index(range.start);
            let width = x_for_index(range.end) - start_x;
            let parts = timing
                .iter()
                .enumerate()
                .filter_map(|(word, span)| {
                    let start = range.start.max(span.start);
                    let end = range.end.min(span.end);
                    (start < end).then(|| TimingPart {
                        word,
                        offset: x_for_index(start) - start_x,
                        before: x_for_index(start) - x_for_index(span.start),
                        width: x_for_index(end) - x_for_index(start),
                    })
                })
                .collect();
            VisualUnit {
                range,
                width,
                parts,
            }
        })
        .collect::<Vec<_>>();

    (units, word_widths)
}

// Provider spans own timing only. They never supply line-break semantics.
pub(in crate::chrome::aside) fn timing_spans(
    line: &str,
    words: &[music::LyricsWord],
) -> Vec<Range<usize>> {
    let mut starts = Vec::with_capacity(words.len());
    let mut cursor = 0;
    for word in words {
        if word.text.is_empty() {
            return approximate_timing_spans(line, words);
        }
        let Some(remainder) = line.get(cursor..) else {
            return approximate_timing_spans(line, words);
        };
        let Some(relative) = remainder.find(&word.text) else {
            return approximate_timing_spans(line, words);
        };
        let start = cursor + relative;
        starts.push(start);
        cursor = start + word.text.len();
    }

    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let start = if index == 0 { 0 } else { *start };
            let end = starts.get(index + 1).copied().unwrap_or(line.len());
            start..end
        })
        .collect()
}

// Mismatched provider text is rare after normalization. Keep original bytes and distribute
// ownership over character boundaries so rendering stays lossless and safe.
fn approximate_timing_spans(line: &str, words: &[music::LyricsWord]) -> Vec<Range<usize>> {
    if words.is_empty() {
        return Vec::new();
    }
    let boundaries = line
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(line.len()))
        .collect::<Vec<_>>();
    let characters = boundaries.len() - 1;
    let weights = words
        .iter()
        .map(|word| word.text.chars().count())
        .collect::<Vec<_>>();
    let total = weights.iter().sum::<usize>();
    let mut consumed = 0usize;
    let mut start = 0;
    words
        .iter()
        .enumerate()
        .map(|(index, _)| {
            consumed += weights[index];
            let target = if index + 1 == words.len() {
                characters
            } else {
                consumed
                    .saturating_mul(characters)
                    .checked_div(total)
                    .unwrap_or((index + 1) * characters / words.len())
            };
            let end = boundaries[target.min(characters)].max(start);
            let span = start..end;
            start = end;
            span
        })
        .collect()
}

pub(in crate::chrome::aside) fn normal_break_ranges(line: &str) -> Vec<Range<usize>> {
    // ICU4X applies UAX #14/Kinsoku rules and dictionary data for Thai and the
    // other complex scripts. Timing units never participate in this pass.
    let mut offsets = LineSegmenter::new_dictionary(LineBreakOptions::default())
        .segment_str(line)
        .filter(|offset| line.is_char_boundary(*offset))
        .collect::<Vec<_>>();
    if offsets.first().copied() != Some(0) {
        offsets.insert(0, 0);
    }
    if offsets.last().copied() != Some(line.len()) {
        offsets.push(line.len());
    }
    offsets.dedup();
    offsets
        .windows(2)
        .filter_map(|pair| (pair[0] < pair[1]).then_some(pair[0]..pair[1]))
        .collect()
}

// Only an individually oversized normal segment may enter grapheme fallback.
pub(in crate::chrome::aside) fn emergency_ranges(
    line: &str,
    normal: Vec<Range<usize>>,
    width: Pixels,
    x_for_index: impl Fn(usize) -> Pixels,
) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for range in normal {
        if x_for_index(range.end) - x_for_index(range.start) <= width {
            ranges.push(range);
            continue;
        }
        let text = &line[range.clone()];
        ranges.extend(text.grapheme_indices(true).map(|(at, cluster)| {
            let start = range.start + at;
            start..start + cluster.len()
        }));
    }
    ranges
}

pub(in crate::chrome::aside) fn wrap_unit_widths(
    units: &[VisualUnit],
    width: Pixels,
) -> Vec<Range<usize>> {
    let mut rows = Vec::new();
    let mut start = 0;
    let mut used = px(0.);

    for (index, unit) in units.iter().enumerate() {
        if index > start && used + unit.width > width {
            rows.push(start..index);
            start = index;
            used = px(0.);
        }
        used += unit.width;
    }
    if start < units.len() {
        rows.push(start..units.len());
    }
    rows
}

pub(in crate::chrome::aside) fn plain_lyrics_rows(
    text: &str,
    size: Pixels,
    width: Pixels,
    window: &mut Window,
) -> Vec<SharedString> {
    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            rows.push(SharedString::from(""));
        } else if let Some(plan) = lyrics_wrap_rows(line, None, size, width, window) {
            rows.extend(plan.text);
        } else {
            rows.push(SharedString::from(line.to_owned()));
        }
    }
    rows
}

pub(in crate::chrome::aside) fn wrapped_rows(
    text: &str,
    size: Pixels,
    width: Pixels,
    window: &mut Window,
) -> usize {
    lyrics_wrap_rows(text, None, size, width, window).map_or(1, |wrapped| wrapped.rows.len().max(1))
}
