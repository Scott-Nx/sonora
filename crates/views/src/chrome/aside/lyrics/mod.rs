use std::collections::HashMap;
use std::time::Duration;

use gpui::{Pixels, Window};
use state::RomanizationScripts;

pub(super) mod karaoke;
pub(super) mod layout;
#[cfg(test)]
mod tests;

pub(super) const PAST: f32 = 0.4;
pub(super) const AHEAD: f32 = 0.6;
pub(super) const SWEEP_LEAST: Duration = Duration::from_millis(180);
pub(super) const KARAOKE_HZ: u32 = 45;
pub(super) const KARAOKE_FRAME: Duration = Duration::from_nanos(1_000_000_000 / KARAOKE_HZ as u64);
pub(super) const LANE_GAP_REM: f32 = 0.25;
pub(super) const LANE_SLACK: f32 = 0.25;

pub(super) use karaoke::{
    SecondaryLaneLook, Sung, background_line_singing, fixed_lyrics_lane, karaoke_lane,
    line_has_passed, primary_karaoke_fade, primary_karaoke_visible, progress_between,
    romanized_lyrics_lane, secondary_lane_started, secondary_lyrics_lane, selected_romanization,
};
pub(super) use layout::{Wrapped, lyrics_plan, lyrics_wrap_rows, plain_lyrics_rows, wrapped_rows};

pub(super) fn lanes_room(
    lanes: &[music::LyricsLane],
    scripts: Option<RomanizationScripts>,
    size: Pixels,
    leading: Pixels,
    width: Pixels,
    window: &mut Window,
    plans: Option<&HashMap<usize, Wrapped>>,
) -> Pixels {
    let rows = lanes
        .iter()
        .enumerate()
        .map(|(lane_index, lane)| {
            let spoken = plans.and_then(|plans| plans.get(&lane_index)).map_or_else(
                || wrapped_rows(&lane.text, size, width, window),
                |plan| plan.rows.len().max(1),
            );
            let romanized = karaoke::selected_romanization(&lane.romanized, scripts)
                .map_or(0, |text| wrapped_rows(&text, size, width, window));
            spoken + romanized
        })
        .sum::<usize>();

    // a lane inherits the line height of the verse, not its own text size
    let gaps = window.rem_size() * LANE_GAP_REM * lanes.len().saturating_sub(1) as f32;

    leading * (rows as f32 + LANE_SLACK) + gaps
}
