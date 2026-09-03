use std::time::Duration;

use super::*;
use crate::{Lyrics, LyricsLine, LyricsWord, Voice};

#[test]
fn two_source_lines_contributing_to_one_guide_line_are_not_duplicated() {
    let part_a = &["hello", "world"];
    let part_b = &["stay", "with", "me"];
    let part_c = &["never", "let", "go", "again"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(part_a, 0, 300),
            thai_worded_line(part_b, 1000, 300),
            thai_worded_line(part_c, 3000, 300),
        ]
        .into(),
    };

    let guide_text_0 = format!("{} {}", part_a.join(" "), part_b.join(" "));
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&guide_text_0, 0, 2500),
            guide_line(&part_c.join(" "), 3000, 5000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Output must have only 2 lines (guide lines), neither part_a nor part_b duplicated
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, guide_text_0);
    assert_eq!(lines[1].text, part_c.join(" "));
}

#[test]
fn one_source_line_contributing_to_two_guide_lines_is_not_duplicated() {
    let full = &["hello", "world", "stay", "with", "me"];
    let part_c = &["never", "let", "go", "again"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(full, 0, 300),
            thai_worded_line(part_c, 3000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("hello world", 0, 1000),
            guide_line("stay with me", 1000, 2500),
            guide_line(&part_c.join(" "), 3000, 5000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Output must have 3 lines, source line 0 not duplicated
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, "hello world");
    assert_eq!(lines[1].text, "stay with me");
    assert_eq!(lines[2].text, part_c.join(" "));
}

#[test]
fn equal_vote_ties_resolve_deterministically_by_topology() {
    let part_a = &["alpha", "beta"];
    let part_b = &["gamma", "delta"];
    let part_c = &["other", "content", "here", "now"];

    // line_a has 2 tokens and 500ms duration
    let mut line_a = thai_worded_line(part_a, 0, 250);
    line_a.voice = Voice::Counter;

    // line_b has 2 tokens and 2000ms duration (larger temporal overlap with guide line)
    let mut line_b = thai_worded_line(part_b, 1000, 1000);
    line_b.voice = Voice::Lead;

    let source = Lyrics::Synced {
        lines: vec![line_a, line_b, thai_worded_line(part_c, 4000, 300)].into(),
    };

    // Guide line combines both: 2 tokens from line_a, 2 tokens from line_b
    let guide_text_0 = format!("{} {}", part_a.join(" "), part_b.join(" "));
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&guide_text_0, 0, 3500),
            guide_line(&part_c.join(" "), 4000, 6000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // line_b wins the equal-vote tie due to larger temporal overlap
    assert_eq!(lines[0].voice, Voice::Lead);
}

#[test]
fn repeated_lyric_text_preserves_topology_and_unmatched_repetition() {
    let chorus = &["we", "will", "rock", "you"];
    let verse = &["buddy", "youre", "a", "boy"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(chorus, 0, 300),
            thai_worded_line(verse, 2000, 300),
            thai_worded_line(chorus, 4000, 300),
        ]
        .into(),
    };

    // Guide only contains the first chorus and verse, missing the repeated chorus at 4000
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&chorus.join(" "), 0, 1500),
            guide_line(&verse.join(" "), 2000, 3500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // 3 lines: first chorus conformed, verse conformed, second chorus preserved intact at 4000ms
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, chorus.join(" "));
    assert_eq!(lines[1].text, verse.join(" "));
    assert_eq!(lines[2].text, chorus.join(" "));
    assert_eq!(lines[2].start, Duration::from_millis(4000));
}

#[test]
fn word_timing_extending_past_line_end_is_used_for_attribution() {
    let part_a = &["alpha", "beta"];
    let part_b = &["gamma", "delta"];
    let part_c = &["other", "content", "here", "now"];

    // line_a: line.end is 500ms, but its words extend to 2500ms!
    let mut line_a = thai_worded_line(part_a, 0, 1200);
    line_a.end = Some(Duration::from_millis(500));
    line_a.voice = Voice::Counter;

    // line_b follows line_a, keeping merged provider timing monotonic.
    let mut line_b = thai_worded_line(part_b, 2500, 400);
    line_b.end = Some(Duration::from_millis(3350));
    line_b.voice = Voice::Lead;

    let source = Lyrics::Synced {
        lines: vec![line_a, line_b, thai_worded_line(part_c, 3000, 300)].into(),
    };

    // Guide line is at 1000..3500.
    // Under raw line.end, line_a ends at 500ms (0 overlap).
    // Under sung_end(), line_a extends to 2450ms (overlaps 1000..2000 by 1000ms!).
    let guide_text_0 = format!("{} {}", part_a.join(" "), part_b.join(" "));
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&guide_text_0, 1000, 3500),
            guide_line(&part_c.join(" "), 3000, 5000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // line_a wins the attribution because its sung_end() extends past line.end
    assert_eq!(lines[0].voice, Voice::Counter);
}

#[test]
fn guide_line_with_no_slots_does_not_misalign_or_panic() {
    let words = &["valid", "lyric", "line"];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words, 0, 300),
            thai_worded_line(&["second", "lyric", "line"], 2000, 300),
        ]
        .into(),
    };

    // Guide line 0 has 0 slots (empty text)
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("", 0, 500),
            guide_line(&words.join(" "), 0, 1500),
            guide_line("second lyric line", 2000, 3500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, words.join(" "));
    assert_eq!(lines[1].text, "second lyric line");
}

#[test]
fn changed_but_legitimately_bounded_wording_still_reconstructs_guide_line() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_source = &["hello", "wurld", "again", "tonight"];
    let words_guide = "hello world again tonight";
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_source, 3000, 300),
            thai_worded_line(words_2, 6000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_1.join(" "), 0, 2500),
            guide_line(words_guide, 3000, 4500),
            guide_line(&words_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, words_1.join(" "));
    assert_eq!(lines[1].text, words_guide);
    assert_eq!(lines[2].text, words_2.join(" "));
}

#[test]
fn boundary_ownership_strictly_requires_paired_boundary_tokens_not_key_equality() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_solo = &["love", "something", "completely", "love"];
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_solo, 3000, 300),
            thai_worded_line(words_2, 6000, 300),
        ]
        .into(),
    };

    // Guide has 3 lines; Line 1 is "love" (only first word paired, last word "love" not paired)
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_1.join(" "), 0, 2500),
            guide_line("love", 3000, 4500),
            guide_line(&words_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn temporally_far_source_candidate_text_paired_by_lcs_is_not_consumed() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_far = &["hello", "world"];
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_far, 50000, 300),
            thai_worded_line(words_2, 6000, 300),
        ]
        .into(),
    };

    // Guide line 1 is at 3000..4500ms; source line is at 50000ms (0 temporal overlap with guide hint)
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_1.join(" "), 0, 2500),
            guide_line("hello world", 3000, 4500),
            guide_line(&words_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn equal_start_lines_preserve_source_topology_order_and_reversed_case() {
    let words_unconsumed = &["alpha", "unconsumed", "source", "line"];
    let words_conformed = &[
        "bravo",
        "conformed",
        "source",
        "line",
        "three",
        "four",
        "five",
        "six",
    ];
    let words_late = &["charlie", "late", "line", "here"];

    // Case 1: Unconsumed first, conformed second
    let source_1 = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_unconsumed, 1000, 300),
            thai_worded_line(words_conformed, 1000, 300),
            thai_worded_line(words_late, 5000, 300),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_conformed.join(" "), 1000, 4000),
            guide_line(&words_late.join(" "), 5000, 7000),
        ]
        .into(),
    };

    let conformed_1 = conform(&source_1, &guide).expect("should conform");
    let Lyrics::Synced { lines: lines_1 } = conformed_1 else {
        panic!("expected synced lyrics");
    };
    assert_eq!(lines_1.len(), 3);
    assert_eq!(lines_1[0].text, words_unconsumed.join(" "));
    assert_eq!(lines_1[1].text, words_conformed.join(" "));
    assert_eq!(lines_1[2].text, words_late.join(" "));

    // Case 2 (Reversed): Conformed first, unconsumed second
    let source_2 = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_conformed, 1000, 300),
            thai_worded_line(words_unconsumed, 1000, 300),
            thai_worded_line(words_late, 5000, 300),
        ]
        .into(),
    };

    let conformed_2 = conform(&source_2, &guide).expect("should conform");
    let Lyrics::Synced { lines: lines_2 } = conformed_2 else {
        panic!("expected synced lyrics");
    };
    assert_eq!(lines_2.len(), 3);
    assert_eq!(lines_2[0].text, words_conformed.join(" "));
    assert_eq!(lines_2[1].text, words_unconsumed.join(" "));
    assert_eq!(lines_2[2].text, words_late.join(" "));
}

#[test]
fn temporally_far_source_with_more_tokens_does_not_usurp_temporally_correct_source() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_far = &["alpha", "bravo", "charlie", "delta"];
    let words_correct = &["alpha", "bravo", "charlie"];
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_correct, 3000, 300),
            thai_worded_line(words_2, 6000, 300),
            thai_worded_line(words_far, 50000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_1.join(" "), 0, 2500),
            guide_line(&words_correct.join(" "), 3000, 4500),
            guide_line(&words_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Temporally-correct source (words_correct @ 3000ms) is represented by guide line 1.
    // Temporally-far source (words_far @ 50000ms) survives intact.
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0].text, words_1.join(" "));
    assert_eq!(lines[1].text, words_correct.join(" "));
    assert_eq!(lines[1].start, Duration::from_millis(3000));
    assert_eq!(lines[2].text, words_2.join(" "));
    assert_eq!(lines[3].text, words_far.join(" "));
    assert_eq!(lines[3].start, Duration::from_millis(50000));
}

#[test]
fn terminal_guide_line_with_no_end_matches_containing_source_span() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_split = &["split", "first", "half", "and", "split", "second", "half"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_split, 3000, 300),
        ]
        .into(),
    };

    // Realistic line-synced LRC: final lines have end: None
    let guide = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::from_millis(0),
                end: None,
                text: words_1.join(" "),
                romanized: None,
                words: None,
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            LyricsLine {
                start: Duration::from_millis(3000),
                end: None,
                text: "split first half".to_owned(),
                romanized: None,
                words: None,
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            LyricsLine {
                start: Duration::from_millis(4000),
                end: None,
                text: "and split second half".to_owned(),
                romanized: None,
                words: None,
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, words_1.join(" "));
    assert_eq!(lines[1].text, "split first half");
    assert_eq!(lines[2].text, "and split second half");
}

#[test]
fn source_timing_token_is_not_reused_across_multiple_guide_lines() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_src = &["go", "home"];
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_src, 3000, 300),
            thai_worded_line(words_2, 6000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_1.join(" "), 0, 2500),
            guide_line("go", 3000, 3500),
            guide_line("go home", 3500, 4500),
            guide_line(&words_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Guide 1 has "go", Guide 2 has "go home".
    // The source "go" token must not be reused as timing in both lines.
    let line_go = lines.iter().find(|l| l.text == "go").unwrap();
    let line_go_home = lines.iter().find(|l| l.text == "go home").unwrap();

    let go_word_in_1 = &line_go.words.as_ref().unwrap()[0];
    let go_word_in_2 = &line_go_home.words.as_ref().unwrap()[0];

    assert_ne!(
        (go_word_in_1.start, go_word_in_1.end),
        (go_word_in_2.start, go_word_in_2.end),
        "source token 'go' must not be reused for both guide lines"
    );
}

#[test]
fn temporal_source_guide_edges_are_token_aware() {
    // Repeated "love" in source: token 1 at 1..2s, token 3 at 9..10s
    let source_words = vec![
        LyricsWord {
            text: "start".to_owned(),
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
        },
        LyricsWord {
            text: "love".to_owned(),
            start: Duration::from_millis(1000),
            end: Duration::from_millis(2000),
        },
        LyricsWord {
            text: "middle".to_owned(),
            start: Duration::from_millis(4000),
            end: Duration::from_millis(5000),
        },
        LyricsWord {
            text: "love".to_owned(),
            start: Duration::from_millis(9000),
            end: Duration::from_millis(10000),
        },
        LyricsWord {
            text: "end".to_owned(),
            start: Duration::from_millis(10000),
            end: Duration::from_millis(11000),
        },
    ];
    let line_0 = LyricsLine {
        start: Duration::from_millis(0),
        end: Some(Duration::from_millis(11000)),
        text: "start love middle love end".to_owned(),
        romanized: None,
        words: Some(source_words),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    let words_surround = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let source = Lyrics::Synced {
        lines: vec![line_0, thai_worded_line(words_surround, 20000, 300)].into(),
    };

    // Guide lines: 0s start, 9s love, 10s end
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("start", 0, 9000),
            guide_line("love", 9000, 10000),
            guide_line("end", 10000, 11000),
            guide_line(&words_surround.join(" "), 20000, 23000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn terminal_guide_line_timing_semantics() {
    // Terminal guide @ 10000 with corresponding source starting @ 10100 must be compatible
    let hint_terminal = GuideHint::Terminal {
        start: Duration::from_millis(10000),
    };
    assert!(token_temporally_compatible(
        Duration::from_millis(10100),
        Duration::from_millis(10500),
        hint_terminal
    ));

    // Source token ending exactly @ 10000 must NOT steal the terminal line
    assert!(!token_temporally_compatible(
        Duration::from_millis(9000),
        Duration::from_millis(10000),
        hint_terminal
    ));

    // Token ending before 10000 must NOT match
    assert!(!token_temporally_compatible(
        Duration::from_millis(8000),
        Duration::from_millis(9500),
        hint_terminal
    ));
}

#[test]
fn provider_token_without_temporal_overlap_has_no_edge() {
    let words = vec![
        LyricsWord {
            start: Duration::from_secs(0),
            end: Duration::from_secs(1),
            text: "start ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(1),
            end: Duration::from_secs(2),
            text: "love ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(9),
            end: Duration::from_secs(10),
            text: "end".to_owned(),
        },
    ];
    let source = timed("start love end", &words).unwrap();
    let source: Vec<_> = source.iter().collect();
    let slots = tokens("start love end");
    let guide: Vec<_> = slots.iter().map(|slot| (0, slot)).collect();
    let pairs = local_pairs(
        &source,
        &guide,
        &[GuideHint::Bounded {
            start: Duration::from_secs(8),
            end: Duration::from_secs(10),
        }],
    );

    assert!(!pairs.contains(&(1, 1)));
    assert_eq!(pairs, vec![(2, 2)]);
}

#[test]
fn zero_duration_provider_cue_is_a_point() {
    let at = Duration::from_secs(2);
    assert!(token_temporally_compatible(
        at,
        at,
        GuideHint::Bounded {
            start: Duration::from_secs(2),
            end: Duration::from_secs(3),
        }
    ));
    assert!(!token_temporally_compatible(
        at,
        at,
        GuideHint::Bounded {
            start: Duration::from_secs(1),
            end: Duration::from_secs(2),
        }
    ));
    assert!(token_temporally_compatible(
        at,
        at,
        GuideHint::Simultaneous { at }
    ));
}

#[test]
fn mostly_unrelated_guide_fails_global_confidence() {
    let first = &["one", "two", "three", "four"];
    let second = &["five", "six", "seven", "eight"];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(first, 0, 200),
            thai_worded_line(second, 2000, 200),
        ]
        .into(),
    };
    let mut guide = vec![
        guide_line(&first.join(" "), 0, 1000),
        guide_line(&second.join(" "), 2000, 3000),
    ];
    guide.extend((0..8).map(|index| {
        guide_line(
            &format!("unrelated guide line {index}"),
            4000 + index * 1000,
            4500 + index * 1000,
        )
    }));

    assert!(
        conform(
            &source,
            &Lyrics::Synced {
                lines: guide.into()
            }
        )
        .is_none()
    );
}

#[test]
fn reversed_provider_starts_reject_group() {
    let reversed = vec![
        LyricsWord {
            start: Duration::from_millis(800),
            end: Duration::from_millis(1500),
            text: "A ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
            text: "B".to_owned(),
        },
    ];
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(1500)),
                text: "A B".to_owned(),
                romanized: None,
                words: Some(reversed),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            thai_worded_line(anchor, 3000, 300),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("A B", 0, 1500),
            guide_line(&anchor.join(" "), 3000, 5200),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn same_start_guides_use_next_distinct_boundary() {
    let text_a = &["one", "two", "three"];
    let text_b = &["four", "five", "six"];
    let text_c = &["seven", "eight", "nine"];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(text_a, 1000, 200),
            thai_worded_line(text_b, 1000, 200),
            thai_worded_line(text_c, 3000, 200),
        ]
        .into(),
    };
    let line = |text: String, start| LyricsLine {
        start: Duration::from_millis(start),
        end: None,
        text,
        romanized: None,
        words: None,
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    let guide = Lyrics::Synced {
        lines: vec![
            line(text_a.join(" "), 1000),
            line(text_b.join(" "), 1000),
            line(text_c.join(" "), 3000),
        ]
        .into(),
    };

    assert_eq!(
        guide_hint(
            match &guide {
                Lyrics::Synced { lines } => lines,
                _ => unreachable!(),
            },
            0
        ),
        GuideHint::Bounded {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(3000),
        }
    );
    assert!(conform(&source, &guide).is_some());
}

#[test]
fn split_lines_sort_around_overlapping_source_line() {
    let split_words = vec![
        (0, 1000, "one "),
        (1000, 2000, "two "),
        (2000, 3000, "three "),
        (8000, 9000, "four "),
        (9000, 10000, "five "),
        (10000, 11000, "six"),
    ]
    .into_iter()
    .map(|(start, end, text)| LyricsWord {
        start: Duration::from_millis(start),
        end: Duration::from_millis(end),
        text: text.to_owned(),
    })
    .collect();
    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(11000)),
                text: "one two three four five six".to_owned(),
                romanized: None,
                words: Some(split_words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            thai_worded_line(&["source", "only"], 5000, 500),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("one two three", 0, 3000),
            guide_line("four five six", 8000, 11000),
        ]
        .into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(
        lines.iter().map(|line| line.start).collect::<Vec<_>>(),
        vec![
            Duration::ZERO,
            Duration::from_millis(5000),
            Duration::from_millis(8000),
        ]
    );
}

#[test]
fn distant_source_line_cannot_join_a_source_group_by_envelope() {
    let source = Lyrics::Synced {
        lines: vec![
            guide_line("one two three", 0, 1000),
            guide_line("four five six", 10000, 11000),
            guide_line("seven eight nine ten eleven twelve", 20000, 21000),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("one two three four five six", 0, 1000),
            guide_line("seven eight nine ten eleven twelve", 20000, 21000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
    let Lyrics::Synced { lines } = source else {
        unreachable!()
    };
    assert_eq!(lines[1].start, Duration::from_secs(10));
    assert_eq!(lines[1].text, "four five six");
}

#[test]
fn unfillable_internal_span_is_not_a_dp_match() {
    let nested = vec![
        LyricsWord {
            start: Duration::ZERO,
            end: Duration::from_millis(1000),
            text: "A ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(800),
            end: Duration::from_millis(1500),
            text: "B".to_owned(),
        },
    ];
    let mut source_lines = vec![LyricsLine {
        start: Duration::ZERO,
        end: Some(Duration::from_millis(1500)),
        text: "A B".to_owned(),
        romanized: None,
        words: Some(nested.clone()),
        secondary: Vec::new(),
        voice: Voice::Lead,
    }];
    let mut guide_lines = vec![guide_line("A X B", 0, 1500)];
    for index in 0..9 {
        let words = format!(
            "exact{index}a exact{index}b exact{index}c exact{index}d exact{index}e exact{index}f"
        );
        source_lines.push(guide_line(&words, 3000 + index * 2000, 4000 + index * 2000));
        guide_lines.push(guide_line(&words, 3000 + index * 2000, 4000 + index * 2000));
    }
    let source = Lyrics::Synced {
        lines: source_lines.into(),
    };
    let guide = Lyrics::Synced {
        lines: guide_lines.into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines[0].text, "A B");
    assert_eq!(lines[0].words.as_ref(), Some(&nested));
}

#[test]
fn omissions_cannot_pay_for_substitutions_in_another_anchor_gap() {
    let mut source_lines = vec![guide_line("start KEEP foo end", 0, 1000)];
    let mut guide_lines = vec![guide_line("start foo X end", 0, 1000)];
    for index in 0..9 {
        let text = format!(
            "exact{index}a exact{index}b exact{index}c exact{index}d exact{index}e exact{index}f"
        );
        source_lines.push(guide_line(&text, 2000 + index * 2000, 3000 + index * 2000));
        guide_lines.push(guide_line(&text, 2000 + index * 2000, 3000 + index * 2000));
    }
    let source = Lyrics::Synced {
        lines: source_lines.into(),
    };
    let guide = Lyrics::Synced {
        lines: guide_lines.into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines[0].text, "start KEEP foo end");
}

#[test]
fn unequal_local_substitution_gap_remains_representable() {
    // start New York end -> start NYC end
    assert!(represented(&[(0, 0), (3, 2)], 4, 3));
}

#[test]
fn temporal_tie_break_corrects_the_closer_repeated_source() {
    let repeated = "hello worl again tonight right now";
    let control = "one two three four five six";
    let source = Lyrics::Synced {
        lines: vec![
            guide_line(repeated, 0, 1000),
            guide_line(repeated, 50, 1050),
            guide_line(control, 2000, 3000),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("hello world again tonight right now", 0, 1000),
            guide_line(control, 2000, 3000),
        ]
        .into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines[0].start, Duration::ZERO);
    assert_eq!(lines[0].text, "hello world again tonight right now");
    assert_eq!(lines[1].start, Duration::from_millis(50));
    assert_eq!(lines[1].text, repeated);
}
