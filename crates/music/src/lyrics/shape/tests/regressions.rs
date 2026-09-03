use std::time::Duration;

use super::*;
use crate::{Lyrics, LyricsLine, LyricsWord, Voice};

#[test]
fn minimal_regression_conform_preserves_source_only_middle_line() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let english_b = &["never", "let", "you", "go"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            thai_worded_line(english_b, 2000, 250),
            thai_worded_line(thai_c, 3500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("meets match/seated thresholds");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, thai_a.join(" "));
    assert_eq!(lines[1].text, english_b.join(" "));
    assert_eq!(lines[2].text, thai_c.join(" "));
    assert!(lines[1].words.as_ref().is_some_and(|w| !w.is_empty()));
}

#[test]
fn source_only_intro_line_is_preserved_chronologically() {
    let intro = &["three", "two", "one", "go"];
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(intro, 0, 250),
            thai_worded_line(thai_a, 1500, 300),
            thai_worded_line(thai_c, 3500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 1500, 3500),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, intro.join(" "));
    assert_eq!(lines[1].text, thai_a.join(" "));
    assert_eq!(lines[2].text, thai_c.join(" "));
}

#[test]
fn source_only_outro_line_is_preserved_chronologically() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];
    let outro = &["goodbye", "my", "friend", "forever"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            thai_worded_line(thai_c, 2000, 300),
            thai_worded_line(outro, 4000, 250),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 2000, 4000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, thai_a.join(" "));
    assert_eq!(lines[1].text, thai_c.join(" "));
    assert_eq!(lines[2].text, outro.join(" "));
}

#[test]
fn multiple_consecutive_source_only_lines_are_preserved() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let interlude_1 = &["hold", "on", "now"];
    let interlude_2 = &["stay", "with", "me"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            thai_worded_line(interlude_1, 2000, 250),
            thai_worded_line(interlude_2, 3000, 250),
            thai_worded_line(thai_c, 4500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 4500, 6500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0].text, thai_a.join(" "));
    assert_eq!(lines[1].text, interlude_1.join(" "));
    assert_eq!(lines[2].text, interlude_2.join(" "));
    assert_eq!(lines[3].text, thai_c.join(" "));
}

#[test]
fn mixed_thai_english_source_only_line_is_preserved() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let mixed = &["baby", "รัก", "หมด", "ใจ"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            thai_worded_line(mixed, 2000, 250),
            thai_worded_line(thai_c, 3500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1].text, mixed.join(" "));
}

#[test]
fn source_only_line_with_word_timing_is_preserved_intact() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let english_b = &["word", "timing", "here"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let english_line = thai_worded_line(english_b, 2000, 300);
    let orig_words = english_line.words.clone();

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            english_line,
            thai_worded_line(thai_c, 3500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1].words, orig_words);
}

#[test]
fn source_only_line_without_word_timing_is_preserved_intact() {
    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let line_synced_only = guide_line("only line synced text", 2000, 3200);

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(thai_a, 0, 300),
            line_synced_only,
            thai_worded_line(thai_c, 3500, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1].text, "only line synced text");
    assert!(lines[1].words.is_none());
}

#[test]
fn matched_lines_preserve_voice_secondary_and_provider_romanized() {
    use crate::{LyricsLane, RomanizedText, WritingSystem};

    let thai_a = &["ไว้", "ที่", "นี่", "เธอ", "อยู่"];
    let thai_c = &["ฉัน", "ยัง", "รอ", "อยู่", "ตรง"];

    let mut line_a = thai_worded_line(thai_a, 0, 300);
    line_a.voice = Voice::Counter;
    line_a.romanized = Some(RomanizedText {
        text: "wai thi ni thoe yu".to_owned(),
        writing_system: WritingSystem::Japanese,
    });
    line_a.secondary = vec![LyricsLane {
        start: Duration::from_millis(100),
        end: Some(Duration::from_millis(1500)),
        text: "(harmony)".to_owned(),
        romanized: None,
        words: None,
    }];

    let source = Lyrics::Synced {
        lines: vec![line_a, thai_worded_line(thai_c, 3500, 300)].into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&thai_a.join(" "), 0, 2000),
            guide_line(&thai_c.join(" "), 3500, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("conforms successfully");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines[0].voice, Voice::Counter);
    assert_eq!(lines[0].secondary.len(), 1);
    assert_eq!(lines[0].secondary[0].text, "(harmony)");
    assert_eq!(
        lines[0].romanized.as_ref().map(|r| r.writing_system),
        Some(WritingSystem::Japanese)
    );
}

#[test]
fn differing_source_and_guide_text_does_not_blindly_copy_source_romanization() {
    use crate::{RomanizedText, WritingSystem};

    let words_a = &["one", "two", "three", "four"];
    let words_b = &["five", "six", "seven", "eight"];

    let mut line_a = thai_worded_line(words_a, 0, 300);
    line_a.romanized = Some(RomanizedText {
        text: "differing old romanization".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let source = Lyrics::Synced {
        lines: vec![line_a, thai_worded_line(words_b, 2000, 300)].into(),
    };

    // Guide text substitutes one interior token.
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("one two changed four", 0, 2000),
            guide_line(&words_b.join(" "), 2000, 4000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Because guide text differed from source text, source romanization was NOT copied
    assert_ne!(
        lines[0].romanized.as_ref().map(|r| r.text.as_str()),
        Some("differing old romanization")
    );
}

#[test]
fn normal_lrc_normalization_still_regenerates_romanization_as_before() {
    let mut lines = vec![
        LyricsLine {
            start: Duration::from_millis(0),
            end: Some(Duration::from_millis(1000)),
            text: "こんにちは".to_owned(),
            romanized: None,
            words: None,
            secondary: Vec::new(),
            voice: Voice::Lead,
        },
        LyricsLine {
            start: Duration::from_millis(1000),
            end: Some(Duration::from_millis(2000)),
            text: "ありがとう".to_owned(),
            romanized: None,
            words: None,
            secondary: Vec::new(),
            voice: Voice::Lead,
        },
    ];

    crate::lyrics::lrc::normalize(&mut lines);

    // Upstream romanize::apply still unconditionally regenerates romanization for Japanese text
    assert!(lines[0].romanized.is_some());
    assert_eq!(lines[0].romanized.as_ref().unwrap().text, "konnichiwa");
    assert!(lines[1].romanized.is_some());
    assert_eq!(lines[1].romanized.as_ref().unwrap().text, "arigatou");
}

#[test]
fn source_only_conform_preservation_does_not_accidentally_attach_stale_romanization() {
    let words_a = &["one", "two", "three", "four"];
    let words_b = &["interlude", "source", "only"];
    let words_c = &["five", "six", "seven", "eight"];

    let mut line_b = thai_worded_line(words_b, 1500, 300);
    line_b.romanized = None;

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_a, 0, 300),
            line_b,
            thai_worded_line(words_c, 3000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_a.join(" "), 0, 1500),
            guide_line(&words_c.join(" "), 3000, 4500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1].text, words_b.join(" "));
    assert!(lines[1].romanized.is_none());
}

#[test]
fn incidental_token_pairing_does_not_delete_source_only_line() {
    let part_a = &["one", "two", "three", "four"];
    let part_b = &["love", "something", "completely", "different"];
    let part_c = &["love", "five", "six", "seven"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(part_a, 0, 300),
            thai_worded_line(part_b, 2000, 300),
            thai_worded_line(part_c, 4000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&part_a.join(" "), 0, 1500),
            guide_line(&part_c.join(" "), 4000, 5500),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // B must survive intact as a source-only line!
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, part_a.join(" "));
    assert_eq!(lines[1].text, part_b.join(" "));
    assert_eq!(lines[2].text, part_c.join(" "));
}

#[test]
fn two_source_only_lines_with_same_start_retain_respective_romanization() {
    use crate::{RomanizedText, WritingSystem};

    let words_a = &["alpha", "one"];
    let words_b = &["beta", "two"];
    let words_c = &[
        "gamma", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    let words_d = &["delta", "extra", "context", "words", "here"];

    let mut line_a = thai_worded_line(words_a, 4000, 300);
    line_a.romanized = Some(RomanizedText {
        text: "rom_alpha".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let mut line_b = thai_worded_line(words_b, 4000, 300);
    line_b.romanized = Some(RomanizedText {
        text: "rom_beta".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_c, 0, 300),
            thai_worded_line(words_d, 6000, 300),
            line_a,
            line_b,
        ]
        .into(),
    };

    // Guide matches line_c and line_d (both lines anchored: 100% >= 80% SEATED)
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_c.join(" "), 0, 3000),
            guide_line(&words_d.join(" "), 6000, 8000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    let line_alpha = lines.iter().find(|l| l.text == "alpha one").unwrap();
    let line_beta = lines.iter().find(|l| l.text == "beta two").unwrap();
    assert_eq!(
        line_alpha.romanized.as_ref().map(|r| r.text.as_str()),
        Some("rom_alpha")
    );
    assert_eq!(
        line_beta.romanized.as_ref().map(|r| r.text.as_str()),
        Some("rom_beta")
    );
}

#[test]
fn same_start_repeated_text_does_not_attach_one_provider_romanization_twice() {
    use crate::{RomanizedText, WritingSystem};

    let words_rep = &["repeat", "text"];
    let words_c = &[
        "gamma", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    let words_d = &["delta", "extra", "context", "words", "here"];

    let mut line_1 = thai_worded_line(words_rep, 4000, 300);
    line_1.romanized = Some(RomanizedText {
        text: "rom_repeat".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let mut line_2 = thai_worded_line(words_rep, 4000, 300);
    line_2.romanized = None;

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_c, 0, 300),
            thai_worded_line(words_d, 6000, 300),
            line_1,
            line_2,
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_c.join(" "), 0, 3000),
            guide_line(&words_d.join(" "), 6000, 8000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    let matching_lines: Vec<_> = lines.iter().filter(|l| l.text == "repeat text").collect();
    assert_eq!(matching_lines.len(), 2);
    // Exactly one line attaches the preservation record, second line does not attach it twice
    let attached_count = matching_lines
        .iter()
        .filter(|l| l.romanized.is_some())
        .count();
    assert_eq!(attached_count, 1);
}

#[test]
fn overlapping_matched_and_source_only_lines_retain_correct_romanization() {
    use crate::{RomanizedText, WritingSystem};

    let words_m = &[
        "matched", "line", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_s = &["source", "only"];
    let words_c = &["trailing", "guide", "context", "for", "conform"];

    let mut line_m = thai_worded_line(words_m, 1000, 300);
    line_m.romanized = Some(RomanizedText {
        text: "rom_matched".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let mut line_s = thai_worded_line(words_s, 1000, 300);
    line_s.romanized = Some(RomanizedText {
        text: "rom_source".to_owned(),
        writing_system: WritingSystem::Cyrillic,
    });

    let source = Lyrics::Synced {
        lines: vec![line_m, line_s, thai_worded_line(words_c, 5000, 300)].into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_m.join(" "), 1000, 4000),
            guide_line(&words_c.join(" "), 5000, 7000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    let out_m = lines.iter().find(|l| l.text == words_m.join(" ")).unwrap();
    let out_s = lines.iter().find(|l| l.text == "source only").unwrap();
    assert_eq!(
        out_m.romanized.as_ref().map(|r| r.text.as_str()),
        Some("rom_matched")
    );
    assert_eq!(
        out_s.romanized.as_ref().map(|r| r.text.as_str()),
        Some("rom_source")
    );
}

#[test]
fn single_incidental_boundary_token_as_sole_guide_contributor_does_not_delete_source_line() {
    let words_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_solo = &["love", "something", "completely", "different"];
    let words_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_1, 0, 300),
            thai_worded_line(words_solo, 3000, 300),
            thai_worded_line(words_2, 6000, 300),
        ]
        .into(),
    };

    // Guide has 3 lines:
    // Line 0 matches words_1 exactly
    // Line 1 is "love" (single token, matching only the first word of words_solo)
    // Line 2 matches words_2 exactly
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
fn boundary_matching_does_not_delete_unmatched_source_interior_run() {
    let words_surround_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let words_omitted = &["start", "keep", "all", "these", "words", "end"];
    let words_surround_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(words_surround_1, 0, 300),
            thai_worded_line(words_omitted, 3000, 300),
            thai_worded_line(words_surround_2, 6000, 300),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&words_surround_1.join(" "), 0, 2500),
            guide_line("start end", 3000, 4500),
            guide_line(&words_surround_2.join(" "), 6000, 8000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn crossing_tokens_do_not_produce_non_monotonic_timings() {
    let words_surround_1 = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ];
    let source_crossing = vec![
        LyricsWord {
            text: "start".to_owned(),
            start: Duration::from_millis(3000),
            end: Duration::from_millis(3200),
        },
        LyricsWord {
            text: "x".to_owned(),
            start: Duration::from_millis(3200),
            end: Duration::from_millis(3400),
        },
        LyricsWord {
            text: "y".to_owned(),
            start: Duration::from_millis(3400),
            end: Duration::from_millis(3600),
        },
        LyricsWord {
            text: "end".to_owned(),
            start: Duration::from_millis(3600),
            end: Duration::from_millis(3800),
        },
    ];
    let line_crossing = LyricsLine {
        start: Duration::from_millis(3000),
        end: Some(Duration::from_millis(3800)),
        text: "start x y end".to_owned(),
        romanized: None,
        words: Some(source_crossing),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    let words_surround_2 = &["nine", "ten", "eleven", "twelve", "thirteen", "fourteen"];

    let mut source_lines = vec![
        thai_worded_line(words_surround_1, 0, 300),
        line_crossing,
        thai_worded_line(words_surround_2, 6000, 300),
    ];
    let mut guide_lines = vec![
        guide_line(&words_surround_1.join(" "), 0, 2500),
        guide_line("start y x end", 3000, 3800),
        guide_line(&words_surround_2.join(" "), 6000, 8000),
    ];
    for index in 0..8 {
        let text = format!(
            "control{index}a control{index}b control{index}c control{index}d control{index}e control{index}f"
        );
        source_lines.push(guide_line(&text, 9000 + index * 2000, 10000 + index * 2000));
        guide_lines.push(guide_line(&text, 9000 + index * 2000, 10000 + index * 2000));
    }
    let source = Lyrics::Synced {
        lines: source_lines.into(),
    };
    let guide = Lyrics::Synced {
        lines: guide_lines.into(),
    };

    let conformed = conform(&source, &guide).expect("should conform");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    // Check monotonicity of words in reconstructed line
    let conformed_line = lines.iter().find(|l| l.text == "start x y end").unwrap();
    let words = conformed_line.words.as_ref().unwrap();
    for window in words.windows(2) {
        assert!(
            window[0].start <= window[1].start,
            "word timings must remain monotonic: {:?} > {:?}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn trailing_guide_injection_does_not_replace_source_line() {
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let original = &["alpha", "beta"];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(anchor, 0, 300),
            thai_worded_line(original, 3000, 300),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&anchor.join(" "), 0, 2500),
            guide_line("alpha beta injected", 3000, 4500),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn crossing_provider_timing_preserves_source_line() {
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let crossing = vec![
        LyricsWord {
            start: Duration::from_millis(3000),
            end: Duration::from_millis(4000),
            text: "alpha ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(2000),
            end: Duration::from_millis(3000),
            text: "beta".to_owned(),
        },
    ];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(anchor, 0, 300),
            LyricsLine {
                start: Duration::from_millis(2000),
                end: Some(Duration::from_millis(4000)),
                text: "alpha beta".to_owned(),
                romanized: None,
                words: Some(crossing.clone()),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(&anchor.join(" "), 0, 1900),
            guide_line("alpha beta", 2000, 4000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn overlapping_provider_spans_are_preserved() {
    let overlap = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
            text: "A ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(800),
            end: Duration::from_millis(1500),
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
                words: Some(overlap.clone()),
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

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines[0].words.as_ref(), Some(&overlap));
}

#[test]
fn changed_guide_uses_its_romanization() {
    use crate::{RomanizedText, WritingSystem};

    let source_text = &["ฉัน", "รัก", "เธอ", "หมด", "ทั้ง", "ใจ"];
    let anchor = &["หนึ่ง", "สอง", "สาม", "สี่", "ห้า", "หก"];
    let source = Lyrics::Synced {
        lines: vec![
            thai_worded_line(source_text, 0, 300),
            thai_worded_line(anchor, 3000, 300),
        ]
        .into(),
    };
    let mut changed = guide_line("ฉัน รัก เธอ, หมด ทั้ง ใจ", 0, 2200);
    changed.romanized = Some(RomanizedText {
        text: "chan rak thoe mot thang chai".to_owned(),
        writing_system: WritingSystem::Other,
    });
    let guide = Lyrics::Synced {
        lines: vec![changed, guide_line(&anchor.join(" "), 3000, 5200)].into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(
        lines[0].romanized.as_ref().map(|text| text.text.as_str()),
        Some("chan rak thoe mot thang chai")
    );
}

#[test]
fn nested_provider_spans_are_preserved() {
    let nested = vec![
        LyricsWord {
            start: Duration::ZERO,
            end: Duration::from_millis(2000),
            text: "A ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(800),
            end: Duration::from_millis(1500),
            text: "B".to_owned(),
        },
    ];
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(2000)),
                text: "A B".to_owned(),
                romanized: None,
                words: Some(nested.clone()),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            thai_worded_line(anchor, 3000, 300),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("A B", 0, 2000),
            guide_line(&anchor.join(" "), 3000, 5200),
        ]
        .into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines[0].words.as_ref(), Some(&nested));
}

#[test]
fn post_conform_parenthetical_background_separated_from_guide() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
            text: "lead".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(2000),
            text: "words".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(2000),
            end: Duration::from_millis(3000),
            text: "echo".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(3000),
            end: Duration::from_millis(4000),
            text: "after".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(4000),
            end: Duration::from_millis(5000),
            text: "words".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(5000),
            end: Duration::from_millis(6000),
            text: "tonight".to_owned(),
        },
    ];

    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(6000)),
                text: "lead words echo after words tonight".to_owned(),
                romanized: None,
                words: Some(words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            guide_line("control line extra tokens here today", 7000, 9000),
        ]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("lead words (echo) after words tonight", 0, 6000),
            guide_line("control line extra tokens here today", 7000, 9000),
        ]
        .into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).expect("conforms successfully") else {
        unreachable!()
    };

    assert_eq!(lines[0].text, "lead words after words tonight");
    let primary_words = lines[0].words.as_ref().unwrap();
    assert_eq!(primary_words.len(), 5);
    assert_eq!(primary_words[0].text, "lead ");
    assert_eq!(primary_words[1].text, "words ");
    assert_eq!(primary_words[2].text, "after ");
    assert_eq!(primary_words[3].text, "words ");
    assert_eq!(primary_words[4].text, "tonight");

    assert_eq!(lines[0].secondary.len(), 1);
    assert_eq!(lines[0].secondary[0].text, "(echo)");
    assert_eq!(lines[0].secondary[0].start, Duration::from_millis(2000));
    assert_eq!(lines[0].secondary[0].end, Some(Duration::from_millis(3000)));

    let sec_words = lines[0].secondary[0].words.as_ref().unwrap();
    assert_eq!(sec_words.len(), 1);
    assert_eq!(sec_words[0].text, "(echo)");
    assert_eq!(sec_words[0].start, Duration::from_millis(2000));
    assert_eq!(sec_words[0].end, Duration::from_millis(3000));
}
