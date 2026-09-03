use std::time::Duration;

use super::*;
use crate::{Lyrics, LyricsLine, LyricsWord, Voice};

#[test]
fn combining_only_provider_chunks_are_not_silently_dropped() {
    let chunk = vec![LyricsWord {
        start: Duration::from_millis(150),
        end: Duration::from_millis(300),
        text: "้".to_owned(),
    }];
    let sung = timed("้", &chunk).unwrap();
    assert_eq!(sung.len(), 1);
    assert_eq!(sung[0].key, "้");
    assert_eq!(sung[0].start, Duration::from_millis(150));
    assert_eq!(sung[0].end, Duration::from_millis(300));

    let multi_combining = vec![LyricsWord {
        start: Duration::from_millis(200),
        end: Duration::from_millis(450),
        text: "ี่".to_owned(),
    }];
    let sung = timed("ี่", &multi_combining).unwrap();
    assert_eq!(sung.len(), 1);
    assert_eq!(sung[0].key, "ี่");
    assert_eq!(sung[0].start, Duration::from_millis(200));
    assert_eq!(sung[0].end, Duration::from_millis(450));
}

#[test]
fn provider_text_never_replaces_empty_canonical_text() {
    let words = vec![LyricsWord {
        start: Duration::ZERO,
        end: Duration::from_secs(1),
        text: "provider wording".to_owned(),
    }];

    assert!(timed("", &words).is_none());
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

    let fine_sung = timed("ไว้", &fine_chunks).unwrap();
    let single_sung = timed("ไว้", &single_chunk).unwrap();

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

#[test]
fn missing_whitespace_between_words_aligns_successfully() {
    let words: Vec<LyricsWord> = ["hello", "world", "one", "two", "three", "four"]
        .iter()
        .enumerate()
        .map(|(i, &w)| LyricsWord {
            start: Duration::from_millis(i as u64 * 300),
            end: Duration::from_millis(i as u64 * 300 + 280),
            text: w.to_owned(),
        })
        .collect();

    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(1800)),
                text: "hello world one two three four".to_owned(),
                romanized: None,
                words: Some(words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            LyricsLine {
                start: Duration::from_millis(2000),
                end: Some(Duration::from_millis(3800)),
                text: "stay with me all night long".to_owned(),
                romanized: None,
                words: Some(
                    ["stay", "with", "me", "all", "night", "long"]
                        .iter()
                        .enumerate()
                        .map(|(i, &w)| LyricsWord {
                            start: Duration::from_millis(2000 + i as u64 * 300),
                            end: Duration::from_millis(2000 + i as u64 * 300 + 280),
                            text: w.to_owned(),
                        })
                        .collect(),
                ),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("hello world one two three four", 0, 1800),
            guide_line("stay with me all night long", 2000, 3800),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("should align without token fusion");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "hello world one two three four");
    let recon: String = lines[0]
        .words
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| w.text.as_str())
        .collect();
    assert_eq!(recon, "hello world one two three four");
}

#[test]
fn punctuation_omitted_from_lyrics_word_chunks_aligns_successfully() {
    let words: Vec<LyricsWord> = ["hello", "world", "one", "two", "three", "four"]
        .iter()
        .enumerate()
        .map(|(i, &w)| LyricsWord {
            start: Duration::from_millis(i as u64 * 300),
            end: Duration::from_millis(i as u64 * 300 + 280),
            text: w.to_owned(),
        })
        .collect();

    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(1800)),
                text: "hello, world! one, two, three, four.".to_owned(),
                romanized: None,
                words: Some(words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            LyricsLine {
                start: Duration::from_millis(2000),
                end: Some(Duration::from_millis(3800)),
                text: "stay with me all night long.".to_owned(),
                romanized: None,
                words: Some(
                    ["stay", "with", "me", "all", "night", "long"]
                        .iter()
                        .enumerate()
                        .map(|(i, &w)| LyricsWord {
                            start: Duration::from_millis(2000 + i as u64 * 300),
                            end: Duration::from_millis(2000 + i as u64 * 300 + 280),
                            text: w.to_owned(),
                        })
                        .collect(),
                ),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("hello, world! one, two, three, four.", 0, 1800),
            guide_line("stay with me all night long.", 2000, 3800),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("aligns despite punctuation in line text");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "hello, world! one, two, three, four.");
    let recon: String = lines[0]
        .words
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| w.text.as_str())
        .collect();
    assert_eq!(recon, "hello, world! one, two, three, four.");
}

#[test]
fn realistic_nospace_thai_conforms_successfully() {
    let chunks = [
        "ไว้",
        "ที่",
        "นี่",
        "เธอ",
        "อยู่",
        "ที่",
        "ฉัน",
        "ยัง",
        "รอ",
        "อยู่",
        "ตรง",
        "นี้",
    ];
    let words: Vec<LyricsWord> = chunks
        .iter()
        .enumerate()
        .map(|(i, &w)| LyricsWord {
            start: Duration::from_millis(i as u64 * 200),
            end: Duration::from_millis(i as u64 * 200 + 180),
            text: w.to_owned(),
        })
        .collect();

    let thai_text = "ไว้ที่นี่เธออยู่ที่ฉันยังรออยู่ตรงนี้";
    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(2400)),
                text: thai_text.to_owned(),
                romanized: None,
                words: Some(words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            LyricsLine {
                start: Duration::from_millis(2500),
                end: Some(Duration::from_millis(5000)),
                text: "ฉันยังคงรอเธออยู่ที่เดิมเสมอ".to_owned(),
                romanized: None,
                words: None,
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(thai_text, 0, 2400),
            guide_line("ฉันยังคงรอเธออยู่ที่เดิมเสมอ", 2500, 5000),
        ]
        .into(),
    };

    let conformed = conform(&source, &guide).expect("realistic unspaced Thai conforms");
    let Lyrics::Synced { lines } = conformed else {
        panic!("expected synced lyrics");
    };

    assert_eq!(lines[0].text, thai_text);
    let recon: String = lines[0]
        .words
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| w.text.as_str())
        .collect();
    assert_eq!(recon, thai_text);
}

#[test]
fn timing_overlap_semantics_regressions() {
    // 1. Sequential chunks
    let seq = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(100),
            text: "one ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(100),
            end: Duration::from_millis(200),
            text: "two".to_owned(),
        },
    ];
    let sung_seq = timed("one two", &seq).unwrap();
    assert_eq!(sung_seq[0].start, Duration::from_millis(0));
    assert_eq!(sung_seq[0].end, Duration::from_millis(100));
    assert_eq!(sung_seq[1].start, Duration::from_millis(100));
    assert_eq!(sung_seq[1].end, Duration::from_millis(200));

    // 2. Overlapping chunks
    let over = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(150),
            text: "one ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(100),
            end: Duration::from_millis(250),
            text: "two".to_owned(),
        },
    ];
    let sung_over = timed("one two", &over).unwrap();
    assert_eq!(sung_over[0].start, Duration::from_millis(0));
    assert_eq!(sung_over[0].end, Duration::from_millis(150));
    assert_eq!(sung_over[1].start, Duration::from_millis(100));
    assert_eq!(sung_over[1].end, Duration::from_millis(250));

    // 3. Nested chunks (chunk 2 inside chunk 1)
    let nested = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "A".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(50),
            end: Duration::from_millis(200),
            text: "B".to_owned(),
        },
    ];
    let sung_nested = timed("AB", &nested).unwrap();
    assert_eq!(sung_nested.len(), 1);
    assert_eq!(sung_nested[0].start, Duration::from_millis(0));
    assert_eq!(sung_nested[0].end, Duration::from_millis(300));

    // 4. Same-start, different-end
    let same_start = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(400),
            text: "A".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(200),
            text: "B".to_owned(),
        },
    ];
    let sung_ss = timed("AB", &same_start).unwrap();
    assert_eq!(sung_ss.len(), 1);
    assert_eq!(sung_ss[0].start, Duration::from_millis(0));
    assert_eq!(sung_ss[0].end, Duration::from_millis(400));

    // 5. Different-start, same-end
    let same_end = vec![
        LyricsWord {
            start: Duration::from_millis(100),
            end: Duration::from_millis(400),
            text: "A".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(400),
            text: "B".to_owned(),
        },
    ];
    let sung_se = timed("AB", &same_end).unwrap();
    assert_eq!(sung_se.len(), 1);
    assert_eq!(sung_se[0].start, Duration::from_millis(0));
    assert_eq!(sung_se[0].end, Duration::from_millis(400));

    // 6. Thai combining mark spanning overlapping chunks
    let thai_combining = vec![
        LyricsWord {
            start: Duration::from_millis(100),
            end: Duration::from_millis(250),
            text: "ว".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(150),
            end: Duration::from_millis(300),
            text: "้".to_owned(),
        },
    ];
    let sung_thai = timed("ว้", &thai_combining).unwrap();
    assert_eq!(sung_thai.len(), 1);
    assert_eq!(sung_thai[0].key, "ว้");
    assert_eq!(sung_thai[0].start, Duration::from_millis(100));
    assert_eq!(sung_thai[0].end, Duration::from_millis(300));
}

#[test]
fn punctuation_symbol_only_lyric_lines_timed_successfully() {
    let sym_words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(500),
            text: "♪ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(500),
            end: Duration::from_millis(1000),
            text: "♪ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(1500),
            text: "♪".to_owned(),
        },
    ];
    let sung = timed("♪ ♪ ♪", &sym_words).unwrap();
    assert_eq!(sung.len(), 3);
    assert_eq!(sung[0].key, "♪");
    assert_eq!(sung[0].start, Duration::from_millis(0));
    assert_eq!(sung[0].end, Duration::from_millis(500));
    assert_eq!(sung[2].start, Duration::from_millis(1000));
    assert_eq!(sung[2].end, Duration::from_millis(1500));
}

#[test]
fn combining_marks_split_across_lyrics_word_boundaries_timed_accurately() {
    let split_chunks = vec![
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
    let sung = timed("ไว้", &split_chunks).unwrap();
    assert_eq!(sung.len(), 2);
    assert_eq!(sung[0].key, "ไ");
    assert_eq!(sung[1].key, "ว้");
    assert_eq!(sung[0].start, Duration::from_millis(0));
    assert_eq!(sung[0].end, Duration::from_millis(100));
    assert_eq!(sung[1].start, Duration::from_millis(100));
    assert_eq!(sung[1].end, Duration::from_millis(300));
}

#[test]
fn provider_word_text_differing_in_casing_and_ticks_aligns_successfully() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "Don't ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(300),
            end: Duration::from_millis(600),
            text: "STOP ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(600),
            end: Duration::from_millis(900),
            text: "the ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: "music ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "now ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "baby".to_owned(),
        },
    ];

    let sung = timed("dont stop the music now baby", &words).unwrap();
    let line_tokens = tokens("dont stop the music now baby");

    assert_eq!(sung.len(), line_tokens.len());
    for (st, lt) in sung.iter().zip(&line_tokens) {
        assert_eq!(st.key, lt.key);
    }
}

#[test]
fn partial_canonical_text_mapping_preserves_unmatched_cue_timing() {
    let line_text = "hello world again tonight";
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(500),
            text: "hello".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(500),
            end: Duration::from_millis(1000),
            text: "wurld".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(1500),
            text: "again".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(2000),
            text: "tonight".to_owned(),
        },
    ];

    let sung = timed(line_text, &words).unwrap();
    assert_eq!(sung.len(), 4);
    assert_eq!(sung[0].key, "hello");
    assert_eq!(sung[0].start, Duration::from_millis(0));
    assert_eq!(sung[0].end, Duration::from_millis(500));

    // "world" must receive the timing of "wurld" (500..1000ms), NOT borrow 0..500 or 1000..1500
    assert_eq!(sung[1].key, "world");
    assert_eq!(sung[1].start, Duration::from_millis(500));
    assert_eq!(sung[1].end, Duration::from_millis(1000));

    assert_eq!(sung[2].key, "again");
    assert_eq!(sung[2].start, Duration::from_millis(1000));
    assert_eq!(sung[2].end, Duration::from_millis(1500));

    assert_eq!(sung[3].key, "tonight");
    assert_eq!(sung[3].start, Duration::from_millis(1500));
    assert_eq!(sung[3].end, Duration::from_millis(2000));
}

#[test]
fn zero_meaningful_anchors_does_not_fabricate_canonical_mappings_and_fails_safely() {
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "xxx".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(300),
            end: Duration::from_millis(600),
            text: "yyy".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(600),
            end: Duration::from_millis(900),
            text: "zzz".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: "qqq".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "rrr".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "sss".to_owned(),
        },
    ];

    let line_text = "one two three four five six";
    assert!(timed(line_text, &words).is_none());

    let source = Lyrics::Synced {
        lines: vec![
            LyricsLine {
                start: Duration::ZERO,
                end: Some(Duration::from_millis(1800)),
                text: line_text.to_owned(),
                romanized: None,
                words: Some(words),
                secondary: Vec::new(),
                voice: Voice::Lead,
            },
            guide_line("other line", 2000, 4000),
        ]
        .into(),
    };

    let guide = Lyrics::Synced {
        lines: vec![
            guide_line(line_text, 0, 1800),
            guide_line("other line", 2000, 4000),
        ]
        .into(),
    };

    // conform must fail safely and return None when mapping cannot be established
    assert!(conform(&source, &guide).is_none());
}

#[test]
fn unmatched_run_with_more_cues_than_characters_does_not_lose_timing_cues() {
    // "hello world" has only 1 character (space) between "hello" and "world"
    // Provider has 3 extra cues in between
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(500),
            text: "hello".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(500),
            end: Duration::from_millis(700),
            text: "cue1".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(700),
            end: Duration::from_millis(900),
            text: "cue2".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1100),
            text: "cue3".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1100),
            end: Duration::from_millis(1600),
            text: "world".to_owned(),
        },
    ];

    let line_text = "hello world";
    // Cues cannot be mapped to whitespace without discarding them:
    // map_words_to_text must fail conservatively to avoid silent timing loss
    assert!(map_words_to_text(line_text, &words).is_none());

    assert!(timed(line_text, &words).is_none());
}

#[test]
fn unmappable_provider_cues_fail_canonical_mapping_conservatively() {
    let text = "hello world";
    let words = vec![
        LyricsWord {
            text: "hello".to_owned(),
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
        },
        LyricsWord {
            text: "cue1".to_owned(),
            start: Duration::from_millis(1000),
            end: Duration::from_millis(1200),
        },
        LyricsWord {
            text: "cue2".to_owned(),
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1400),
        },
        LyricsWord {
            text: "cue3".to_owned(),
            start: Duration::from_millis(1400),
            end: Duration::from_millis(1600),
        },
        LyricsWord {
            text: "world".to_owned(),
            start: Duration::from_millis(1600),
            end: Duration::from_millis(2600),
        },
    ];

    // map_words_to_text must return None because cue1..cue3 overlap no canonical token
    assert!(
        map_words_to_text(text, &words).is_none(),
        "canonical mapping must reject cues that do not overlap canonical tokens"
    );

    // Canonical timing stays unavailable; provider wording never replaces line text.
    let line = LyricsLine {
        start: Duration::from_millis(0),
        end: Some(Duration::from_millis(2600)),
        text: text.to_owned(),
        romanized: None,
        words: Some(words),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    assert!(timed(&line.text, line.words.as_ref().unwrap()).is_none());
}

#[test]
fn complexity_protection_rejects_oversized_source_before_sung() {
    // Create an oversized lyrics with more words than LIMIT
    let mut huge_words = Vec::new();
    for i in 0..(LIMIT + 10) {
        huge_words.push(LyricsWord {
            text: format!("w{i}"),
            start: Duration::from_millis(i as u64 * 100),
            end: Duration::from_millis((i as u64 + 1) * 100),
        });
    }
    let source = Lyrics::Synced {
        lines: vec![LyricsLine {
            start: Duration::ZERO,
            end: Some(Duration::from_millis(100000)),
            text: "huge text".to_owned(),
            romanized: None,
            words: Some(huge_words),
            secondary: Vec::new(),
            voice: Voice::Lead,
        }]
        .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![guide_line("huge text", 0, 100000)].into(),
    };

    // Fast complexity check rejects immediately
    assert!(conform(&source, &guide).is_none());
}

#[test]
fn lexical_token_limit_is_independent_of_cue_count() {
    let text = "界".repeat(LIMIT + 1);
    let lines: Vec<_> = (0..LEAST)
        .map(|index| guide_line(&text, index as u64 * 1000, index as u64 * 1000 + 900))
        .collect();
    let source = Lyrics::Synced {
        lines: lines.clone().into(),
    };
    let guide = Lyrics::Synced {
        lines: lines.into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn extra_edge_cues_do_not_claim_canonical_text() {
    let leading = vec![
        LyricsWord {
            start: Duration::from_secs(0),
            end: Duration::from_secs(4),
            text: "intro".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(5),
            end: Duration::from_secs(6),
            text: "hello ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(6),
            end: Duration::from_secs(7),
            text: "world".to_owned(),
        },
    ];
    let mut trailing = leading[1..].to_vec();
    trailing.push(LyricsWord {
        start: Duration::from_secs(7),
        end: Duration::from_secs(8),
        text: "outro".to_owned(),
    });

    assert!(timed("hello world", &leading).is_none());
    assert!(timed("hello world", &trailing).is_none());
}

#[test]
fn oversubscribed_internal_gap_is_unsafe() {
    let words = vec![
        LyricsWord {
            start: Duration::from_secs(0),
            end: Duration::from_secs(1),
            text: "hello".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(1),
            end: Duration::from_secs(2),
            text: "wrong-one".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(2),
            end: Duration::from_secs(3),
            text: "wrong-two".to_owned(),
        },
        LyricsWord {
            start: Duration::from_secs(3),
            end: Duration::from_secs(4),
            text: "world".to_owned(),
        },
    ];

    assert!(timed("hello brave world", &words).is_none());
}

#[test]
fn lexical_tokens_not_cue_count_meet_least_gate() {
    let first = LyricsLine {
        start: Duration::ZERO,
        end: Some(Duration::from_millis(1800)),
        text: "one two three four five six".to_owned(),
        romanized: None,
        words: Some(vec![
            LyricsWord {
                start: Duration::ZERO,
                end: Duration::from_millis(900),
                text: "one two three ".to_owned(),
            },
            LyricsWord {
                start: Duration::from_millis(900),
                end: Duration::from_millis(1800),
                text: "four five six".to_owned(),
            },
        ]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };
    let second = thai_worded_line(&["seven", "eight"], 2000, 300);
    let source = Lyrics::Synced {
        lines: vec![first, second].into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![
            guide_line("one two three four five six", 0, 1800),
            guide_line("seven eight", 2000, 3000),
        ]
        .into(),
    };

    assert!(conform(&source, &guide).is_some());
}

#[test]
fn many_unsafe_source_lines_stop_before_group_search() {
    let source = Lyrics::Synced {
        lines: (0..256)
            .map(|index| LyricsLine {
                start: Duration::from_millis(index * 1000),
                end: Some(Duration::from_millis(index * 1000 + 900)),
                text: format!("canonical line {index}"),
                romanized: None,
                words: Some(vec![LyricsWord {
                    start: Duration::from_millis(index * 1000),
                    end: Duration::from_millis(index * 1000 + 900),
                    text: "unrelated".to_owned(),
                }]),
                secondary: Vec::new(),
                voice: Voice::Lead,
            })
            .collect::<Vec<_>>()
            .into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![guide_line("one two three four five six", 0, 1000)].into(),
    };

    assert!(conform(&source, &guide).is_none());
}

#[test]
fn twenty_one_exact_lines_fit_work_budget_after_temporal_pruning() {
    let lines: Vec<_> = (0..21)
        .map(|index| {
            guide_line(
                &format!(
                    "line{index}a line{index}b line{index}c line{index}d line{index}e line{index}f"
                ),
                index * 2000,
                index * 2000 + 1000,
            )
        })
        .collect();
    let source = Lyrics::Synced {
        lines: lines.clone().into(),
    };
    let guide = Lyrics::Synced {
        lines: lines.into(),
    };

    let Lyrics::Synced { lines } = conform(&source, &guide).unwrap() else {
        unreachable!()
    };
    assert_eq!(lines.len(), 21);
}

#[test]
fn oversized_canonical_domain_rejects_before_provider_anchor_search() {
    let text = (0..=LIMIT)
        .map(|index| format!("canonical{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let cues: Vec<_> = (0..LIMIT)
        .map(|index| LyricsWord {
            start: Duration::from_millis(index as u64),
            end: Duration::from_millis(index as u64 + 1),
            text: format!("unrelated{index}"),
        })
        .collect();

    assert!(map_words_to_text(&text, &cues).is_none());
}

#[test]
fn cumulative_token_limits_cover_source_and_guide_construction() {
    let dense = "界".repeat(1000);
    let many_dense_lines = || {
        (0..10)
            .map(|index| guide_line(&dense, index * 2000, index * 2000 + 1000))
            .collect::<Vec<_>>()
    };
    let small = vec![guide_line("one two three four five six", 0, 1000)];

    assert!(
        conform(
            &Lyrics::Synced {
                lines: many_dense_lines().into()
            },
            &Lyrics::Synced {
                lines: small.clone().into()
            }
        )
        .is_none()
    );
    assert!(
        conform(
            &Lyrics::Synced {
                lines: small.into()
            },
            &Lyrics::Synced {
                lines: many_dense_lines().into()
            }
        )
        .is_none()
    );
}

#[test]
fn bidirectional_canonical_coverage_rejects_uncovered_extra_token() {
    let text = "one two EXTRA three four five six";
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "one".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(300),
            end: Duration::from_millis(600),
            text: "two".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(600),
            end: Duration::from_millis(900),
            text: "three".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: "four".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "five".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "six".to_owned(),
        },
    ];

    // EXTRA must never borrow timing from two or three; uncovered canonical slot fails safely
    assert!(map_words_to_text(text, &words).is_none());
    assert!(timed(text, &words).is_none());

    // Coarse provider cues spanning multiple tokens legitimately provide proportional timing
    let coarse = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(900),
            text: "one two three".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: "four".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "five".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "six".to_owned(),
        },
    ];
    assert!(map_words_to_text("one two three four five six", &coarse).is_some());
    let sung = timed("one two three four five six", &coarse).unwrap();
    assert_eq!(sung.len(), 6);
    assert_eq!(sung[0].key, "one");
    assert_eq!(sung[1].key, "two");
    assert_eq!(sung[2].key, "three");
}

#[test]
fn source_lexical_limit_exhaustion_aborts_conform_despite_passing_thresholds() {
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let source_a = thai_worded_line(anchor, 0, 300);
    let big_line_text = (0..3000)
        .map(|i| format!("tok{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source_b = guide_line(&big_line_text, 3000, 6000);

    let source = Lyrics::Synced {
        lines: vec![source_a, source_b].into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![guide_line(&anchor.join(" "), 0, 1800)].into(),
    };

    // Even though source A matches guide perfectly and would pass MATCHED/KEPT/SEATED,
    // conform must return None because aggregate source lexical LIMIT is exceeded.
    assert!(conform(&source, &guide).is_none());
}

#[test]
fn canonical_anchor_matching_work_budget_terminates_high_prefix_cost_within_token_limit() {
    let prefix = "a".repeat(40);
    let text = (0..300)
        .map(|i| format!("{prefix}x{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let words: Vec<_> = (0..300)
        .map(|i| LyricsWord {
            start: Duration::from_millis(i as u64 * 10),
            end: Duration::from_millis((i as u64 + 1) * 10),
            text: format!("{prefix}y{i}"),
        })
        .collect();

    // 300 words and 300 tokens are well within LIMIT (3000), but prefix scan work budget >= CELLS
    // terminates mapping conservatively
    assert!(map_words_to_text(&text, &words).is_none());
}

#[test]
fn exact_concatenation_mixed_punctuation_cue_without_slot_fails_safely() {
    let text = "one two ! three four five six";
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(300),
            text: "one ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(300),
            end: Duration::from_millis(600),
            text: "two ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(600),
            end: Duration::from_millis(900),
            text: "!".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: " three ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "four ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "five ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1800),
            end: Duration::from_millis(2100),
            text: "six".to_owned(),
        },
    ];
    assert_eq!(
        words.iter().map(|w| w.text.as_str()).collect::<String>(),
        text
    );
    // Meaningful "!" cue has no canonical slot in mixed line; must fail conservatively
    assert!(map_words_to_text(text, &words).is_none());
    assert!(timed(text, &words).is_none());
}

#[test]
fn coarse_one_cue_to_many_slots_bidirectional_coverage_remains_valid() {
    let text = "alpha beta gamma delta epsilon zeta";
    let words = vec![
        LyricsWord {
            start: Duration::from_millis(0),
            end: Duration::from_millis(1000),
            text: "alpha beta gamma ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1000),
            end: Duration::from_millis(2000),
            text: "delta epsilon zeta".to_owned(),
        },
    ];
    assert_eq!(
        words.iter().map(|w| w.text.as_str()).collect::<String>(),
        text
    );
    assert!(map_words_to_text(text, &words).is_some());
    let sung = timed(text, &words).unwrap();
    assert_eq!(sung.len(), 6);
    assert_eq!(sung[0].key, "alpha");
    assert_eq!(sung[1].key, "beta");
    assert_eq!(sung[2].key, "gamma");
    assert_eq!(sung[3].key, "delta");
    assert_eq!(sung[4].key, "epsilon");
    assert_eq!(sung[5].key, "zeta");
}

#[test]
fn many_to_one_thai_combining_coverage_remains_valid() {
    let text = "ไว้ ที่ นี่ เธอ อยู่ ตรง";
    let words = vec![
        LyricsWord {
            start: Duration::ZERO,
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
            text: "้ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(300),
            end: Duration::from_millis(600),
            text: "ที่ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(600),
            end: Duration::from_millis(900),
            text: "นี่ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(900),
            end: Duration::from_millis(1200),
            text: "เธอ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1200),
            end: Duration::from_millis(1500),
            text: "อยู่ ".to_owned(),
        },
        LyricsWord {
            start: Duration::from_millis(1500),
            end: Duration::from_millis(1800),
            text: "ตรง".to_owned(),
        },
    ];
    assert_eq!(
        words.iter().map(|w| w.text.as_str()).collect::<String>(),
        text
    );
    assert!(map_words_to_text(text, &words).is_some());
    let sung = timed(text, &words).unwrap();
    assert_eq!(sung.len(), 12);
    assert_eq!(sung[0].key, "ไ");
    assert_eq!(sung[0].start, Duration::ZERO);
    assert_eq!(sung[0].end, Duration::from_millis(100));
    assert_eq!(sung[1].key, "ว้");
    assert_eq!(sung[1].start, Duration::from_millis(100));
    assert_eq!(sung[1].end, Duration::from_millis(300));
}

#[test]
fn unsafe_source_lines_lexical_tokens_consume_aggregate_limit_and_abort() {
    let anchor = &["one", "two", "three", "four", "five", "six"];
    let source_a = thai_worded_line(anchor, 0, 300);

    let text_b = (0..2000)
        .map(|i| format!("tokb{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source_b = LyricsLine {
        start: Duration::from_millis(3000),
        end: Some(Duration::from_millis(4000)),
        text: text_b,
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_millis(3000),
            end: Duration::from_millis(3500),
            text: "unrelated".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    let text_c = (0..2000)
        .map(|i| format!("tokc{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source_c = LyricsLine {
        start: Duration::from_millis(5000),
        end: Some(Duration::from_millis(6000)),
        text: text_c,
        romanized: None,
        words: Some(vec![LyricsWord {
            start: Duration::from_millis(5000),
            end: Duration::from_millis(5500),
            text: "unrelated".to_owned(),
        }]),
        secondary: Vec::new(),
        voice: Voice::Lead,
    };

    let source = Lyrics::Synced {
        lines: vec![source_a, source_b, source_c].into(),
    };
    let guide = Lyrics::Synced {
        lines: vec![guide_line(&anchor.join(" "), 0, 1800)].into(),
    };

    // Source A (6 tokens) matches guide, but B (2000 tokens) + C (2000 tokens) exceed aggregate LIMIT.
    // Even though B and C are Unsafe, their lexical tokens must count against aggregate LIMIT.
    assert!(conform(&source, &guide).is_none());
}

#[test]
fn candidate_evaluation_cells_exhaustion_aborts_conform_without_partial_result() {
    // 11 source lines, 11 guide lines, 20 unique lexical tokens per line.
    // All lines temporally overlap completely (0..10_000 ms).
    // Source line N matches guide line N.
    let make_line = |prefix: &str, line_idx: usize, worded: bool| {
        let tokens: Vec<String> = (0..20).map(|t| format!("{prefix}{line_idx}t{t}")).collect();
        let text = tokens.join(" ");
        let words = worded.then(|| {
            tokens
                .iter()
                .enumerate()
                .map(|(t, tok)| LyricsWord {
                    start: Duration::from_millis(t as u64 * 100),
                    end: Duration::from_millis((t as u64 + 1) * 100),
                    text: tok.clone(),
                })
                .collect()
        });
        LyricsLine {
            start: Duration::ZERO,
            end: Some(Duration::from_millis(10_000)),
            text,
            romanized: None,
            words,
            secondary: Vec::new(),
            voice: Voice::Lead,
        }
    };

    let source_lines: Vec<_> = (0..11).map(|i| make_line("tok", i, true)).collect();
    let guide_lines: Vec<_> = (0..11).map(|i| make_line("tok", i, false)).collect();

    let source = Lyrics::Synced {
        lines: source_lines.into(),
    };
    let guide = Lyrics::Synced {
        lines: guide_lines.into(),
    };

    // Fully overlapping candidate graph exhausts CELLS before full exploration.
    // It must NOT emit a partial 10/11 conform; CELLS exhaustion must abort conform immediately.
    assert!(conform(&source, &guide).is_none());
}

#[test]
fn mixed_wide_segment_enforces_exact_token_limit() {
    let text = format!("가{}", "a".repeat(10));
    // Single word segment with 1 Hangul + 10 Latin graphemes = 11 lexical slots.
    assert_eq!(tokens(&text).len(), 11);
    // Exactly limit slots are allowed
    assert_eq!(bounded_tokens_to(&text, 11).unwrap().len(), 11);
    // Attempting slot limit + 1 returns None immediately from bounded_tokens_to
    assert!(bounded_tokens_to(&text, 10).is_none());
    assert!(bounded_tokens_to(&text, 5).is_none());
    assert!(bounded_tokens_to(&text, 1).is_none());
    assert!(bounded_tokens_to(&text, 0).is_none());
}
