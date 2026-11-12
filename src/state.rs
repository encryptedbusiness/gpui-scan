use std::{
  cell::{Cell, RefCell},
  collections::{HashMap, HashSet, VecDeque},
  rc::Rc,
  time::Duration,
};

use web_time::Instant;

use gpui::{
  App, Bounds, Entity, EntityId, Global, Pixels, Render, SharedString, Subscription, WindowId,
};

use crate::{
  config::Config,
  dock::ScanDock,
  geom::{self, WidgetModel},
};

/// How long a re-render outline stays on screen after the element updated.
///
/// React Scan keeps an outline for 45 frames. At 60fps that is 750ms.
pub(crate) const HIGHLIGHT_LIFETIME: Duration = geom::OUTLINE_LIFETIME;

/// How far back frames are counted for the FPS reading.
const FPS_WINDOW: Duration = Duration::from_secs(1);

/// Identifies one tracked place in the element tree.
///
/// The source location alone is not enough: a list that tracks its rows from a single call site
/// would collapse into one entry, so callers can add an explicit instance key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TrackKey {
  pub file: &'static str,
  pub line: u32,
  pub column: u32,
  pub label: SharedString,
  pub instance: u64,
}

impl TrackKey {
  pub fn new(
    location: &'static std::panic::Location<'static>,
    label: SharedString,
    instance: u64,
  ) -> Self {
    Self {
      file: location.file(),
      line: location.line(),
      column: location.column(),
      label,
      instance,
    }
  }
}

struct PendingRender {
  key: TrackKey,
  bounds: Bounds<Pixels>,
}

/// How often the per-element update count is reset, so the badge reads as updates per second.
const RENDER_WINDOW: Duration = Duration::from_secs(1);

/// An element that updated recently and is currently outlined.
pub(crate) struct Highlight {
  /// The rectangle on screen, chasing [`Self::target`] by a fifth of the gap each frame.
  pub bounds: Bounds<Pixels>,
  /// Where the element was laid out on its latest update.
  pub target: Bounds<Pixels>,
  pub updated_at: Instant,
  /// Updates during the current [`RENDER_WINDOW`], drawn as `Name ×count`.
  pub renders: u32,
  window_start: Instant,
}

/// A rolling frame counter.
///
/// Frames are stamped with the moment the platform delivered them, so the reading tracks what the
/// application actually presented instead of how often the toolbar looked at the clock.
#[derive(Default)]
pub(crate) struct FpsMeter {
  ticks: VecDeque<Instant>,
}

impl FpsMeter {
  pub fn tick(&mut self, now: Instant) {
    self.ticks.push_back(now);
    // A page that has been open for less than the window has nothing older to drop, and the
    // subtraction would underflow on the web clock.
    let Some(cutoff) = now.checked_sub(FPS_WINDOW) else {
      return;
    };
    while self.ticks.front().is_some_and(|tick| *tick < cutoff) {
      self.ticks.pop_front();
    }
  }

  fn span(&self) -> Option<(f32, usize)> {
    let first = *self.ticks.front()?;
    let last = *self.ticks.back()?;
    let seconds = last.duration_since(first).as_secs_f32();
    if seconds <= 0.0 || self.ticks.len() < 2 {
      return None;
    }
    Some((seconds, self.ticks.len() - 1))
  }

  /// Frames per second over the retained window, or `None` before enough frames were seen.
  pub fn fps(&self) -> Option<f32> {
    let (seconds, frames) = self.span()?;
    Some(frames as f32 / seconds)
  }
}

pub(crate) struct ScanState {
  config: Config,
  pending: Vec<PendingRender>,
  highlights: HashMap<TrackKey, Highlight>,
  /// The strong handle keeps the dock alive across frames; the element tree only reborrows it.
  docks: HashMap<WindowId, Entity<ScanDock>>,
  /// Where the toolbar and the open inspector sit in each window.
  pub widget: WidgetModel,
  ticking: HashSet<WindowId>,
  signals: HashMap<EntityId, Rc<Cell<bool>>>,
  subscriptions: HashMap<EntityId, Subscription>,
}

impl ScanState {
  pub(crate) fn new(config: Config) -> Self {
    Self {
      config,
      pending: Vec::new(),
      highlights: HashMap::new(),
      docks: HashMap::new(),
      widget: WidgetModel::default(),
      ticking: HashSet::new(),
      signals: HashMap::new(),
      subscriptions: HashMap::new(),
    }
  }

  pub fn config(&self) -> &Config {
    &self.config
  }

  /// Records that a tracked element was laid out again during the current frame.
  pub fn note_render(&mut self, key: TrackKey, bounds: Bounds<Pixels>) {
    self.pending.push(PendingRender { key, bounds });
  }

  /// Turns everything recorded during the frame into highlights and prunes the expired ones.
  ///
  /// Returns how many outlines are still visible.
  pub fn commit_frame(&mut self, now: Instant) -> usize {
    let mut seen = HashSet::with_capacity(self.pending.len());
    for pending in self.pending.drain(..) {
      if !seen.insert(pending.key.clone()) {
        continue;
      }

      if let Some(highlight) = self.highlights.get_mut(&pending.key) {
        // Keep the on-screen rect so it can chase the new bounds instead of jumping.
        highlight.target = pending.bounds;
        highlight.updated_at = now;
        if now.duration_since(highlight.window_start) >= RENDER_WINDOW {
          highlight.window_start = now;
          highlight.renders = 0;
        }
        highlight.renders += 1;
      } else {
        self.highlights.insert(
          pending.key,
          Highlight {
            bounds: pending.bounds,
            target: pending.bounds,
            updated_at: now,
            renders: 1,
            window_start: now,
          },
        );
      }
    }

    self.prune(now);
    self.highlights.len()
  }

  #[cfg(test)]
  pub fn pending_count(&self) -> usize {
    self.pending.len()
  }

  /// Moves each outline a step toward the rectangle it was last laid out in.
  pub fn step_highlights(&mut self) {
    for highlight in self.highlights.values_mut() {
      highlight.bounds.origin.x = px_lerp(highlight.bounds.origin.x, highlight.target.origin.x);
      highlight.bounds.origin.y = px_lerp(highlight.bounds.origin.y, highlight.target.origin.y);
      highlight.bounds.size.width =
        px_lerp(highlight.bounds.size.width, highlight.target.size.width);
      highlight.bounds.size.height =
        px_lerp(highlight.bounds.size.height, highlight.target.size.height);
    }
  }

  pub fn prune(&mut self, now: Instant) {
    let Some(cutoff) = now.checked_sub(HIGHLIGHT_LIFETIME) else {
      return;
    };
    self
      .highlights
      .retain(|_, highlight| highlight.updated_at >= cutoff);
  }

  pub fn highlights(&self) -> impl Iterator<Item = (&TrackKey, &Highlight)> {
    self.highlights.iter()
  }

  pub fn docks(&self) -> &HashMap<WindowId, Entity<ScanDock>> {
    &self.docks
  }

  pub fn register_dock(&mut self, window: WindowId, dock: Entity<ScanDock>) {
    self.docks.insert(window, dock);
  }

  /// Returns true the first time a window starts its frame loop.
  pub fn start_ticking(&mut self, window: WindowId) -> bool {
    self.ticking.insert(window)
  }
}

/// Watches a view so that [`Tracked`](crate::Tracked) knows it was notified.
///
/// GPUI rebuilds every view on each frame, so being laid out says nothing about whether a view
/// changed. A notification is the only signal that its own state moved.
pub(crate) fn observe_view<V: Render>(entity: &Entity<V>, cx: &mut App) -> Rc<Cell<bool>> {
  let id = entity.entity_id();

  let existing = global(cx).borrow().signals.get(&id).cloned();
  if let Some(signal) = existing {
    return signal;
  }

  let signal = Rc::new(Cell::new(true));
  let notified = Rc::clone(&signal);
  let subscription = cx.observe(entity, move |_, _| notified.set(true));

  {
    let shared = global(cx);
    let mut state = shared.borrow_mut();
    state.signals.insert(id, signal.clone());
    state.subscriptions.insert(id, subscription);
  }

  signal
}

pub(crate) struct ScanGlobal {
  state: Rc<RefCell<ScanState>>,
}

impl Global for ScanGlobal {}

impl Default for ScanGlobal {
  fn default() -> Self {
    Self {
      state: Rc::new(RefCell::new(ScanState::new(Config::default()))),
    }
  }
}

/// The shared scanner state, created on first use so tracking works even without [`init`].
///
/// [`init`]: crate::init
pub(crate) fn global(cx: &mut App) -> Rc<RefCell<ScanState>> {
  cx.default_global::<ScanGlobal>().state.clone()
}

pub(crate) fn install(config: Config, cx: &mut App) {
  if let Some(existing) = cx.try_global::<ScanGlobal>() {
    existing.state.borrow_mut().config = config;
    return;
  }

  cx.set_global(ScanGlobal {
    state: Rc::new(RefCell::new(ScanState::new(config))),
  });
}

pub(crate) fn note_render(cx: &mut App, key: TrackKey, bounds: Bounds<Pixels>) {
  global(cx).borrow_mut().note_render(key, bounds);
}

fn px_lerp(current: Pixels, target: Pixels) -> Pixels {
  gpui::px(geom::lerp_toward(current.as_f32(), target.as_f32()))
}
