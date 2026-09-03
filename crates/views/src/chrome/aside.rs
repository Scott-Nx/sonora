use std::{collections::HashMap, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

use gpui::prelude::*;
use icu_segmenter::{LineSegmenter, options::LineBreakOptions};

use gpui::{
    Animation, AnimationExt as _, App, Bounds, ContentMask, Context, DecorationRun, Div,
    DragMoveEvent, Element, Entity, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    LayoutId, LineLayout, MouseDownEvent, Pixels, Point, Render, ScrollHandle, ScrollStrategy,
    ScrollWheelEvent, ShapedLine, SharedString, SpringConfig, SpringState, Style, Task, TextAlign,
    UniformListScrollHandle, Window, div, ease_in_out, point, px, size, svg, uniform_list,
};
use i18n::t;
use music::{Track, Voice};
use router::{Destination, LibraryTab, Link as _, LocalTab};
use state::{
    AppSettings, Lyrics, LyricsState, Playback, PlaybackState, Queue, RomanizationScripts, SideTab,
    Sonora, Whence,
};
use ui::{
    ActiveTheme as _, Button, Card, DraggedPin, Edge, Motion, Motioned as _, Pin, Pinnable as _,
    Popup, Scrollbar, Scroller, Spot, Text, Vacancy, drop_gap, drop_marker, ease_out_cubic,
    ease_out_expo, eyebrow, faint, mix, snapped, vacant,
};

use crate::chrome::{Chrome, section_label};
use crate::shared::effects;
use crate::shared::menus::ItemMenu;
use crate::shared::pins::Pinned as _;

const QUEUE: &str = "queue";
const BULLET: SharedString = SharedString::new_static("·");
const FADE: f32 = 96.;
const REST: f32 = FADE * 0.75;
const TAIL_ROWS: usize = 2;
const BLUR: f32 = 0.13;
const VEIL: f32 = 0.3;
const HAZE: f32 = 0.45;
const VERSE_FADE: f32 = 1.25;
const VERSE_SPRING: SpringConfig = SpringConfig::new(170., 23., 1.);
const PAST: f32 = 0.4;
const AHEAD: f32 = 0.6;
const ACTIVE_VERSE_GROWTH: Pixels = px(2.);
const FULLSCREEN_VERSE_GROWTH: Pixels = px(3.);
const LYRICS_HORIZONTAL_INSET_REM: f32 = 1.5;
const PINNED_SHARE: f32 = 0.25;
const PIN: f32 = 0.3;
// how far a row falls behind, in verse sizes
const LAG: f32 = 24.;
// never past this share
const LAG_SHARE: f32 = 0.28;
// movement the last row skips
const LAG_TRAIL: f32 = 0.9;
// The first row's physical spring. Rows farther along the viewport keep the same damping ratio but
// use a lower natural frequency, producing the cascading iMessage-like settle.
const LAG_SPRING: SpringConfig = SpringConfig::new(210., 22., 1.);
const LAG_STAGGER: f32 = 0.35;
const LAG_LEAST: Pixels = px(0.05);
const LAG_STALL: f32 = 0.064;
// Below this a blur is not worth a layer of its own.
const HAZE_LEAST: Pixels = px(0.05);
// How far a verse sinks while it is held.
const PRESSED: f32 = 0.955;
// The widest a line of lyrics is set, in multiples of its own size. Left to fill
// a fullscreen panel, a lead verse and a background one end up at opposite edges.
const REACH: f32 = 24.;
// What a sheet settling on the best answer comes in through: it blurs and fades
// on the way, once, on a curve that is the same going in as coming out.
const RESOLVE_BLUR: f32 = 0.2;
const RESOLVE_FADE: f32 = 0.5;
const SETTLE: std::time::Duration = std::time::Duration::from_secs(4);
const INSTRUMENTAL_BREAK: std::time::Duration = std::time::Duration::from_secs(5);
const SWEEP_LEAST: std::time::Duration = std::time::Duration::from_millis(180);
// karaoke sweep ceiling
const KARAOKE_HZ: u32 = 45;
const KARAOKE_FRAME: std::time::Duration =
    std::time::Duration::from_nanos(1_000_000_000 / KARAOKE_HZ as u64);
const SWEEP_STRETCH: f32 = 1.4;
// what a lane row actually takes, plus the gaps between lanes
const LANE_GAP_REM: f32 = 0.25;
const LANE_SLACK: f32 = 0.25;

fn track(queue: &Queue, position: QueuePosition) -> Option<Track> {
    match position {
        QueuePosition::Past(index) => queue.past().nth(index).cloned(),
        QueuePosition::Current => queue.current().cloned(),
        QueuePosition::Upcoming(index) => queue.upcoming().nth(index).cloned(),
        QueuePosition::Similar(index) => queue.similar().nth(index).cloned(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum QueuePosition {
    Past(usize),
    Current,
    Upcoming(usize),
    Similar(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Header(&'static str),
    Track(QueuePosition),
}

#[derive(Clone, Copy)]
struct Sections {
    past: usize,
    current: bool,
    upcoming: usize,
    similar: usize,
}

impl Sections {
    fn past_end(self) -> usize {
        match self.past {
            0 => 0,
            count => count + 1,
        }
    }

    fn current_end(self) -> usize {
        self.past_end() + 2 * usize::from(self.current)
    }

    fn upcoming_end(self) -> usize {
        self.current_end()
            + match self.upcoming {
                0 => 0,
                count => count + 1,
            }
    }

    fn len(self) -> usize {
        self.upcoming_end()
            + match self.similar {
                0 => 0,
                count => count + 1,
            }
    }

    fn current_index(self) -> Option<usize> {
        self.current.then(|| self.past_end() + 1)
    }

    fn slot(self, index: usize) -> Slot {
        if index < self.past_end() {
            return match index {
                0 => Slot::Header("queue-history"),
                _ => Slot::Track(QueuePosition::Past(index - 1)),
            };
        }
        if index < self.current_end() {
            return match index == self.past_end() {
                true => Slot::Header("queue-now-playing"),
                false => Slot::Track(QueuePosition::Current),
            };
        }
        if index < self.upcoming_end() {
            return match index == self.current_end() {
                true => Slot::Header("queue-up-next"),
                false => Slot::Track(QueuePosition::Upcoming(index - self.current_end() - 1)),
            };
        }
        match index == self.upcoming_end() {
            true => Slot::Header("queue-similar"),
            false => Slot::Track(QueuePosition::Similar(index - self.upcoming_end() - 1)),
        }
    }
}

/// A place in the sheet the pointer can be over: a verse, or the melody break
/// above it. They share a line index, so they need telling apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Warm {
    Verse(usize),
    Break(usize),
}

/// How near the pointer a spot is, and how far it has been pressed.
#[derive(Clone, Copy, Default)]
struct Touch {
    warmth: f32,
    depth: f32,
    waking: bool,
    settling: bool,
}

#[derive(Clone, Copy)]
struct Sung {
    karaoke: bool,
    scripts: Option<RomanizationScripts>,
    theme: ui::Theme,
    lift: f32,
    from: gpui::Point<f32>,
}

#[derive(Clone, Copy)]
struct RowLook {
    playing: bool,
    drop_line: Option<Edge>,
}

#[derive(Clone)]
struct ContextMenuState {
    track: Track,
    revision: u64,
    position: Point<Pixels>,
}

impl QueuePosition {
    fn past(self) -> Option<usize> {
        match self {
            Self::Past(index) => Some(index),
            _ => None,
        }
    }

    fn upcoming(self) -> Option<usize> {
        match self {
            Self::Upcoming(index) => Some(index),
            _ => None,
        }
    }

    fn similar(self) -> Option<usize> {
        match self {
            Self::Similar(index) => Some(index),
            _ => None,
        }
    }
}

pub(crate) struct Aside {
    queue: Entity<Queue>,
    playback: Entity<Playback>,
    lyrics: Entity<Lyrics>,
    settings: Entity<AppSettings>,
    tab: SideTab,
    verse_bar: Entity<Scrollbar>,
    followed: Option<usize>,
    nudges: u64,
    pinned: bool,
    nudged: Option<std::time::Instant>,
    verse_of: Option<String>,
    verse_take: u64,
    placing: bool,
    context_menu: Option<ContextMenuState>,
    track_menu: ItemMenu,
    drop_gap: Option<usize>,
    scroll: UniformListScrollHandle,
    scrollbar: Entity<Scrollbar>,
    past_len: usize,
    anchor: bool,
    titled: bool,
    aiming: bool,
    rested: Option<Pixels>,
    since: std::time::Instant,
    over: Option<Warm>,
    hovered: Option<Warm>,
    fading: Option<Warm>,
    linger: Option<Task<()>>,
    previous_active_line: Option<usize>,
    departing_line: Option<usize>,
    departed: std::time::Instant,
    arrived: std::time::Instant,
    arrival: u64,
    departure: u64,
    lyrics_wrap_width: Option<Pixels>,
    lyrics_wrap_size: Option<Pixels>,
    lyrics_wrap_font: Option<String>,
    lyrics_wraps: HashMap<usize, Wrapped>,
    lane_rooms: HashMap<usize, Pixels>,
    lane_plans: HashMap<usize, HashMap<usize, Wrapped>>,
    plain_rows: Option<Vec<SharedString>>,
    seen: Pixels,
    flying: bool,
    flew: bool,
    slid: std::time::Instant,
    drifts: HashMap<usize, SpringState>,
    pinning: Option<usize>,
    held: Option<Warm>,
    rising: Option<Warm>,
    sank: std::time::Instant,
    sinking: Option<Task<()>>,
    swept_frame: std::time::Instant,
    sweeping: Option<Task<()>>,
    showed: bool,
    resolving: bool,
}

impl Aside {
    pub(crate) fn new(
        queue: Entity<Queue>,
        playback: Entity<Playback>,
        tab: SideTab,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&queue, |this, queue, cx| {
            let revision = queue.read(cx).revision();
            if this
                .context_menu
                .as_ref()
                .is_some_and(|menu| menu.revision != revision)
            {
                this.track_menu.reset(cx);
                this.context_menu = None;
            }
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let me = cx.entity_id();
        let scroll = UniformListScrollHandle::new();
        let scrollbar =
            cx.new(|_| Scrollbar::new(scroll.0.borrow().base_handle.clone()).watching(me));
        let playlist_scrollbar = cx.new(|_| Scrollbar::inset().watching(me));
        let lyrics = Sonora::global(cx).lyrics.clone();
        cx.observe(&lyrics, |_, _, cx| cx.notify()).detach();
        let settings = Sonora::global(cx).settings.clone();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        let verse_bar = cx.new(|_| {
            Scrollbar::new(ScrollHandle::new())
                .spring(VERSE_SPRING)
                .watching(me)
        });

        Self {
            queue,
            playback,
            lyrics,
            settings,
            tab,
            verse_bar,
            followed: None,
            nudges: 0,
            pinned: true,
            nudged: None,
            verse_of: None,
            verse_take: 0,
            placing: false,
            context_menu: None,
            track_menu: ItemMenu::new(playlist_scrollbar),
            drop_gap: None,
            scroll,
            scrollbar,
            past_len: 0,
            anchor: true,
            titled: true,
            aiming: false,
            rested: None,
            since: std::time::Instant::now(),
            over: None,
            hovered: None,
            fading: None,
            linger: None,
            previous_active_line: None,
            departing_line: None,
            departed: std::time::Instant::now(),
            arrived: std::time::Instant::now(),
            arrival: 0,
            departure: 0,
            lyrics_wrap_width: None,
            lyrics_wrap_size: None,
            lyrics_wrap_font: None,
            lyrics_wraps: HashMap::new(),
            lane_rooms: HashMap::new(),
            lane_plans: HashMap::new(),
            plain_rows: None,
            seen: px(0.),
            flying: false,
            flew: true,
            slid: std::time::Instant::now(),
            drifts: HashMap::new(),
            pinning: None,
            held: None,
            rising: None,
            sank: std::time::Instant::now(),
            sinking: None,
            swept_frame: std::time::Instant::now(),
            sweeping: None,
            showed: false,
            resolving: false,
        }
    }

    pub(crate) fn strip(&mut self) {
        self.titled = false;
    }

    pub(crate) fn tab(&self) -> SideTab {
        self.tab
    }

    pub(crate) fn show(&mut self, tab: SideTab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            self.forget_verse();
            self.anchor_verse();
        }
        self.anchor = true;
        cx.notify();
    }

    pub(crate) fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.track_menu.reset(cx);
        self.context_menu = None;
        cx.notify();
    }

    /// How far a verse has sunk under the pointer, or risen back after being let
    /// go of.
    fn sink_progress(&self, window: &mut Window) -> f32 {
        let span = Motion::Quick.span().as_secs_f32().max(f32::EPSILON);
        let progress = (self.sank.elapsed().as_secs_f32() / span).clamp(0., 1.);
        if progress < 1. {
            window.request_animation_frame();
        }
        ease_in_out(progress)
    }

    fn touch(&self, spot: Warm, sharpen: f32, sink: f32) -> Touch {
        let waking = self.hovered == Some(spot);
        let settling = self.fading == Some(spot);
        Touch {
            warmth: match (waking, settling) {
                (true, _) => sharpen,
                (_, true) => 1. - sharpen,
                _ => 0.,
            },
            depth: match (self.held == Some(spot), self.rising == Some(spot)) {
                (true, _) => sink,
                (_, true) => 1. - sink,
                _ => 0.,
            },
            waking,
            settling,
        }
    }

    fn press_verse(&mut self, spot: Warm, down: bool, cx: &mut Context<Self>) {
        match down {
            true => {
                if self.held == Some(spot) {
                    return;
                }
                self.held = Some(spot);
                self.rising = None;
                self.sinking = None;
                self.sank = std::time::Instant::now();
            }
            false => {
                if self.held != Some(spot) {
                    return;
                }
                self.held = None;
                self.rising = Some(spot);
                self.sank = std::time::Instant::now();
                self.sinking = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Motion::Quick.span()).await;
                    this.update(cx, |this, cx| {
                        if this.rising != Some(spot) {
                            return;
                        }
                        this.rising = None;
                        cx.notify();
                    })
                    .ok();
                }));
            }
        }
        cx.notify();
    }

    fn sweep_karaoke(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.sweeping.is_some() {
            return;
        }
        let saved = match window.is_window_active() {
            true => None,
            false => self.settings.read(cx).saver().interval(),
        };
        let interval = KARAOKE_FRAME.max(saved.unwrap_or_default());
        let wait = interval.saturating_sub(self.swept_frame.elapsed());
        self.sweeping = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| {
                this.swept_frame = std::time::Instant::now();
                this.sweeping = None;
                cx.notify();
            })
            .ok();
        }));
    }

    fn sharpen_progress(&self, window: &mut Window) -> f32 {
        let span = Motion::Quick.span().as_secs_f32().max(f32::EPSILON);
        let progress = (self.since.elapsed().as_secs_f32() / span).clamp(0., 1.);
        if progress < 1. {
            window.request_animation_frame();
        }
        ease_in_out(progress)
    }

    fn forget_verse(&mut self) {
        self.flying = false;
        self.pinning = None;
        self.previous_active_line = None;
        self.departing_line = None;
        self.placing = true;
        self.forget_measurements();
    }

    fn forget_measurements(&mut self) {
        self.lyrics_wraps.clear();
        self.lane_rooms.clear();
        self.lane_plans.clear();
        self.plain_rows = None;
        self.drifts.clear();
    }

    // the panel took the wheel
    fn flown(&mut self, goal: Pixels, from: Pixels) {
        self.flying = true;
        self.flew = goal <= from;
    }

    // only automatic scrolls
    fn lagged(
        &mut self,
        scroll: &ScrollHandle,
        presentation: Pixels,
        verse: Pixels,
        nudges: u64,
    ) -> Drag {
        let now = std::time::Instant::now();
        let beat = now.duration_since(self.slid).as_secs_f32().min(LAG_STALL);
        self.slid = now;

        // follow the seen position
        let offset = scroll.offset().y + presentation;
        let step = offset - self.seen;
        self.seen = offset;
        if nudges != self.nudges {
            self.flying = false;
        }

        Drag {
            step: match self.flying {
                true => step,
                false => px(0.),
            },
            beat,
            downward: self.flew,
            most: (verse * LAG).min(scroll.bounds().size.height * LAG_SHARE),
        }
    }

    // A physical spring per row. Feeding the inverse scroll delta makes each row lag behind the
    // sheet; retaining velocity lets it settle naturally and survive a retarget without restarting.
    fn dragged(&mut self, row: usize, along: f32, drag: Drag, window: &mut Window) -> Pixels {
        let held = self.drifts.get(&row).copied();
        if held.is_none() && drag.step == px(0.) {
            return px(0.);
        }
        let mut state = held.unwrap_or_default();
        state.position = (px(state.position) - drag.step * (LAG_TRAIL * along))
            .clamp(-drag.most, drag.most)
            .as_f32();
        let spring = lag_spring(along);
        state = spring.step(state, 0., drag.beat);
        if spring.is_settled(state, 0., LAG_LEAST.as_f32()) {
            self.drifts.remove(&row);
            return px(0.);
        }
        self.drifts.insert(row, state);
        window.request_animation_frame();
        px(state.position)
    }

    fn set_hovered(&mut self, spot: Warm, over: bool, cx: &mut Context<Self>) {
        if !over {
            if self.over == Some(spot) {
                self.over = None;
            }
            if self.hovered == Some(spot) {
                self.hovered = None;
                self.fading = Some(spot);
                self.since = std::time::Instant::now();
                self.linger = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Motion::Quick.span()).await;
                    this.update(cx, |this, cx| {
                        if this.fading != Some(spot) {
                            return;
                        }
                        this.fading = None;
                        cx.notify();
                    })
                    .ok();
                }));
                cx.notify();
            }
            return;
        }

        self.over = Some(spot);
        if self.hovered == Some(spot) {
            return;
        }
        self.fading = None;
        self.linger = Some(cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                if this.over != Some(spot) {
                    return;
                }
                this.hovered = Some(spot);
                cx.notify();
            })
            .ok();
        }));
    }

    fn enqueue(&mut self, pin: &Pin, gap: Option<usize>, cx: &mut Context<Self>) {
        self.playback
            .update(cx, |playback, cx| playback.enqueue_pin(pin, gap, cx));
    }

    fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        self.track_menu.reset(cx);
        self.context_menu = None;
        cx.notify();
    }

    fn row(
        track: Track,
        index: usize,
        position: QueuePosition,
        queue_revision: u64,
        look: RowLook,
        cx: &Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let RowLook { playing, drop_line } = look;
        let theme = *cx.theme();
        let past_index = position.past();
        let queue_index = position.upcoming();
        let similar_index = position.similar();
        let title = match position {
            QueuePosition::Past(_) => theme.muted_foreground,
            QueuePosition::Current => theme.primary,
            QueuePosition::Upcoming(_) | QueuePosition::Similar(_) => theme.foreground,
        };
        let pin = track.pin();
        let menu_track = track.clone();

        let card = Card::new(
            ("queue-track", index),
            SharedString::from(track.name.clone()),
        )
        .cover(track.cover.clone())
        .bare_meta(
            crate::shared::cells::artist_links(
                SharedString::from(format!("queue-track-artist-{index}")),
                track.artist_refs.clone(),
                track.artists.clone(),
                theme.muted_foreground,
            )
            .text_size(theme.text(Text::Small))
            .truncate(),
        )
        .tint(title)
        .when(track.explicit, Card::explicit)
        .play(
            playing,
            cx.listener(move |this, _, _, cx| {
                let stale = this.queue.read(cx).revision() != queue_revision;
                this.playback.update(cx, |playback, cx| match position {
                    QueuePosition::Current => playback.toggle_play(cx),
                    QueuePosition::Past(index) if !stale => playback.play_past(index, cx),
                    QueuePosition::Upcoming(index) if !stale => playback.play_upcoming(index, cx),
                    QueuePosition::Similar(index) if !stale => playback.play_similar(index, cx),
                    _ => {}
                });
            }),
        )
        .menu(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            this.track_menu.reset(cx);
            this.context_menu = Some(ContextMenuState {
                track: menu_track.clone(),
                revision: queue_revision,
                position: event.position,
            });
            cx.notify();
        }))
        .when_some(past_index, |this, index| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_past(index, cx));
                }
            }))
        })
        .when_some(queue_index, |this, target| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_upcoming(target, cx));
                }
            }))
            .action(
                Button::new(("remove-queued-track", index))
                    .ghost()
                    .small()
                    .mr_1()
                    .icon("icons/x.svg")
                    .tooltip("menu-remove-from-queue")
                    .tint(theme.muted_foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.queue.update(cx, |queue, cx| {
                            if queue.revision() == queue_revision {
                                queue.remove_upcoming(target, cx);
                            }
                        });
                    })),
            )
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DraggedPin>, _, cx| {
                    let Some(gap) = drop_gap(event.bounds, event.event.position, target) else {
                        return;
                    };
                    let gap = match event.drag(cx).spot(QUEUE) {
                        Some(held) => (gap != held.index && gap != held.index + 1).then_some(gap),
                        None => Some(gap),
                    };
                    if this.drop_gap != gap {
                        this.drop_gap = gap;
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(move |this, dragged: &DraggedPin, _, cx| {
                let gap = this.drop_gap.take();
                match dragged.spot(QUEUE) {
                    Some(held) => {
                        if let Some(gap) = gap {
                            this.queue.update(cx, |queue, cx| {
                                if queue.revision() == held.revision {
                                    queue.move_upcoming_to_gap(held.index, gap, cx);
                                }
                            });
                        }
                    }
                    None => this.enqueue(&dragged.pin, gap, cx),
                }
                cx.notify();
            }))
        })
        .when_some(similar_index, |this, target| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_similar(target, cx));
                }
            }))
            .action(
                Button::new(("remove-similar-track", index))
                    .ghost()
                    .small()
                    .mr_1()
                    .icon("icons/x.svg")
                    .tooltip("menu-remove-from-queue")
                    .tint(theme.muted_foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.queue.update(cx, |queue, cx| {
                            if queue.revision() == queue_revision {
                                queue.remove_similar(target, cx);
                            }
                        });
                    })),
            )
        })
        .when_some(pin, |this, pin| match queue_index {
            Some(index) => this.pin_from(pin, Spot::new(QUEUE, index).revision(queue_revision)),
            None => this.pin(pin),
        });

        div()
            .id(("queue-track-container", index))
            .relative()
            .min_w_0()
            .child(card)
            .when_some(drop_line, |this, edge| this.child(drop_marker(edge, cx)))
    }

    fn menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let ContextMenuState {
            track, position, ..
        } = self.context_menu.clone()?;

        Some(
            Popup::new(position, self.track_menu.for_track(&track, cx))
                .on_close(cx.listener(|this, _, _, cx| this.dismiss_menu(cx))),
        )
    }

    fn header(
        &self,
        sections: Sections,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = *cx.theme();

        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap_2()
            .h(snapped(theme.metrics.header, window))
            .px_2()
            .when(self.titled, |this| {
                this.border_b_1().border_color(theme.border).child(eyebrow(
                    match self.tab {
                        SideTab::Queue => t!("queue-title"),
                        SideTab::Lyrics => t!("lyrics-title"),
                    },
                    cx,
                ))
            })
            .when(!self.titled, |this| {
                this.justify_end().pr(theme.metrics.control + px(8.))
            })
            .when(self.tab == SideTab::Queue, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Button::new("toggle-radio")
                                .ghost()
                                .small()
                                .icon("icons/radio.svg")
                                .tooltip("queue-radio")
                                .tint(match self.playback.read(cx).radio() {
                                    true => theme.primary,
                                    false => theme.muted_foreground,
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.playback
                                        .update(cx, |playback, cx| playback.toggle_radio(cx));
                                })),
                        )
                        .child(
                            Button::new("reset-queue")
                                .ghost()
                                .small()
                                .label(t!("queue-reset"))
                                .tint(theme.muted_foreground)
                                .disabled(!self.queue.read(cx).reordered())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.queue.update(cx, |queue, cx| queue.reset(cx));
                                })),
                        )
                        .child(
                            Button::new("clear-queue")
                                .ghost()
                                .small()
                                .label(t!("queue-clear"))
                                .tint(theme.muted_foreground)
                                .disabled(sections.upcoming == 0)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.queue.update(cx, |queue, cx| queue.clear_upcoming(cx));
                                })),
                        ),
                )
            })
    }

    fn follow(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let theme = *cx.theme();
        if self.tab != SideTab::Lyrics || self.pinned {
            return None;
        }

        Some(
            div()
                .absolute()
                .when_else(self.titled, |this| this.bottom_3(), |this| this.bottom_16())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div().flex().flex_none().block_mouse_except_scroll().child(
                        Button::new("resume-pin")
                            .ghost()
                            .small()
                            .icon("icons/undo-2.svg")
                            .tooltip("lyrics-follow")
                            .rounded_full()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.popover)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.anchor_verse();
                                cx.notify();
                            })),
                    ),
                ),
        )
    }

    fn verses(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let position = self.playback.read(cx).live_position();
        let singing = matches!(self.playback.read(cx).state(), PlaybackState::Playing);
        let lyrics = self.lyrics.read(cx);
        let state = lyrics.state().clone();
        let shown = lyrics.current().map(|hit| hit.lyrics.clone());
        let credit = lyrics
            .current()
            .map(|hit| (hit.source, hit.writers.clone()));
        let following = lyrics.following().map(str::to_owned);
        let take = lyrics.revision();
        let (karaoke_lyrics, romanization_scripts, lyrics_wrap_font) = {
            let settings = self.settings.read(cx);
            (
                settings.karaoke_lyrics(),
                settings
                    .romanized_lyrics()
                    .then(|| settings.romanization_scripts()),
                settings.font().to_owned(),
            )
        };
        let karaoke_effects = karaoke_lyrics && effects();
        let sung = Sung {
            karaoke: karaoke_effects,
            scripts: romanization_scripts,
            theme,
            lift: 1.,
            from: gpui::point(0., 0.5),
        };

        if self.verse_of != following {
            self.verse_of = following;
            self.verse_take = take;
            self.forget_verse();
            self.anchor_verse();
            let scroll = self.verse_bar.read(cx).scroll().clone();
            scroll.set_offset(gpui::point(scroll.offset().x, px(0.)));
            self.verse_bar
                .update(cx, |bar, _| bar.remember_offset(scroll.offset().y));
        } else if self.verse_take != take {
            self.verse_take = take;
            // Nothing was on screen before, so there is no change to play: the
            // sheet is simply put up.
            self.resolving = self.showed;
            self.forget_verse();
            self.anchor_verse();
        }
        self.showed = shown.is_some();

        let empty = |key: &'static str, cx: &mut Context<Self>| {
            vacant(i18n::lookup(key, None), cx)
                .flex_1()
                .into_any_element()
        };
        let lines = match (&state, &shown) {
            (LyricsState::Ready, Some(music::Lyrics::Synced { lines })) => Some(lines.clone()),
            _ => None,
        };

        // aim before reading
        if let Some(lines) = &lines {
            let live = active_lyrics_row(lines, position);
            let focus = match self.pinning {
                Some(row) if Some(row) != live => Some(row),
                _ => {
                    self.pinning = None;
                    live
                }
            };
            self.pin_verse(focus, window, cx);
        }

        let verse = match self.titled {
            true => theme.text(Text::Large),
            false => theme.text(Text::Title) + FULLSCREEN_VERSE_GROWTH,
        };
        let reach = verse * REACH;
        let wrap_size = active_verse_size(verse);
        let scroll = self.verse_bar.read(cx).scroll().clone();
        let (nudges, presentation) = {
            let bar = self.verse_bar.read(cx);
            (bar.nudges(), bar.presentation().y)
        };
        let animations = ui::motion::animates(cx);
        if !animations {
            self.drifts.clear();
        }
        let drag = match (lines.is_some(), animations) {
            (true, true) => self.lagged(&scroll, presentation, verse, nudges),
            _ => Drag::default(),
        };
        let inset = window.rem_size() * LYRICS_HORIZONTAL_INSET_REM;
        let wrap_width = (scroll.bounds().size.width - inset)
            .min(reach - inset)
            .max(px(0.));
        if self.lyrics_wrap_width != Some(wrap_width)
            || self.lyrics_wrap_size != Some(wrap_size)
            || self.lyrics_wrap_font.as_deref() != Some(lyrics_wrap_font.as_str())
        {
            self.lyrics_wrap_width = Some(wrap_width);
            self.lyrics_wrap_size = Some(wrap_size);
            self.lyrics_wrap_font = Some(lyrics_wrap_font);
            self.forget_measurements();
            window.request_animation_frame();
        }

        let mut body: Vec<gpui::AnyElement> = match (&lines, &state) {
            (Some(lines), _) => {
                let active_line = sung_line(lines, position);
                if singing
                    && karaoke_effects
                    && active_line.is_some_and(|index| lines[index].worded())
                {
                    self.sweep_karaoke(window, cx);
                }
                if self.previous_active_line != active_line {
                    if self.previous_active_line.is_some() {
                        self.departing_line = self.previous_active_line;
                        self.departure = self.departure.wrapping_add(1);
                        self.departed = std::time::Instant::now();
                    }
                    if active_line.is_some() {
                        self.arrival = self.arrival.wrapping_add(1);
                        self.arrived = std::time::Instant::now();
                    }
                    self.previous_active_line = active_line;
                }
                if self.departing_line.is_some() && self.departed.elapsed() >= Motion::Base.span() {
                    self.departing_line = None;
                }
                let instrumental_line = active_instrumental(lines, position);
                let hazing = effects() && self.pinned;
                let blur = verse * BLUR;
                let sharpen = self.sharpen_progress(window);
                // with motion turned down a press is simply on or off
                let sink = match animations {
                    true => self.sink_progress(window),
                    false => 1.,
                };
                let view = scroll.bounds();
                if hazing && scroll.bounds_for_item(0).is_none() {
                    window.request_animation_frame();
                }
                let mut rendered = Vec::with_capacity(lyric_row_count(lines));

                for (index, line) in lines.iter().enumerate() {
                    let seek = line.start;
                    let gap = instrumental_gap_before(lines, index);
                    let instrumental_start = line.start.saturating_sub(gap);
                    let has_instrumental = gap >= INSTRUMENTAL_BREAK;
                    let instrumental_progress = if has_instrumental {
                        progress_between(instrumental_start, line.start, position)
                    } else {
                        0.
                    };
                    let instrumental_has_passed = position >= line.start;

                    let verse_touch = self.touch(Warm::Verse(index), sharpen, sink);
                    let notes_touch = self.touch(Warm::Break(index), sharpen, sink);
                    let warmth = verse_touch.warmth;
                    let depth = verse_touch.depth;
                    // whatever the pointer rests on comes back into focus
                    let clearing = |touch: Touch, depth: f32| match (touch.waking, touch.settling) {
                        (true, _) => depth * (1. - sharpen),
                        (false, true) => depth * sharpen,
                        (false, false) => depth,
                    };
                    let haze = |depth: f32| clearing(verse_touch, depth);
                    if has_instrumental {
                        let notes_row = rendered.len();
                        if singing && instrumental_line == Some(index) {
                            window.request_animation_frame();
                        }
                        let notes_along = viewport_along(&scroll, notes_row, view, drag.downward);
                        let notes_drift = self.dragged(notes_row, notes_along, drag, window);
                        let notes_translation = presentation + notes_drift;
                        let softness = match hazing && instrumental_line != Some(index) {
                            true => clearing(
                                notes_touch,
                                viewport_haze(&scroll, notes_row, view, blur, notes_translation),
                            ),
                            false => 0.,
                        };
                        let notes = instrumental_row(
                            instrumental_progress,
                            instrumental_has_passed,
                            verse,
                            &theme,
                        )
                        .id(("instrumental", index))
                        .w_full()
                        .max_w(reach)
                        .px_2()
                        .rounded(theme.radius)
                        .cursor_pointer()
                        .when(notes_touch.warmth > 0., |this| {
                            this.bg(theme.table_hover.opacity(notes_touch.warmth))
                        })
                        .when(notes_touch.depth > 0., |this| {
                            this.layer_scale(1. - (1. - PRESSED) * notes_touch.depth)
                        })
                        .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                            this.set_hovered(Warm::Break(index), *over, cx)
                        }))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Break(index), true, cx)
                            }),
                        )
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Break(index), false, cx)
                            }),
                        )
                        .on_mouse_up_out(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Break(index), false, cx)
                            }),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.seek_verse(notes_row, instrumental_start, cx);
                        }))
                        .when(softness > 0., |this| this.opacity(1. - VEIL * softness))
                        .map(|this| match blur * softness {
                            soft if soft > HAZE_LEAST => this.blur(soft),
                            _ => this,
                        });
                        rendered.push(adrift(notes, notes_translation, window).into_any_element());
                    }

                    let row = rendered.len();
                    let along = viewport_along(&scroll, row, view, drag.downward);
                    let drift = self.dragged(row, along, drag, window);
                    let translation = presentation + drift;
                    let active = Some(index) == active_line;
                    let departing = Some(index) == self.departing_line;
                    let karaoke = Some(index) == active_line && line.worded() && karaoke_effects;
                    let primary_karaoke = karaoke && line.words.is_some();
                    if let std::collections::hash_map::Entry::Vacant(slot) =
                        self.lyrics_wraps.entry(index)
                        && let Some(wrapped) = lyrics_wrap_rows(
                            &line.text,
                            line.words.as_deref(),
                            wrap_size,
                            wrap_width,
                            window,
                        )
                    {
                        slot.insert(wrapped);
                    }
                    let wrapped = self.lyrics_wraps.get(&index);
                    let line_has_ended = active_line.is_some_and(|active| index < active)
                        || line_has_passed(line, position);
                    let worded = karaoke_effects && line.worded() && line.words.is_some();
                    let shade = |singing: bool| match (singing, line_has_ended) {
                        (true, _) if worded => theme.muted_foreground,
                        (true, _) => theme.foreground,
                        (false, true) => theme.muted_foreground.opacity(PAST),
                        (false, false) => theme.muted_foreground.opacity(AHEAD),
                    };
                    let tint = shade(Some(index) == active_line);

                    let dimming = (animations && departing).then_some(self.departure);
                    let growing =
                        animations && active && self.arrived.elapsed() < Motion::Base.span();
                    let shrinking = dimming.is_some();
                    let active_size = active_verse_size(verse);
                    let small = verse / active_size;
                    let big = active_size / verse;
                    // both ways land on 1
                    let lift = match (growing, shrinking) {
                        (true, _) => small + (1. - small) * ramp(self.arrived, window),
                        (_, true) => big - (big - 1.) * ramp(self.departed, window),
                        _ => 1.,
                    };
                    let paint = match (growing, shrinking) {
                        (true, _) => mix(shade(false), tint, ramp(self.arrived, window)),
                        (_, true) => mix(shade(true), tint, ramp(self.departed, window)),
                        _ => tint,
                    };
                    let sung = Sung {
                        lift,
                        from: match line.voice.lead() {
                            true => gpui::point(0., 0.5),
                            false => gpui::point(1., 0.5),
                        },
                        ..sung
                    };

                    let primary = match (primary_karaoke, line.words.as_ref(), wrapped) {
                        (true, Some(words), Some(plan)) => {
                            karaoke_lane(plan, words, position, line.voice, sung).into_any_element()
                        }
                        (_, _, Some(plan)) => {
                            fixed_lyrics_lane(&plan.text, line.voice, sung).into_any_element()
                        }
                        _ => div()
                            .child(SharedString::from(line.text.clone()))
                            .into_any_element(),
                    };
                    let fade = match (line.secondary.is_empty(), active, departing) {
                        (true, _, _) => None,
                        (_, true, _) => Some(("lane-in", self.arrival, growing)),
                        (_, _, true) => Some(("lane-out", self.departure, animations)),
                        _ => None,
                    };
                    if fade.is_some() && sung.karaoke {
                        let plans = self.lane_plans.entry(index).or_default();
                        for (lane_index, lane) in line.secondary.iter().enumerate() {
                            let Some(words) =
                                lane.words.as_deref().filter(|words| !words.is_empty())
                            else {
                                continue;
                            };
                            plans.entry(lane_index).or_insert_with(|| {
                                lyrics_wrap_rows(
                                    &lane.text,
                                    Some(words),
                                    theme.text(Text::Body),
                                    wrap_width,
                                    window,
                                )
                                .unwrap_or_else(|| {
                                    lyrics_plan(
                                        &lane.text,
                                        Some(words),
                                        theme.text(Text::Body),
                                        None,
                                        window,
                                    )
                                })
                            });
                        }
                    }
                    let room = fade.map(|_| match self.lane_rooms.get(&index) {
                        Some(room) => *room,
                        None => {
                            let room = lanes_room(
                                &line.secondary,
                                romanization_scripts,
                                theme.text(Text::Body),
                                active_verse_size(verse) * ui::LEADING,
                                wrap_width,
                                window,
                                self.lane_plans.get(&index),
                            );
                            self.lane_rooms.insert(index, room);
                            room
                        }
                    });
                    let lanes = fade.zip(room).map(|((tag, take, animated), room)| {
                        let arriving = tag == "lane-in";
                        let group = div().flex().flex_col().gap_1().children(
                            line.secondary.iter().enumerate().map(|(lane_index, lane)| {
                                let plan = if sung.karaoke {
                                    self.lane_plans
                                        .get(&index)
                                        .and_then(|plans| plans.get(&lane_index))
                                } else {
                                    None
                                };
                                let sung_by_end = line
                                    .sung_end()
                                    .is_some_and(|end| secondary_lane_started(lane, end));
                                secondary_lyrics_lane(
                                    lane,
                                    true,
                                    line_has_ended,
                                    position,
                                    SecondaryLaneLook {
                                        dimming: dimming.filter(|_| sung_by_end),
                                        voice: line.voice,
                                        sung,
                                        plan,
                                    },
                                )
                            }),
                        );
                        match animated {
                            true => group
                                .overflow_hidden()
                                .with_animation(
                                    (tag, take as usize),
                                    Animation::new(Motion::Base.span()).with_easing(ease_in_out),
                                    move |this, t| {
                                        let shown = match arriving {
                                            true => t,
                                            false => 1. - t,
                                        };
                                        this.opacity(shown).max_h(room * shown)
                                    },
                                )
                                .into_any_element(),
                            false => group.into_any_element(),
                        }
                    });
                    let content = div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .when(!line.voice.lead(), |this| this.items_end().text_right())
                        .child(primary)
                        .when_some(
                            selected_romanization(&line.romanized, romanization_scripts),
                            |this, text| this.child(romanized_lyrics_lane(text, &theme)),
                        )
                        .children(lanes)
                        .when(depth > 0., |this| {
                            this.layer_scale(1. - (1. - PRESSED) * depth)
                        });

                    let softness = match hazing && Some(index) != active_line {
                        true => haze(viewport_haze(&scroll, row, view, blur, translation)),
                        false => 0.,
                    };
                    let traded = index
                        .checked_sub(1)
                        .is_some_and(|previous| lines[previous].voice != line.voice);
                    let verse_line = div()
                        .id(("verse", index))
                        .w_full()
                        .max_w(reach)
                        .px_2()
                        .py_1()
                        .when(traded, |this| this.mt_2())
                        .rounded(theme.radius)
                        .cursor_pointer()
                        .when(warmth > 0., |this| {
                            this.bg(theme.table_hover.opacity(warmth))
                        })
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Verse(index), true, cx)
                            }),
                        )
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Verse(index), false, cx)
                            }),
                        )
                        .on_mouse_up_out(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.press_verse(Warm::Verse(index), false, cx)
                            }),
                        )
                        .text_size(verse)
                        .line_height(active_verse_size(verse) * ui::LEADING)
                        .text_color(tint)
                        .font_weight(FontWeight::SEMIBOLD)
                        .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                            this.set_hovered(Warm::Verse(index), *over, cx)
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.seek_verse(row, seek, cx);
                        }))
                        .child(content);

                    let verse_line = verse_line
                        .when(softness > 0., |this| this.opacity(1. - VEIL * softness))
                        .map(|this| match blur * softness {
                            soft if soft > HAZE_LEAST => this.blur(soft),
                            _ => this,
                        });
                    let verse_line = match (growing, shrinking, active) {
                        (_, true, false) => verse_line.text_color(paint),
                        (true, _, _) | (_, _, true) => {
                            verse_line.text_size(active_size).text_color(paint)
                        }
                        _ => verse_line,
                    };
                    rendered.push(adrift(verse_line, translation, window).into_any_element());
                }

                rendered
            }
            (None, LyricsState::Ready) => match &shown {
                Some(music::Lyrics::Plain { text, romanized }) => {
                    let rows = match &self.plain_rows {
                        Some(rows) => rows.clone(),
                        None => {
                            let rows =
                                plain_lyrics_rows(text, theme.text(Text::Body), wrap_width, window);
                            self.plain_rows = Some(rows.clone());
                            rows
                        }
                    };
                    vec![
                        div()
                            .w_full()
                            .max_w(reach)
                            .px_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_size(theme.text(Text::Body))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .children(rows)
                            .when_some(
                                selected_romanization(romanized, romanization_scripts),
                                |this, text| this.child(romanized_lyrics_lane(text, &theme)),
                            )
                            .into_any_element(),
                    ]
                }
                _ => vec![wordless("lyrics-missing", "icons/mic-off.svg")],
            },
            (None, LyricsState::Idle) => vec![empty("lyrics-idle", cx)],
            (None, LyricsState::Loading) => vec![empty("lyrics-loading", cx)],
            (None, LyricsState::Instrumental) => {
                vec![wordless("lyrics-instrumental", "icons/guitar.svg")]
            }
            (None, LyricsState::Missing) => {
                vec![wordless("lyrics-missing", "icons/mic-off.svg")]
            }
            (None, LyricsState::Failed(_)) => vec![empty("lyrics-failed", cx)],
        };

        if state == LyricsState::Ready
            && let Some((source, writers)) = &credit
        {
            let credit = body.len();
            let along = viewport_along(&scroll, credit, scroll.bounds(), drag.downward);
            let drift = self.dragged(credit, along, drag, window);
            let translation = match lines.is_some() {
                true => presentation + drift,
                false => px(0.),
            };
            let note = div()
                .w_full()
                .max_w(reach)
                .px_2()
                .pt_2()
                .flex()
                .flex_col()
                .text_size(theme.text(Text::Small))
                .text_color(theme.muted_foreground)
                .child(t!("lyrics-source", source = *source))
                .when(!writers.is_empty(), |this| {
                    let writers = writers.join(", ");
                    this.child(t!("lyrics-writers", writers = writers.as_str()))
                });
            body.push(adrift(note, translation, window).into_any_element());
        }

        let (over, under) = match &lines {
            Some(lines) => self.verse_slack(lyric_row_count(lines), window, cx),
            None => (px(REST), px(REST)),
        };

        let sheet = Scroller::new("lyrics", &self.verse_bar)
            .when(lines.is_some(), Scroller::manual_presentation)
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .flex_1()
            .min_h_0()
            .px_1()
            .pt(over)
            .pb(under)
            .when(effects(), |this| {
                let fade = verse * VERSE_FADE;
                this.fade_edges(fade, fade)
            })
            .children(body);

        // A sheet only ever replaces another once, when every source has
        // answered, and that is the one change worth showing.
        match self.resolving && ui::motion::animates(cx) {
            true => sheet
                .with_animation(
                    ("verse-sheet", self.verse_take as usize),
                    Animation::new(Motion::Base.span()).with_easing(ui::ease_in_out_cubic),
                    move |this, t| {
                        this.blur(verse * RESOLVE_BLUR * (1. - t))
                            .opacity(1. - RESOLVE_FADE * (1. - t))
                    },
                )
                .into_any_element(),
            false => sheet.into_any_element(),
        }
    }

    fn verse_slack(&self, count: usize, window: &Window, cx: &App) -> (Pixels, Pixels) {
        let scroll = self.verse_bar.read(cx).scroll().clone();
        let view = scroll.bounds().size.height;
        if view <= px(0.) {
            window.request_animation_frame();
            return (px(REST), px(REST));
        }
        let tail = count
            .checked_sub(1)
            .and_then(|last| scroll.bounds_for_item(last))
            .map_or(px(0.), |item| item.size.height);

        (
            snapped((view * PIN).max(px(REST)), window),
            snapped((view * (1. - PIN) - tail).max(px(REST)), window),
        )
    }

    fn anchor_verse(&mut self) {
        self.pinned = true;
        self.aiming = false;
        self.rested = None;
        self.followed = None;
        self.nudged = None;
    }

    /// Seeks to a verse and holds the panel on the row it was asked for. The
    /// clock takes a moment to report the new position, and until it does the
    /// verse being sung is still the old one, which is where the panel would
    /// otherwise fly off to.
    fn seek_verse(&mut self, row: usize, position: std::time::Duration, cx: &mut Context<Self>) {
        self.pinning = Some(row);
        self.seek_lyrics(position, cx);
    }

    fn seek_lyrics(&mut self, position: std::time::Duration, cx: &mut Context<Self>) {
        self.anchor_verse();
        self.playback
            .update(cx, |playback, cx| playback.seek(position, cx));
        cx.notify();
    }

    fn pin_verse(&mut self, sung: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let scroll = self.verse_bar.read(cx).scroll().clone();
        let resting = scroll.offset().y;
        let nudges = self.verse_bar.read(cx).nudges();
        if self.nudges != nudges {
            self.nudges = nudges;
            self.pinned = false;
            self.flying = false;
            self.drifts.clear();
            self.nudged = Some(std::time::Instant::now());
        }
        if !self.pinned {
            self.followed = sung;
            // Keep the reader in charge for as long as they keep moving: the timer counts from the
            // last scroll, not from the first one.
            if self.rested != Some(resting) {
                self.rested = Some(resting);
                self.nudged = Some(std::time::Instant::now());
            }
            if self.nudged.is_some_and(|at| at.elapsed() >= SETTLE) {
                self.anchor_verse();
            } else {
                return;
            }
        }
        if sung.is_none() {
            return;
        }
        // The rows a verse sits among change on the very frame it starts being sung, and their
        // bounds only settle once that frame has been laid out. Aim on the next one.
        if self.followed != sung {
            self.followed = sung;
            self.aiming = true;
            window.request_animation_frame();
            return;
        }
        if !self.aiming {
            return;
        }
        let Some(item) = sung.and_then(|index| scroll.bounds_for_item(index)) else {
            return;
        };
        self.aiming = false;
        let view = scroll.bounds();
        // Preserve the fractional target. Spring scrolls are presented by the compositor, so the
        // text never has to walk the raster grid while the layer is settling.
        let goal = anchored_lyrics_offset(
            view.origin.y,
            item.origin.y,
            view.size.height,
            scroll.max_offset().y,
        );
        self.flown(goal, scroll.offset().y);
        match std::mem::take(&mut self.placing) || cx.reduce_motion() {
            true => self.verse_bar.update(cx, |bar, _| bar.place(goal)),
            false => self.verse_bar.update(cx, |bar, _| bar.aim(goal, window)),
        }
    }

    fn pin(&mut self, sections: Sections, window: &Window, cx: &Context<Self>) {
        let Some(index) = sections.current_index() else {
            self.anchor = false;
            return;
        };

        let viewport = self.scroll.0.borrow().base_handle.bounds().size.height;
        if viewport <= px(0.) {
            window.request_animation_frame();
            return;
        }

        let row = snapped(cx.theme().metrics.list_row, window);
        let above = (viewport * PINNED_SHARE / row).round() as usize;
        self.scroll
            .scroll_to_item_strict_with_offset(index, ScrollStrategy::Top, above);
        self.anchor = false;
    }

    // unnamed origins stay unlabelled
    fn playing_from(&self, cx: &App) -> Option<(SharedString, Destination)> {
        let origin = self.playback.read(cx).origin()?;
        let id = SharedString::from(origin.id.clone());
        let place = match origin.whence {
            Whence::Album => Destination::Album(id),
            Whence::Playlist => Destination::Playlist(id),
            Whence::Artist => Destination::Artist(id),
            Whence::Radio => Destination::Song(id),
            Whence::Saved => Destination::Library(LibraryTab::Songs),
            Whence::Local => match origin.id.is_empty() {
                true => Destination::Local(LocalTab::Songs),
                false => Destination::Local(LocalTab::Favorites),
            },
        };
        let name = match origin.whence {
            Whence::Saved => t!("library-liked-songs"),
            Whence::Local => match origin.id.is_empty() {
                true => t!("nav-local"),
                false => t!("library-liked-songs"),
            },
            _ => origin.name.clone()?,
        };

        Some((name, place))
    }

    fn rows(&self, sections: Sections, cx: &mut Context<Self>) -> gpui::UniformList {
        let queue = self.queue.clone();
        let from = self.playing_from(cx);
        let drop_gap = self.drop_gap;
        let upcoming = sections.upcoming;
        let audible = matches!(self.playback.read(cx).state(), PlaybackState::Playing);

        uniform_list(
            "queue-rows",
            sections.len() + TAIL_ROWS,
            cx.processor(move |_, range: Range<usize>, window, cx| {
                let (revision, slots) = {
                    let queue = queue.read(cx);
                    let slots = range
                        .clone()
                        .map(|index| {
                            let slot = (index < sections.len()).then(|| sections.slot(index));
                            let found = match slot {
                                Some(Slot::Track(position)) => track(queue, position),
                                Some(Slot::Header(_)) | None => None,
                            };
                            (index, slot, found)
                        })
                        .collect::<Vec<_>>();
                    (queue.revision(), slots)
                };

                slots
                    .into_iter()
                    .map(|(index, slot, found)| match (slot, found) {
                        (None, _) => div().into_any_element(),
                        (Some(Slot::Header(key)), _) => {
                            let label = section_label(key, window, cx);
                            match (key, from.clone()) {
                                ("queue-now-playing", Some((name, place))) => label
                                    .w_full()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(cx.theme().text(Text::Small))
                                            .text_color(cx.theme().muted_foreground)
                                            .child(BULLET),
                                    )
                                    .child(faint(cx).child(t!("queue-from")))
                                    .child(source_link(name, place, cx))
                                    .into_any_element(),
                                _ => label.into_any_element(),
                            }
                        }
                        (Some(Slot::Track(position)), Some(found)) => {
                            let drop_line = match (position.upcoming(), drop_gap) {
                                (Some(queued), Some(gap)) if gap == queued => Some(Edge::Above),
                                (Some(queued), Some(gap))
                                    if gap == upcoming && queued + 1 == upcoming =>
                                {
                                    Some(Edge::Below)
                                }
                                _ => None,
                            };
                            let playing = audible && position == QueuePosition::Current;
                            let look = RowLook { playing, drop_line };
                            Self::row(found, index, position, revision, look, cx).into_any_element()
                        }
                        (Some(Slot::Track(_)), None) => div().into_any_element(),
                    })
                    .collect()
            }),
        )
    }
}

impl Render for Aside {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.scrollbar.read(cx).sync();
        let queue = self.queue.read(cx);
        let sections = Sections {
            past: queue.past().len(),
            current: queue.current().is_some(),
            upcoming: queue.upcoming().len(),
            similar: queue.similar().len(),
        };
        let empty = sections.len() == 0;
        if !cx.has_active_drag() {
            self.drop_gap = None;
        }

        if self.past_len != sections.past {
            self.past_len = sections.past;
            self.anchor = true;
        }
        if self.anchor && self.tab == SideTab::Queue {
            self.pin(sections, window, cx);
        }

        div()
            .id("aside")
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .min_w_0()
            .on_drag_move(cx.listener(|this, _: &DragMoveEvent<DraggedPin>, _, cx| {
                if this.drop_gap.take().is_some() {
                    cx.notify();
                }
            }))
            .child(self.header(sections, window, cx))
            .child(
                div()
                    .id("queue-drop")
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .when(self.tab == SideTab::Queue, |this| {
                        this.on_drop(cx.listener(|this, dragged: &DraggedPin, _, cx| {
                            let gap = this.drop_gap.take();
                            if dragged.spot(QUEUE).is_none() {
                                this.enqueue(&dragged.pin, gap, cx);
                            }
                            cx.notify();
                        }))
                    })
                    .when(self.tab == SideTab::Lyrics, |this| {
                        this.child(self.verses(window, cx))
                    })
                    .when(self.tab == SideTab::Queue && empty, |this| {
                        this.child(vacant(t!("queue-empty"), cx).flex_1())
                    })
                    .when(self.tab == SideTab::Queue && !empty, |this| {
                        let gliding = self.scrollbar.clone();

                        this.child(
                            div()
                                .relative()
                                .flex_1()
                                .min_h_0()
                                .child(
                                    div()
                                        .size_full()
                                        .when(effects(), |this| {
                                            this.fade_edges(px(FADE * 0.5), px(FADE))
                                        })
                                        .child(
                                            self.rows(sections, cx)
                                                .px_2()
                                                .pt(px(FADE * 0.5))
                                                .track_scroll(&self.scroll)
                                                .size_full()
                                                .on_scroll_wheel(
                                                    move |event: &ScrollWheelEvent, window, cx| {
                                                        if event.delta.precise() {
                                                            return;
                                                        }
                                                        gliding
                                                            .update(cx, |bar, _| bar.nudge(window));
                                                    },
                                                ),
                                        ),
                                )
                                .child(self.scrollbar.clone()),
                        )
                    })
                    .children(self.follow(cx)),
            )
            .children(self.menu(cx))
    }
}

fn source_link(name: SharedString, to: Destination, cx: &App) -> impl IntoElement {
    let theme = *cx.theme();

    div()
        .id("queue-source")
        .min_w_0()
        .flex_shrink(1.)
        .truncate()
        .text_size(theme.text(Text::Small))
        .text_color(theme.muted_foreground)
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|style| style.text_color(theme.foreground).underline())
        .link(to)
        .child(name)
}

fn fixed_lyrics_lane(rows: &[SharedString], voice: Voice, sung: Sung) -> Div {
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

fn karaoke_lane(
    plan: &Wrapped,
    words: &[music::LyricsWord],
    position: std::time::Duration,
    voice: Voice,
    sung: Sung,
) -> Div {
    let theme = &sung.theme;
    let windows = (0..words.len())
        .map(|word| {
            let (start, end) = karaoke_window(words, word);
            (start, end, word + 1 >= words.len())
        })
        .collect::<Vec<_>>();
    let lit = |shape: ShapedLine, reveal: Vec<RevealRange>| {
        KaraokeText::new(shape, reveal, theme.foreground)
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

struct KaraokeText {
    line: ShapedLine,
    ranges: Vec<RevealRange>,
    foreground: Hsla,
}

struct KaraokeTextLayout {
    line_height: Pixels,
    base: DecorationRun,
    foreground: DecorationRun,
}

impl KaraokeText {
    fn new(line: ShapedLine, ranges: Vec<RevealRange>, foreground: Hsla) -> Self {
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

        for range in self.ranges.iter().copied() {
            if range.end <= range.start {
                continue;
            }

            let mask = ContentMask {
                bounds: Bounds::new(
                    point(bounds.origin.x + range.start, bounds.origin.y),
                    size(range.end - range.start, bounds.size.height),
                ),
            };

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

#[derive(Clone, Copy)]
struct RevealRange {
    start: Pixels,
    end: Pixels,
}

fn revealed(
    plan: &Wrapped,
    units: Range<usize>,
    windows: &[(std::time::Duration, std::time::Duration, bool)],
    position: std::time::Duration,
) -> Vec<RevealRange> {
    let mut ranges: Vec<(Pixels, Pixels)> = Vec::new();
    let mut offset = px(0.);
    for index in units {
        let unit = &plan.units[index];
        for part in &unit.parts {
            let Some(&(start, end, last)) = windows.get(part.word) else {
                continue;
            };

            // A wide phrase fills evenly; the eased curve only reads as a flourish across Latin.
            let even = plan.evenly.get(part.word).copied().unwrap_or(false);
            let share = match even {
                true => progress_between(start, end, position),
                false => swept(start, end, position, last),
            };
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

struct SecondaryLaneLook<'a> {
    dimming: Option<u64>,
    voice: Voice,
    sung: Sung,
    plan: Option<&'a Wrapped>,
}

fn secondary_lyrics_lane(
    lane: &music::LyricsLane,
    line_active: bool,
    line_passed: bool,
    position: std::time::Duration,
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
    let size = theme.text(Text::Body);
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
            |this, text| this.child(romanized_lyrics_lane(text, theme)),
        )
        .into_any_element()
}

fn secondary_lane_started(lane: &music::LyricsLane, position: std::time::Duration) -> bool {
    position >= lane.start
}

fn secondary_karaoke_visible(
    lane: &music::LyricsLane,
    line_active: bool,
    position: std::time::Duration,
) -> bool {
    line_active && secondary_lane_started(lane, position)
}

fn selected_romanization(
    romanized: &Option<music::RomanizedText>,
    scripts: Option<RomanizationScripts>,
) -> Option<String> {
    let romanized = romanized.as_ref()?;
    scripts?
        .contains(romanized.writing_system)
        .then(|| romanized.text.clone())
}

fn romanized_lyrics_lane(text: String, theme: &ui::Theme) -> Div {
    div()
        .text_size(theme.text(Text::Body))
        .text_color(theme.muted_foreground)
        .child(SharedString::from(text))
}

fn karaoke_window(
    words: &[music::LyricsWord],
    index: usize,
) -> (std::time::Duration, std::time::Duration) {
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

/// How far along a transition started at this moment is, asking for frames while
/// it runs.
fn ramp(at: std::time::Instant, window: &mut Window) -> f32 {
    let span = Motion::Base.span().as_secs_f32().max(f32::EPSILON);
    let progress = (at.elapsed().as_secs_f32() / span).clamp(0., 1.);
    if progress < 1. {
        window.request_animation_frame();
    }
    ease_out_expo(progress)
}

/// Scales one line of text without touching the space between it and the next.
fn lifted(row: Div, sung: Sung) -> Div {
    match sung.lift == 1. {
        true => row,
        false => row.layer_scale(sung.lift).layer_scale_origin(sung.from),
    }
}

fn active_verse_size(verse: Pixels) -> Pixels {
    verse + ACTIVE_VERSE_GROWTH
}

#[derive(Clone)]
struct TimingPart {
    word: usize,
    offset: Pixels,
    before: Pixels,
    width: Pixels,
}

#[derive(Clone)]
struct VisualUnit {
    range: Range<usize>,
    width: Pixels,
    parts: Vec<TimingPart>,
}

// Original text remains source of truth; units and rows only hold byte ranges into it.
#[derive(Clone)]
struct Wrapped {
    units: Vec<VisualUnit>,
    rows: Vec<Range<usize>>,
    word_widths: Vec<Pixels>,
    evenly: Vec<bool>,
    text: Vec<SharedString>,
    shapes: Vec<ShapedLine>,
}

fn lyrics_wrap_rows(
    line: &str,
    words: Option<&[music::LyricsWord]>,
    font_size: Pixels,
    width: Pixels,
    window: &mut Window,
) -> Option<Wrapped> {
    (width > px(0.)).then(|| lyrics_plan(line, words, font_size, Some(width), window))
}

fn lyrics_plan(
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
    let evenly = evenly_timed(line, &timing, &units, timing.len());

    Wrapped {
        units,
        rows,
        word_widths,
        evenly,
        text,
        shapes,
    }
}

fn shaped_range(line: &ShapedLine, range: Range<usize>) -> ShapedLine {
    let (_, suffix) = line.split_at(range.start);
    let (slice, _) = suffix.split_at(range.end - range.start);
    slice
}

fn measured_units(
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
fn timing_spans(line: &str, words: &[music::LyricsWord]) -> Vec<Range<usize>> {
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

fn normal_break_ranges(line: &str) -> Vec<Range<usize>> {
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
fn emergency_ranges(
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

fn evenly_timed(
    line: &str,
    timing: &[Range<usize>],
    units: &[VisualUnit],
    words: usize,
) -> Vec<bool> {
    let mut pieces = vec![0usize; words];
    for unit in units {
        for part in &unit.parts {
            pieces[part.word] += 1;
        }
    }
    (0..words)
        .map(|word| {
            pieces[word] > 1
                || timing
                    .get(word)
                    .is_some_and(|range| line[range.clone()].chars().any(wide))
        })
        .collect()
}

fn wide(letter: char) -> bool {
    matches!(letter,
        '\u{2E80}'..='\u{303E}'
        | '\u{3041}'..='\u{33FF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{A000}'..='\u{A4CF}'
        | '\u{AC00}'..='\u{D7AF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FF00}'..='\u{FF60}'
    )
}

fn wrap_unit_widths(units: &[VisualUnit], width: Pixels) -> Vec<Range<usize>> {
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

fn anchored_lyrics_offset(view: Pixels, item: Pixels, height: Pixels, reach: Pixels) -> Pixels {
    let delta = view - item + height * PIN;
    delta.clamp(-reach, px(0.))
}

fn swept(
    start: std::time::Duration,
    end: std::time::Duration,
    position: std::time::Duration,
    tail: bool,
) -> f32 {
    let span = end.saturating_sub(start);
    let progress = match span.is_zero() {
        true => progress_between(start, start + SWEEP_LEAST, position),
        false => {
            let stretch = match tail {
                true => 1.,
                false => SWEEP_STRETCH,
            };
            let endpoint = ease_out_cubic((1. / stretch).min(1.));
            ease_out_cubic((progress_between(start, end, position) / stretch).min(1.)) / endpoint
        }
    };
    progress.clamp(0., 1.)
}

fn progress_between(
    start: std::time::Duration,
    end: std::time::Duration,
    position: std::time::Duration,
) -> f32 {
    if position < start {
        return 0.;
    }
    if position >= end {
        return 1.;
    }
    let span = (end - start).as_secs_f32();
    ((position - start).as_secs_f32() / span).clamp(0., 1.)
}

fn instrumental_gap_before(lines: &[music::LyricsLine], index: usize) -> std::time::Duration {
    let start = lines[index].start;
    match index {
        0 => start,
        _ => {
            let previous = &lines[index - 1];
            start.saturating_sub(previous.sung_end().unwrap_or(previous.start))
        }
    }
}

fn active_instrumental(
    lines: &[music::LyricsLine],
    position: std::time::Duration,
) -> Option<usize> {
    let next_line = lines.iter().position(|line| line.start > position)?;
    let gap = instrumental_gap_before(lines, next_line);
    let start = lines[next_line].start.saturating_sub(gap);
    (gap >= INSTRUMENTAL_BREAK && position >= start).then_some(next_line)
}

fn lyric_row_count(lines: &[music::LyricsLine]) -> usize {
    lines.len()
        + (0..lines.len())
            .filter(|index| instrumental_gap_before(lines, *index) >= INSTRUMENTAL_BREAK)
            .count()
}

fn line_row(lines: &[music::LyricsLine], index: usize) -> usize {
    index
        + (0..=index)
            .filter(|line| instrumental_gap_before(lines, *line) >= INSTRUMENTAL_BREAK)
            .count()
}

fn active_lyrics_row(lines: &[music::LyricsLine], position: std::time::Duration) -> Option<usize> {
    if let Some(index) = sung_line(lines, position) {
        return Some(line_row(lines, index));
    }
    let index = active_instrumental(lines, position)?;
    line_row(lines, index).checked_sub(1)
}

fn sung_line(lines: &[music::LyricsLine], position: std::time::Duration) -> Option<usize> {
    match active_instrumental(lines, position) {
        Some(_) => None,
        None => music::lyrics::active(lines, position),
    }
}

fn lanes_room(
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
            let romanized = selected_romanization(&lane.romanized, scripts)
                .map_or(0, |text| wrapped_rows(&text, size, width, window));
            spoken + romanized
        })
        .sum::<usize>();

    // a lane inherits the line height of the verse, not its own text size
    let gaps = window.rem_size() * LANE_GAP_REM * lanes.len().saturating_sub(1) as f32;

    leading * (rows as f32 + LANE_SLACK) + gaps
}

fn plain_lyrics_rows(
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

fn wrapped_rows(text: &str, size: Pixels, width: Pixels, window: &mut Window) -> usize {
    lyrics_wrap_rows(text, None, size, width, window).map_or(1, |wrapped| wrapped.rows.len().max(1))
}

// culling needs layout
fn adrift(row: impl Styled + IntoElement, shift: Pixels, window: &Window) -> Div {
    let grid = snapped(shift, window);

    div().w_full().flex().flex_col().items_center().child(
        row.top(grid)
            .layer_translate(gpui::point(px(0.), shift - grid)),
    )
}

struct Place {
    top: Pixels,
    height: Pixels,
    travel: f32,
    along: f32,
}

// shift is drawn, not laid out
fn viewport_place(
    scroll: &ScrollHandle,
    row: usize,
    view: Bounds<Pixels>,
    shift: Pixels,
) -> Option<Place> {
    let height = view.size.height;
    let item = scroll.bounds_for_item(row)?;
    if height <= px(0.) {
        return None;
    }
    let top = item.origin.y - view.origin.y + scroll.offset().y + shift;
    let travel = top - height * PIN;
    let reach = height
        * match travel >= px(0.) {
            true => 1. - PIN,
            false => PIN,
        };
    Some(Place {
        top,
        height: item.size.height,
        travel: (travel / reach.max(px(1.))).clamp(-1., 1.),
        along: (top / height).clamp(0., 1.),
    })
}

fn viewport_haze(
    scroll: &ScrollHandle,
    row: usize,
    view: Bounds<Pixels>,
    margin: Pixels,
    drift: Pixels,
) -> f32 {
    let Some(place) = viewport_place(scroll, row, view, drift) else {
        return 0.;
    };
    if place.top + place.height + margin < px(0.) || place.top - margin > view.size.height {
        return 0.;
    }
    place.travel.abs().powf(HAZE)
}

#[derive(Clone, Copy, Default)]
struct Drag {
    step: Pixels,
    beat: f32,
    downward: bool,
    most: Pixels,
}

fn lag_spring(along: f32) -> SpringConfig {
    let frequency = 1. - LAG_STAGGER * along.clamp(0., 1.);
    SpringConfig::new(
        LAG_SPRING.stiffness * frequency * frequency,
        LAG_SPRING.damping * frequency,
        LAG_SPRING.mass,
    )
}

// incoming rows last
fn viewport_along(scroll: &ScrollHandle, row: usize, view: Bounds<Pixels>, downward: bool) -> f32 {
    let Some(place) = viewport_place(scroll, row, view, px(0.)) else {
        return 0.;
    };
    match downward {
        true => place.along,
        false => 1. - place.along,
    }
}

fn line_has_passed(line: &music::LyricsLine, position: std::time::Duration) -> bool {
    line.sung_end().is_some_and(|end| position >= end)
}

fn instrumental_row(progress: f32, past: bool, verse: Pixels, theme: &ui::Theme) -> Div {
    let note_size = verse * 1.;
    div()
        .flex()
        .items_center()
        .gap_2()
        .py(verse * 0.45)
        .children((0..3).map(|index| {
            let note_progress = (progress * 3. - index as f32).clamp(0., 1.);
            let tint = match past {
                true => theme.muted_foreground.opacity(PAST),
                false => mix(
                    theme.muted_foreground.opacity(AHEAD),
                    theme.primary,
                    note_progress,
                ),
            };
            div()
                .size(note_size)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icons::path("icons/music-2.svg"))
                        .size(note_size)
                        .text_color(tint),
                )
        }))
}

fn wordless(key: &'static str, icon: &'static str) -> gpui::AnyElement {
    Vacancy::new(i18n::lookup(key, None))
        .icon(icon)
        .flex_1()
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::{ops::Range, time::Duration};

    use unicode_segmentation::UnicodeSegmentation;

    use music::{LyricsLane, LyricsLine, LyricsWord, Voice};

    use super::{
        QueuePosition, Sections, Slot, TimingPart, VisualUnit, Wrapped, active_lyrics_row,
        anchored_lyrics_offset, emergency_ranges, karaoke_window, lag_spring, line_has_passed,
        line_row, lyric_row_count, measured_units, normal_break_ranges, revealed,
        secondary_karaoke_visible, swept, timing_spans, wrap_unit_widths,
    };
    use gpui::{Pixels, SharedString, px};

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
            evenly: vec![true, true],
            text: vec![SharedString::from("AA BB")],
            shapes: Vec::new(),
        }
    }

    fn slots(sections: Sections) -> Vec<Slot> {
        (0..sections.len()).map(|i| sections.slot(i)).collect()
    }

    #[test]
    fn lays_out_every_section() {
        let sections = Sections {
            past: 2,
            current: true,
            upcoming: 2,
            similar: 2,
        };

        assert_eq!(sections.current_index(), Some(4));
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-history"),
                Slot::Track(QueuePosition::Past(0)),
                Slot::Track(QueuePosition::Past(1)),
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-up-next"),
                Slot::Track(QueuePosition::Upcoming(0)),
                Slot::Track(QueuePosition::Upcoming(1)),
                Slot::Header("queue-similar"),
                Slot::Track(QueuePosition::Similar(0)),
                Slot::Track(QueuePosition::Similar(1)),
            ]
        );
    }

    #[test]
    fn suggests_similar_tracks_without_anything_up_next() {
        let sections = Sections {
            past: 0,
            current: true,
            upcoming: 0,
            similar: 1,
        };

        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-similar"),
                Slot::Track(QueuePosition::Similar(0)),
            ]
        );
    }

    #[test]
    fn drops_headers_for_empty_sections() {
        let sections = Sections {
            past: 0,
            current: true,
            upcoming: 1,
            similar: 0,
        };

        assert_eq!(sections.current_index(), Some(1));
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-up-next"),
                Slot::Track(QueuePosition::Upcoming(0)),
            ]
        );
    }

    #[test]
    fn lays_out_history_without_a_current_track() {
        let sections = Sections {
            past: 1,
            current: false,
            upcoming: 0,
            similar: 0,
        };

        assert_eq!(sections.current_index(), None);
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-history"),
                Slot::Track(QueuePosition::Past(0))
            ]
        );
    }

    #[test]
    fn an_empty_queue_has_no_rows() {
        let sections = Sections {
            past: 0,
            current: false,
            upcoming: 0,
            similar: 0,
        };

        assert_eq!(sections.len(), 0);
        assert_eq!(sections.current_index(), None);
    }

    #[test]
    fn a_long_instrumental_pause_gets_its_own_lyrics_row() {
        let lines = [
            LyricsLine {
                start: Duration::from_secs(2),
                end: Some(Duration::from_secs(5)),
                text: "first".to_owned(),
                romanized: None,
                words: None,
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
        assert_eq!(line_row(&lines, 0), 0);
        assert_eq!(line_row(&lines, 1), 2);
        assert_eq!(active_lyrics_row(&lines, Duration::from_secs(8)), Some(1));
        assert_eq!(active_lyrics_row(&lines, Duration::from_secs(13)), Some(2));
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
    fn lyrics_follow_uses_unscrolled_item_bounds() {
        let offset = anchored_lyrics_offset(px(0.), px(200.), px(100.), px(500.));

        assert_eq!(offset, px(-170.));
    }

    #[test]
    fn lyrics_follow_preserves_a_subpixel_target() {
        let offset = anchored_lyrics_offset(px(0.25), px(200.125), px(100.5), px(500.));

        assert!((offset.as_f32() - -169.725).abs() < 0.001);
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
    fn lyrics_row_springs_stagger_without_changing_their_damping_ratio() {
        let (first_frequency, first_ratio) = lag_spring(0.).canonical();
        let (last_frequency, last_ratio) = lag_spring(1.).canonical();

        assert!(first_frequency > last_frequency);
        assert!((first_ratio - last_ratio).abs() < f32::EPSILON);
        assert!(
            first_ratio < 1.,
            "the lyrics settle should have a subtle overshoot"
        );
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
        let windows = [(
            Duration::from_millis(1500),
            Duration::from_millis(1900),
            true,
        )];
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
    fn positive_sweep_ends_at_provider_end() {
        assert_eq!(
            swept(
                Duration::from_millis(1000),
                Duration::from_millis(1100),
                Duration::from_millis(1100),
                false,
            ),
            1.
        );
        assert_eq!(
            swept(
                Duration::from_millis(1000),
                Duration::from_millis(10_000),
                Duration::from_millis(10_000),
                false,
            ),
            1.
        );
        assert!(
            swept(
                Duration::from_millis(1500),
                Duration::from_millis(2500),
                Duration::from_millis(2250),
                true,
            ) < 1.
        );
    }

    #[test]
    fn overlapping_reveal_ranges_do_not_fill_the_gap() {
        let plan = reveal_plan();
        let windows = [
            (
                Duration::from_millis(1000),
                Duration::from_millis(2000),
                false,
            ),
            (
                Duration::from_millis(1500),
                Duration::from_millis(2500),
                true,
            ),
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
            (
                Duration::from_millis(1000),
                Duration::from_millis(1500),
                false,
            ),
            (
                Duration::from_millis(1500),
                Duration::from_millis(2000),
                true,
            ),
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
        assert!(spans.iter().all(|range| {
            text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
        }));
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

    fn emergency_ranges_for_test(
        text: &str,
        normal: Vec<Range<usize>>,
        width: Pixels,
    ) -> Vec<Range<usize>> {
        emergency_ranges(text, normal, width, |index| px(index as f32))
    }
}
