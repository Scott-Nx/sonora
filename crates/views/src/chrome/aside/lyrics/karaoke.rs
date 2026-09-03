use std::ops::Range;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    App, Bounds, ContentMask, DecorationRun, Div, Element, GlobalElementId, Hsla,
    InspectorElementId, LayoutId, LineLayout, Pixels, Point, ShapedLine, SharedString, Style,
    TextAlign, Window, div, point, px,
};
use music::Voice;
use state::RomanizationScripts;
use ui::{Motion, Motioned as _, mix};

use super::layout::Wrapped;
use super::{AHEAD, PAST, SWEEP_LEAST};

#[derive(Clone, Copy)]
pub(in crate::chrome::aside) struct Sung {
    pub(in crate::chrome::aside) karaoke: bool,
    pub(in crate::chrome::aside) lane: Pixels,
    pub(in crate::chrome::aside) scripts: Option<RomanizationScripts>,
    pub(in crate::chrome::aside) theme: ui::Theme,
    pub(in crate::chrome::aside) karaoke_tint: Hsla,
    pub(in crate::chrome::aside) lift: f32,
    pub(in crate::chrome::aside) from: Point<f32>,
}

pub(in crate::chrome::aside) struct SecondaryLaneLook<'a> {
    pub(in crate::chrome::aside) dimming: Option<u64>,
    pub(in crate::chrome::aside) voice: Voice,
    pub(in crate::chrome::aside) sung: Sung,
    pub(in crate::chrome::aside) plan: Option<&'a Wrapped>,
}

pub(in crate::chrome::aside) fn fixed_lyrics_lane(
    rows: &[SharedString],
    voice: Voice,
    sung: Sung,
) -> Div {
    div()
        .flex()
        .flex_col()
        .children(rows.iter().map(move |row| {
            lifted(
                div()
                    .w_full()
                    .when(!voice.lead(), |this| this.text_right())
                    .child(row.clone()),
                sung,
            )
        }))
}

pub(in crate::chrome::aside) fn karaoke_lane(
    plan: &Wrapped,
    words: &[music::LyricsWord],
    position: Duration,
    voice: Voice,
    sung: Sung,
) -> Div {
    let windows = (0..words.len())
        .map(|word| karaoke_window(words, word))
        .collect::<Vec<_>>();
    let lit = |shape: ShapedLine, reveal: Vec<RevealRange>| {
        KaraokeText::new(shape, reveal, sung.karaoke_tint)
    };

    match plan.rows.is_empty() {
        false => div()
            .flex()
            .flex_col()
            .text_left()
            .children((0..plan.rows.len()).map(|row| {
                let reveal = revealed(plan, plan.rows[row].clone(), &windows, position);
                lifted(
                    div()
                        .flex()
                        .when(!voice.lead(), |this| this.justify_end())
                        .child(lit(plan.shapes[row].clone(), reveal)),
                    sung,
                )
            })),
        true => div()
            .flex()
            .flex_wrap()
            .text_left()
            .when(!voice.lead(), |this| this.justify_end())
            .children((0..plan.units.len()).map(|index| {
                let reveal = revealed(plan, index..index + 1, &windows, position);
                lit(plan.shapes[index].clone(), reveal)
            })),
    }
}

pub(in crate::chrome::aside) struct KaraokeText {
    line: ShapedLine,
    ranges: Vec<RevealRange>,
    foreground: Hsla,
}

pub(in crate::chrome::aside) struct KaraokeTextLayout {
    line_height: Pixels,
    base: DecorationRun,
    foreground: DecorationRun,
}

impl KaraokeText {
    pub(in crate::chrome::aside) fn new(
        line: ShapedLine,
        ranges: Vec<RevealRange>,
        foreground: Hsla,
    ) -> Self {
        Self {
            line,
            ranges,
            foreground,
        }
    }
}

impl IntoElement for KaraokeText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for KaraokeText {
    type RequestLayoutState = KaraokeTextLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let base_run = style.to_run(self.line.len());
        let line_height = style.line_height_in_pixels(window.rem_size());

        let mut layout_style = Style {
            flex_shrink: 0.,
            ..Style::default()
        };
        layout_style.size.width = self.line.width().into();
        layout_style.size.height = line_height.into();

        let base = DecorationRun {
            len: self.line.len() as u32,
            color: base_run.color,
            background_color: base_run.background_color,
            underline: base_run.underline,
            strikethrough: base_run.strikethrough,
        };
        let foreground = DecorationRun {
            len: self.line.len() as u32,
            color: self.foreground,
            background_color: base_run.background_color,
            underline: base_run.underline,
            strikethrough: base_run.strikethrough,
        };

        (
            window.request_layout(layout_style, [], cx),
            KaraokeTextLayout {
                line_height,
                base,
                foreground,
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        LineLayout::paint(
            &self.line,
            bounds.origin,
            layout.line_height,
            TextAlign::Left,
            None,
            std::slice::from_ref(&layout.base),
            window,
            cx,
        )
        .ok();

        let parent_mask = window.content_mask();
        let line_width = self.line.width();
        for range in self.ranges.iter().copied() {
            if range.end <= range.start {
                continue;
            }

            let mask = reveal_mask(parent_mask, bounds.origin.x, line_width, range);

            window.with_content_mask(Some(mask), |window| {
                LineLayout::paint(
                    &self.line,
                    bounds.origin,
                    layout.line_height,
                    TextAlign::Left,
                    None,
                    std::slice::from_ref(&layout.foreground),
                    window,
                    cx,
                )
                .ok();
            });
        }
    }
}

pub(in crate::chrome::aside) fn reveal_mask(
    parent: ContentMask<Pixels>,
    origin_x: Pixels,
    line_width: Pixels,
    range: RevealRange,
) -> ContentMask<Pixels> {
    let left = match range.start <= px(0.) {
        true => parent.bounds.left(),
        false => origin_x + range.start,
    };
    let right = match range.end >= line_width {
        true => parent.bounds.right(),
        false => origin_x + range.end,
    };
    ContentMask {
        bounds: Bounds::from_corners(
            point(left, parent.bounds.top()),
            point(right, parent.bounds.bottom()),
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::chrome::aside) struct RevealRange {
    pub(in crate::chrome::aside) start: Pixels,
    pub(in crate::chrome::aside) end: Pixels,
}

pub(in crate::chrome::aside) fn revealed(
    plan: &Wrapped,
    units: Range<usize>,
    windows: &[(Duration, Duration)],
    position: Duration,
) -> Vec<RevealRange> {
    let mut ranges: Vec<(Pixels, Pixels)> = Vec::new();
    let mut offset = px(0.);
    for index in units {
        let unit = &plan.units[index];
        for part in &unit.parts {
            let Some(&(start, end)) = windows.get(part.word) else {
                continue;
            };

            let share = progress_between(start, end, position);
            if share > 0. {
                let whole = plan
                    .word_widths
                    .get(part.word)
                    .copied()
                    .unwrap_or(part.width);
                let progress = match part.width > px(0.) {
                    true => ((whole * share - part.before) / part.width).clamp(0., 1.),
                    false => 0.,
                };
                let start = offset + part.offset;
                let end = start + part.width * progress;
                if end <= start {
                    continue;
                }
                match ranges.last_mut() {
                    Some((_, previous_end)) if start <= *previous_end => {
                        *previous_end = (*previous_end).max(end);
                    }
                    _ => ranges.push((start, end)),
                }
            }
        }
        offset += unit.width;
    }

    ranges
        .into_iter()
        .map(|(start, end)| RevealRange { start, end })
        .collect()
}

pub(in crate::chrome::aside) fn secondary_lyrics_lane(
    lane: &music::LyricsLane,
    line_active: bool,
    line_passed: bool,
    position: Duration,
    look: SecondaryLaneLook<'_>,
) -> gpui::AnyElement {
    let SecondaryLaneLook {
        dimming,
        voice,
        sung,
        plan,
    } = look;
    let theme = &sung.theme;
    let passed = line_passed || lane.sung_end().is_some_and(|end| position >= end);
    let shade = |singing: bool| {
        let active =
            singing && position >= lane.start && lane.sung_end().is_none_or(|end| position < end);
        let karaoke =
            secondary_karaoke_visible(lane, singing, position) && sung.karaoke && lane.worded();

        match (active, passed, karaoke) {
            (_, _, true) => theme.muted_foreground,
            (true, _, false) => theme.foreground,
            (false, true, false) => theme.muted_foreground.opacity(PAST),
            (false, false, false) => theme.muted_foreground.opacity(AHEAD),
        }
    };
    let tint = shade(line_active);
    let size = sung.lane;
    let karaoke_capable = sung.karaoke && lane.worded();
    let lyrics =
        div()
            .text_size(size)
            .map(|this| match (karaoke_capable, lane.words.as_ref(), plan) {
                (true, Some(words), Some(plan)) => {
                    this.child(karaoke_lane(plan, words, position, voice, sung))
                }
                _ => this.child(SharedString::from(lane.text.clone())),
            });
    let held = shade(true);
    let lyrics = match dimming {
        Some(departure) => lyrics
            .motion(("lane-dim", departure as usize), Motion::Quick, {
                move |this, t| this.text_color(mix(held, tint, t))
            })
            .into_any_element(),
        None => lyrics.text_color(tint).into_any_element(),
    };
    div()
        .flex()
        .flex_col()
        .when(!voice.lead(), |this| this.items_end().text_right())
        .child(lyrics)
        .when_some(
            selected_romanization(&lane.romanized, sung.scripts),
            |this, text| this.child(romanized_lyrics_lane(text, size, theme)),
        )
        .into_any_element()
}

pub(in crate::chrome::aside) fn secondary_lane_started(
    lane: &music::LyricsLane,
    position: Duration,
) -> bool {
    position >= lane.start
}

pub(in crate::chrome::aside) fn secondary_karaoke_visible(
    lane: &music::LyricsLane,
    line_active: bool,
    position: Duration,
) -> bool {
    line_active && secondary_lane_started(lane, position)
}

pub(in crate::chrome::aside) fn selected_romanization(
    romanized: &Option<music::RomanizedText>,
    scripts: Option<RomanizationScripts>,
) -> Option<String> {
    let romanized = romanized.as_ref()?;
    scripts?
        .contains(romanized.writing_system)
        .then(|| romanized.text.clone())
}

pub(in crate::chrome::aside) fn romanized_lyrics_lane(
    text: String,
    size: Pixels,
    theme: &ui::Theme,
) -> Div {
    div()
        .text_size(size)
        .text_color(theme.muted_foreground)
        .child(SharedString::from(text))
}

pub(in crate::chrome::aside) fn karaoke_window(
    words: &[music::LyricsWord],
    index: usize,
) -> (Duration, Duration) {
    let word = &words[index];
    let start = word.start;
    let end = match word.end > start {
        true => word.end,
        false => words
            .get(index + 1)
            .map(|next| next.start.max(start))
            .filter(|end| *end > start)
            .unwrap_or(start + SWEEP_LEAST),
    };
    (start, end)
}

/// Scales one line of text without touching the space between it and the next.
pub(in crate::chrome::aside) fn lifted(row: Div, sung: Sung) -> Div {
    match sung.lift == 1. {
        true => row,
        false => row.layer_scale(sung.lift).layer_scale_origin(sung.from),
    }
}

pub(in crate::chrome::aside) fn progress_between(
    start: Duration,
    end: Duration,
    position: Duration,
) -> f32 {
    if position <= start {
        return 0.;
    }
    if position >= end {
        return 1.;
    }
    let span = (end - start).as_secs_f32();
    ((position - start).as_secs_f32() / span).clamp(0., 1.)
}

pub(in crate::chrome::aside) fn line_has_passed(
    line: &music::LyricsLine,
    position: Duration,
) -> bool {
    line.sung_end().is_some_and(|end| position >= end)
}

pub(in crate::chrome::aside) fn primary_karaoke_visible(
    line: &music::LyricsLine,
    line_active: bool,
    position: Duration,
) -> bool {
    line_active
        || (position >= line.start
            && line
                .sung_end()
                .is_some_and(|end| position < end + Motion::Control.span()))
}

pub(in crate::chrome::aside) fn primary_karaoke_fade(
    line: &music::LyricsLine,
    line_active: bool,
    position: Duration,
) -> f32 {
    if line_active {
        return 0.;
    }
    line.sung_end().map_or(0., |end| {
        progress_between(end, end + Motion::Control.span(), position)
    })
}

pub(in crate::chrome::aside) fn background_line_singing(
    line: &music::LyricsLine,
    line_active: bool,
    position: Duration,
) -> bool {
    !line_active && position >= line.start && line.sung_end().is_some_and(|end| position < end)
}
