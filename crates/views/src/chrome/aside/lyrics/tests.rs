use std::ops::Range;
use std::time::Duration;

use gpui::{Bounds, ContentMask, Pixels, SharedString, point, px};
use music::{LyricsLane, LyricsLine, LyricsWord, Voice};
use ui::Motion;
use unicode_segmentation::UnicodeSegmentation;

use super::super::{active_lyrics_row, lyric_row_count};
use super::karaoke::{
    RevealRange, background_line_singing, karaoke_window, line_has_passed, primary_karaoke_fade,
    primary_karaoke_visible, progress_between, reveal_mask, revealed, secondary_karaoke_visible,
};
use super::layout::{
    TimingPart, VisualUnit, Wrapped, emergency_ranges, measured_units, normal_break_ranges,
    timing_spans, wrap_unit_widths,
};

fn test_units(widths: &[Pixels]) -> Vec<VisualUnit> {
    widths
        .iter()
        .map(|&width| VisualUnit {
            range: 0..0,
            width,
            parts: Vec::new(),
        })
        .collect()
}

fn segments(text: &str) -> Vec<&str> {
    normal_break_ranges(text)
        .into_iter()
        .map(|range| &text[range])
        .collect()
}

fn timed_words(parts: &[&str]) -> Vec<LyricsWord> {
    parts
        .iter()
        .enumerate()
        .map(|(index, text)| LyricsWord {
            start: Duration::from_millis(index as u64 * 100),
            end: Duration::from_millis(index as u64 * 100 + 100),
            text: (*text).to_owned(),
        })
        .collect()
}

fn reveal_plan() -> Wrapped {
    Wrapped {
        units: vec![
            VisualUnit {
                range: 0..2,
                width: px(10.),
                parts: vec![TimingPart {
                    word: 0,
                    offset: px(0.),
                    before: px(0.),
                    width: px(10.),
                }],
            },
            VisualUnit {
                range: 3..5,
                width: px(10.),
                parts: vec![TimingPart {
                    word: 1,
                    offset: px(0.),
                    before: px(0.),
                    width: px(10.),
                }],
            },
        ],
        rows: std::iter::once(0..2).collect(),
        word_widths: vec![px(10.), px(10.)],
        text: vec![SharedString::from("AA BB")],
        shapes: Vec::new(),
    }
}

fn emergency_ranges_for_test(
    text: &str,
    normal: Vec<Range<usize>>,
    width: Pixels,
) -> Vec<Range<usize>> {
    emergency_ranges(text, normal, width, |index| px(index as f32))
}

#[test]
fn word_timing_exposes_a_pause_hidden_by_the_line_end() {
    let lines = [
        LyricsLine {
            start: Duration::from_secs(2),
            end: Some(Duration::from_secs(12)),
            text: "first".to_owned(),
            romanized: None,
            words: Some(vec![LyricsWord {
                start: Duration::from_secs(2),
                end: Duration::from_secs(5),
                text: "first".to_owned(),
            }]),
            secondary: Vec::new(),
            voice: Voice::Lead,
        },
        LyricsLine {
            start: Duration::from_secs(12),
            end: Some(Duration::from_secs(15)),
            text: "second".to_owned(),
            romanized: None,
            words: None,
            secondary: Vec::new(),
            voice: Voice::Lead,
        },
    ];

    assert_eq!(lyric_row_count(&lines), 3);
    assert_eq!(active_lyrics_row(&lines, Duration::from_secs(8)), Some(1));
    assert!(line_has_passed(&lines[0], Duration::from_secs(8)));
}

#[test]
fn timing_spans_keep_original_spacing() {
    let text = "I said oooh I'm drowning in the night";
    let words = ["I", "said", "oooh", "I'm", "drowning", "in", "the", "night"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| LyricsWord {
            start: Duration::from_millis(index as u64 * 100),
            end: Duration::from_millis(index as u64 * 100 + 100),
            text: text.to_owned(),
        })
        .collect::<Vec<_>>();

    let spans = timing_spans(text, &words);
    assert_eq!(
        spans
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<Vec<_>>(),
        [
            "I ",
            "said ",
            "oooh ",
            "I'm ",
            "drowning ",
            "in ",
            "the ",
            "night"
        ]
    );
}

#[test]
fn normal_breaking_keeps_original_text() {
    let text = "Ладони полны слёзок, но время";
    let ranges = normal_break_ranges(text);

    assert_eq!(ranges.first().map(|range| range.start), Some(0));
    assert_eq!(ranges.last().map(|range| range.end), Some(text.len()));
    assert_eq!(
        ranges
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<Vec<_>>(),
        ["Ладони ", "полны ", "слёзок, ", "но ", "время"]
    );
}

#[test]
fn lyrics_wrap_only_at_normal_unit_boundaries() {
    let units = test_units(&[px(60.), px(30.), px(20.)]);
    let rows = wrap_unit_widths(&units, px(80.));

    assert_eq!(rows, [0..1, 1..3]);
}

#[test]
fn lyrics_keep_an_oversized_unit_on_its_own_row() {
    let rows = wrap_unit_widths(&test_units(&[px(120.), px(30.), px(30.)]), px(80.));

    assert_eq!(rows, [0..1, 1..3]);
}

#[test]
fn timing_parts_can_cross_a_wrapped_row() {
    let timing = [0..2, 2..5, 5..6];
    let (units, widths) = measured_units(vec![0..3, 3..6], &timing, &|index| px(index as f32));

    assert_eq!(widths, [px(2.), px(3.), px(1.)]);
    assert_eq!(
        units[0]
            .parts
            .iter()
            .map(|part| part.word)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(
        units[1]
            .parts
            .iter()
            .map(|part| part.word)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(units[1].parts[0].before, px(1.));
    assert_eq!(wrap_unit_widths(&units, px(3.)), [0..1, 1..2]);
}

#[test]
fn a_late_first_word_starts_at_provider_time() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1900),
            text: "first".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(2000),
            end: Duration::from_millis(2400),
            text: "second".to_owned(),
        },
    ];

    assert_eq!(
        karaoke_window(&words, 0),
        (Duration::from_millis(1500), Duration::from_millis(1900))
    );
    let plan = reveal_plan();
    let windows = [(Duration::from_millis(1500), Duration::from_millis(1900))];
    assert!(revealed(&plan, 0..1, &windows, Duration::from_millis(1499)).is_empty());
}

#[test]
fn overlapping_word_windows_keep_provider_ends() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(2000),
            text: "A".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(2500),
            text: "B".to_owned(),
        },
    ];

    assert_eq!(
        karaoke_window(&words, 0),
        (Duration::from_millis(1000), Duration::from_millis(2000))
    );
    assert_eq!(
        karaoke_window(&words, 1),
        (Duration::from_millis(1500), Duration::from_millis(2500))
    );
}

#[test]
fn zero_duration_word_uses_only_limited_fallback() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(1000),
            text: "A".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(2000),
            text: "B".to_owned(),
        },
    ];

    assert_eq!(
        karaoke_window(&words, 0),
        (Duration::from_millis(1000), Duration::from_millis(1500))
    );
    assert_eq!(
        karaoke_window(&words[..1], 0),
        (Duration::from_millis(1000), Duration::from_millis(1180))
    );
}

#[test]
fn positive_duration_progress_is_linear() {
    let start = Duration::from_millis(1000);
    let end = Duration::from_millis(2000);

    assert_eq!(
        progress_between(start, end, Duration::from_millis(999)),
        0.0
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(1000)),
        0.0
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(1250)),
        0.25
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(1500)),
        0.5
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(1750)),
        0.75
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(2000)),
        1.0
    );
    assert_eq!(
        progress_between(start, end, Duration::from_millis(2500)),
        1.0
    );
}

#[test]
fn sequential_words_join_continuously_at_boundary() {
    let a_start = Duration::from_millis(1000);
    let a_end = Duration::from_millis(1500);
    let b_start = Duration::from_millis(1500);
    let b_end = Duration::from_millis(2000);

    let boundary = Duration::from_millis(1500);
    assert_eq!(progress_between(a_start, a_end, boundary), 1.0);
    assert_eq!(progress_between(b_start, b_end, boundary), 0.0);
}

#[test]
fn reveal_mask_geometry_rules() {
    let parent = ContentMask {
        bounds: Bounds::from_corners(point(px(10.), px(20.)), point(px(200.), px(100.))),
    };
    let origin_x = px(100.);
    let line_width = px(50.);

    let interior = reveal_mask(
        parent,
        origin_x,
        line_width,
        RevealRange {
            start: px(20.),
            end: px(40.),
        },
    );
    assert_eq!(interior.bounds.left(), px(120.));
    assert_eq!(interior.bounds.right(), px(140.));
    assert_eq!(interior.bounds.top(), px(20.));
    assert_eq!(interior.bounds.bottom(), px(100.));

    let starts_at_edge = reveal_mask(
        parent,
        origin_x,
        line_width,
        RevealRange {
            start: px(0.),
            end: px(40.),
        },
    );
    assert_eq!(starts_at_edge.bounds.left(), px(10.));
    assert_eq!(starts_at_edge.bounds.right(), px(140.));
    assert_eq!(starts_at_edge.bounds.top(), px(20.));
    assert_eq!(starts_at_edge.bounds.bottom(), px(100.));

    let terminal_complete = reveal_mask(
        parent,
        origin_x,
        line_width,
        RevealRange {
            start: px(30.),
            end: px(50.),
        },
    );
    assert_eq!(terminal_complete.bounds.left(), px(130.));
    assert_eq!(terminal_complete.bounds.right(), px(200.));
    assert_eq!(terminal_complete.bounds.top(), px(20.));
    assert_eq!(terminal_complete.bounds.bottom(), px(100.));

    let row_complete = reveal_mask(
        parent,
        origin_x,
        line_width,
        RevealRange {
            start: px(0.),
            end: px(50.),
        },
    );
    assert_eq!(row_complete.bounds.left(), px(10.));
    assert_eq!(row_complete.bounds.right(), px(200.));
    assert_eq!(row_complete.bounds.top(), px(20.));
    assert_eq!(row_complete.bounds.bottom(), px(100.));
}

#[test]
fn overlapping_reveal_ranges_do_not_fill_the_gap() {
    let plan = reveal_plan();
    let windows = [
        (Duration::from_millis(1000), Duration::from_millis(2000)),
        (Duration::from_millis(1500), Duration::from_millis(2500)),
    ];
    let reveal = revealed(
        &plan,
        0..plan.units.len(),
        &windows,
        Duration::from_millis(1750),
    );

    assert_eq!(reveal.len(), 2);
    assert_eq!(reveal[0].start, px(0.));
    assert_eq!(reveal[0].end, px(7.5));
    assert_eq!(reveal[1].start, px(10.));
    assert_eq!(reveal[1].end, px(12.5));
}

#[test]
fn sequential_reveal_ranges_still_join_continuously() {
    let plan = reveal_plan();
    let windows = [
        (Duration::from_millis(1000), Duration::from_millis(1500)),
        (Duration::from_millis(1500), Duration::from_millis(2000)),
    ];
    let reveal = revealed(
        &plan,
        0..plan.units.len(),
        &windows,
        Duration::from_millis(1750),
    );

    assert_eq!(reveal.len(), 1);
    assert_eq!(reveal[0].start, px(0.));
    assert_eq!(reveal[0].end, px(15.));
}

#[test]
fn a_finished_background_lane_stays_sung_until_its_line_departs() {
    let lane = LyricsLane {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(3)),
        text: "(E)".to_owned(),
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(3),
            text: "(E)".to_owned(),
        }]),
    };

    assert!(!secondary_karaoke_visible(
        &lane,
        true,
        Duration::from_millis(1999)
    ));
    assert!(secondary_karaoke_visible(
        &lane,
        true,
        Duration::from_secs(2)
    ));
    assert!(secondary_karaoke_visible(
        &lane,
        true,
        Duration::from_secs(4)
    ));
    assert!(!secondary_karaoke_visible(
        &lane,
        false,
        Duration::from_secs(4)
    ));
}

#[test]
fn an_overlapped_primary_line_keeps_singing_in_the_background() {
    let line = LyricsLine {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(8)),
        text: "Wake me up inside".to_owned(),
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(8),
            text: "Wake me up inside".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    assert!(!primary_karaoke_visible(
        &line,
        false,
        Duration::from_millis(1999)
    ));
    assert!(primary_karaoke_visible(
        &line,
        false,
        Duration::from_secs(5)
    ));
    assert!(primary_karaoke_visible(
        &line,
        false,
        Duration::from_secs(8)
    ));
    assert!(!primary_karaoke_visible(
        &line,
        false,
        Duration::from_secs(8) + Motion::Base.span()
    ));
}

#[test]
fn the_active_primary_line_keeps_its_completed_sweep_until_departure() {
    let line = LyricsLine {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(5)),
        text: "line".to_owned(),
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(5),
            text: "line".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    assert!(primary_karaoke_visible(&line, true, Duration::from_secs(8)));
}

#[test]
fn a_finished_background_line_fades_from_white_to_gray() {
    let line = LyricsLine {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(8)),
        text: "Wake me up inside".to_owned(),
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(8),
            text: "Wake me up inside".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    let fade = Motion::Control.span();

    assert_eq!(
        primary_karaoke_fade(&line, false, Duration::from_millis(7999)),
        0.
    );
    assert_eq!(
        primary_karaoke_fade(&line, false, Duration::from_secs(8) + fade / 2),
        0.5
    );
    assert_eq!(
        primary_karaoke_fade(&line, false, Duration::from_secs(8) + fade),
        1.
    );
    assert_eq!(
        primary_karaoke_fade(&line, true, Duration::from_secs(8) + fade),
        0.
    );
}

#[test]
fn only_a_currently_singing_background_line_gets_the_reduced_blur() {
    let line = LyricsLine {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(8)),
        text: "Wake me up inside".to_owned(),
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(8),
            text: "Wake me up inside".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    assert!(!background_line_singing(
        &line,
        false,
        Duration::from_millis(1999)
    ));
    assert!(background_line_singing(
        &line,
        false,
        Duration::from_secs(5)
    ));
    assert!(!background_line_singing(
        &line,
        false,
        Duration::from_secs(8)
    ));
    assert!(!background_line_singing(
        &line,
        true,
        Duration::from_secs(5)
    ));
}

#[test]
fn a_finished_line_stays_past_during_a_gap() {
    let line = LyricsLine {
        start: Duration::from_secs(2),
        end: Some(Duration::from_secs(5)),
        text: "line".to_owned(),
        romanized: None,
        words: None,
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    assert!(line_has_passed(&line, Duration::from_secs(8)));
}

#[test]
fn thai_normal_breaks_ignore_timing_granularity() {
    let text = "เข้าใจสักที";
    let expected = ["เข้าใจ", "สัก", "ที"];
    let granularities = [
        ["เข้า", "ใจ", "สั", "ก", "ที"].as_slice(),
        ["เข้าใจ", "สัก", "ที"].as_slice(),
        ["เข้าใจสักที"].as_slice(),
    ];

    assert_eq!(segments(text), expected);
    for parts in granularities {
        let words = timed_words(parts);
        let spans = timing_spans(text, &words);
        let (units, _) =
            measured_units(normal_break_ranges(text), &spans, &|index| px(index as f32));
        assert_eq!(
            spans
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<Vec<_>>(),
            parts
        );
        assert_eq!(wrap_unit_widths(&units, px(20.)), [0..1, 1..3]);
    }
}

#[test]
fn mismatched_timing_still_owns_the_original_line() {
    let text = "เข้าใจสักที";
    let words = timed_words(&["เข้าใจ", "missing"]);
    let spans = timing_spans(text, &words);

    assert_eq!(
        spans
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<String>(),
        text
    );
    assert!(
        spans.iter().all(|range| {
            text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
        })
    );
}

#[test]
fn required_thai_phrases_use_dictionary_boundaries() {
    for (text, expected) in [
        (
            "ฉันรักเธอมากที่สุด",
            ["ฉัน", "รัก", "เธอ", "มาก", "ที่สุด"].as_slice(),
        ),
        ("กรุงเทพมหานคร", ["กรุงเทพมหานคร"].as_slice()),
        (
            "อยากให้เข้าใจกันสักที",
            ["อยาก", "ให้", "เข้าใจ", "กัน", "สัก", "ที"].as_slice(),
        ),
        ("ประเทศไทย", ["ประเทศไทย"].as_slice()),
    ] {
        assert_eq!(segments(text), expected);
    }
}

#[test]
fn provider_grapheme_splits_do_not_create_normal_breaks() {
    for (text, parts) in [
        ("สัก", ["ส", "ั", "ก"].as_slice()),
        ("ที่", ["ท", "ี่"].as_slice()),
    ] {
        let words = timed_words(parts);
        let spans = timing_spans(text, &words);
        let normal = normal_break_ranges(text);
        let normal_offsets = normal
            .iter()
            .flat_map(|range| [range.start, range.end])
            .collect::<Vec<_>>();

        assert_eq!(
            spans
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<Vec<_>>(),
            parts
        );
        assert_eq!(segments(text), [text]);
        assert!(
            spans
                .iter()
                .skip(1)
                .all(|range| !normal_offsets.contains(&range.start))
        );
    }
}

#[test]
fn emergency_ranges_split_only_the_oversized_normal_unit() {
    let text = "a bbbbb c";
    let normal = normal_break_ranges(text);
    let ranges = emergency_ranges_for_test(text, normal.clone(), px(3.));
    let pieces = ranges
        .iter()
        .map(|range| &text[range.clone()])
        .collect::<Vec<_>>();

    assert_eq!(
        normal
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<Vec<_>>(),
        ["a ", "bbbbb ", "c"]
    );
    assert_eq!(pieces.first(), Some(&"a "));
    assert_eq!(pieces.last(), Some(&"c"));
    assert_eq!(pieces[1..pieces.len() - 1].concat(), "bbbbb ");
    assert!(pieces.len() > normal.len());
}

#[test]
fn emergency_ranges_preserve_extended_graphemes() {
    for text in ["สัก", "ที่", "มากๆ", "กรุงเทพฯ"] {
        let normal = normal_break_ranges(text);
        let ranges = emergency_ranges_for_test(text, normal, px(1.));
        let expected = text
            .grapheme_indices(true)
            .map(|(at, cluster)| at..at + cluster.len())
            .collect::<Vec<_>>();

        assert_eq!(ranges, expected, "{text}");
    }
}

#[test]
fn cjk_and_kinsoku_boundaries_survive() {
    assert_eq!(segments("你好世界"), ["你", "好", "世", "界"]);
    assert_eq!(segments("「你好」"), ["「你", "好」"]);

    for text in [
        "いゝ", "いゞ", "イヽ", "イヾ", "あっ", "アッ", "日々", "あー", "あ…", "あ、", "あ。",
    ] {
        assert_eq!(segments(text), [text], "{text}");
    }
}

#[test]
fn mixed_script_breaks_reconstruct_the_source() {
    for text in ["Sonora เพลงดีมาก", "เพลงดีมาก Sonora", "Hello世界 เพลง"]
    {
        let ranges = normal_break_ranges(text);
        assert_eq!(ranges.first().map(|range| range.start), Some(0));
        assert_eq!(ranges.last().map(|range| range.end), Some(text.len()));
        assert_eq!(
            ranges
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<String>(),
            text
        );
        assert!(ranges.iter().all(
            |range| text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
        ));
    }
}
