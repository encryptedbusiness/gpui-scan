use std::{
  cell::{Cell, RefCell},
  collections::{HashMap, HashSet, VecDeque},
  rc::{Rc, Weak},
  thread_local,
  time::Duration,
};

use rustc_hash::FxHashMap;
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
    Self::at(
      location.file(),
      location.line(),
      location.column(),
      label,
      instance,
    )
  }

  pub fn at(
    file: &'static str,
    line: u32,
    column: u32,
    label: SharedString,
    instance: u64,
  ) -> Self {
    Self {
      file,
      line,
      column,
      label,
      instance,
    }
  }
}

struct PendingRender {
  key: TrackKey,
  bounds: Bounds<Pixels>,
  /// `true` for a fresh update, `false` when only the on-screen rectangle moved (for example
  /// because an ancestor scrolled) and the highlight's lifetime must not be refreshed.
  fresh: bool,
}

/// Per-view bookkeeping for the `scan` attribute.
struct Notice {
  /// A notification arrived that has not been consumed by a render yet.
  notified: bool,
  /// The view has a visible outline, so its bounds are tracked every frame until it expires.
  active: bool,
  /// When [`Self::active`] stops being true. Extending it leaves stale expiries behind; the
  /// expiry pass compares against this value before clearing the flag.
  active_until: Instant,
}

/// How often the per-element update count is reset, so the badge reads as updates per second.
const RENDER_WINDOW: Duration = Duration::from_secs(1);

/// How long after a scroll event outlines snap to their target instead of easing toward it.
///
/// Scrolling moves every element at once. Letting the outline lerp there reads as a lag, so it is
/// pinned while the scroll is in flight; element movement outside a scroll keeps the React Scan
/// chase.
const SCROLL_SNAP: Duration = Duration::from_millis(150);

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
  /// Notification flags for views tracked by the `scan` attribute, stored inline so the render
  /// path never touches a per-view allocation.
  notified: FxHashMap<EntityId, Notice>,
  /// When each active view's outline stops being tracked. Kept in notification order so the
  /// per-frame pass only touches entries that expired.
  active_expiries: VecDeque<(Instant, EntityId)>,
  notify_subscriptions: FxHashMap<EntityId, Subscription>,
  signals: FxHashMap<EntityId, Rc<Cell<bool>>>,
  subscriptions: FxHashMap<EntityId, Subscription>,
  /// The last scroll event observed by the overlay. Outlines snap while this is recent.
  last_scroll: Option<Instant>,
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
      notified: FxHashMap::default(),
      active_expiries: VecDeque::new(),
      notify_subscriptions: FxHashMap::default(),
      signals: FxHashMap::default(),
      subscriptions: FxHashMap::default(),
      last_scroll: None,
    }
  }

  pub fn config(&self) -> &Config {
    &self.config
  }

  /// Records that a tracked element was laid out again during the current frame.
  pub fn note_render(&mut self, key: TrackKey, bounds: Bounds<Pixels>) {
    self.pending.push(PendingRender {
      key,
      bounds,
      fresh: true,
    });
  }

  /// Records that an active outline moved without the view changing, such as when an ancestor
  /// scrolled. This updates the rectangle the outline chases without extending its lifetime.
  pub fn note_bounds(&mut self, key: TrackKey, bounds: Bounds<Pixels>) {
    self.pending.push(PendingRender {
      key,
      bounds,
      fresh: false,
    });
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
        if pending.fresh {
          highlight.updated_at = now;
          if now.duration_since(highlight.window_start) >= RENDER_WINDOW {
            highlight.window_start = now;
            highlight.renders = 0;
          }
          highlight.renders += 1;
        }
      } else if pending.fresh {
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

  /// Moves each outline toward the rectangle it was last laid out in.
  ///
  /// During a scroll the outline is pinned to the target; scrolling moves every element at once,
  /// so easing there reads as the overlay lagging behind the content.
  pub fn step_highlights(&mut self, now: Instant) {
    let snap = self
      .last_scroll
      .is_some_and(|scrolled| now.duration_since(scrolled) < SCROLL_SNAP);
    for highlight in self.highlights.values_mut() {
      if snap {
        highlight.bounds = highlight.target;
        continue;
      }
      highlight.bounds.origin.x = px_lerp(highlight.bounds.origin.x, highlight.target.origin.x);
      highlight.bounds.origin.y = px_lerp(highlight.bounds.origin.y, highlight.target.origin.y);
      highlight.bounds.size.width =
        px_lerp(highlight.bounds.size.width, highlight.target.size.width);
      highlight.bounds.size.height =
        px_lerp(highlight.bounds.size.height, highlight.target.size.height);
    }
  }

  pub fn mark_scroll(&mut self, now: Instant) {
    self.last_scroll = Some(now);
  }

  pub fn prune(&mut self, now: Instant) {
    self.prune_active(now);
    let Some(cutoff) = now.checked_sub(HIGHLIGHT_LIFETIME) else {
      return;
    };
    self
      .highlights
      .retain(|_, highlight| highlight.updated_at >= cutoff);
  }

  /// Clears the active flag of every outline whose lifetime has passed.
  ///
  /// Expiries are pushed in notification order. A later notification pushes a later expiry; the
  /// earlier entry is discarded without clearing the flag because `active_until` was extended.
  fn prune_active(&mut self, now: Instant) {
    while let Some((until, entity)) = self.active_expiries.front().copied() {
      if until > now {
        break;
      }
      self.active_expiries.pop_front();
      if let Some(notice) = self.notified.get_mut(&entity)
        && notice.active_until <= now
      {
        notice.active = false;
      }
    }
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

  /// The notification flag of a view, cloned so the caller does not hold the state borrow.
  pub fn signal(&self, entity: EntityId) -> Option<Rc<Cell<bool>>> {
    self.signals.get(&entity).cloned()
  }

  pub fn insert_signal(
    &mut self,
    entity: EntityId,
    signal: Rc<Cell<bool>>,
    subscription: Subscription,
  ) {
    self.signals.insert(entity, signal);
    self.subscriptions.insert(entity, subscription);
  }

  /// Reads and clears the notification flag of a view.
  ///
  /// Returns `(known, fresh, active)`. `fresh` means a notification is being consumed, so the
  /// outline's lifetime should be extended. `active` means the view still has a visible outline
  /// and its bounds should be tracked even though it did not change. The first time a view is
  /// seen it is reported like a mount.
  pub fn take_notified(&mut self, entity: EntityId) -> (bool, bool, bool) {
    match self.notified.get_mut(&entity) {
      Some(notice) => {
        let fresh = notice.notified;
        notice.notified = false;
        (true, fresh, notice.active || fresh)
      }
      None => {
        self.notified.insert(
          entity,
          Notice {
            notified: false,
            active: true,
            active_until: Instant::now(),
          },
        );
        (false, true, true)
      }
    }
  }

  /// Extends the active window of a view until `until`.
  pub fn activate(&mut self, entity: EntityId, until: Instant) {
    if let Some(notice) = self.notified.get_mut(&entity) {
      notice.active = true;
      notice.active_until = until;
    }
    self.active_expiries.push_back((until, entity));
  }

  pub fn mark_notified(&mut self, entity: EntityId) {
    if let Some(notice) = self.notified.get_mut(&entity) {
      notice.notified = true;
    }
  }

  pub fn insert_notify_subscription(&mut self, entity: EntityId, subscription: Subscription) {
    self.notify_subscriptions.insert(entity, subscription);
  }
}

/// Watches a view so that [`Tracked`](crate::Tracked) knows it was notified.
///
/// GPUI rebuilds every view on each frame, so being laid out says nothing about whether a view
/// changed. A notification is the only signal that its own state moved.
pub(crate) fn observe_view<V: Render>(entity: &Entity<V>, cx: &mut App) -> Rc<Cell<bool>> {
  let id = entity.entity_id();

  let existing = {
    let shared = global(cx);
    let state = shared.borrow();
    state.signal(id)
  };
  if let Some(signal) = existing {
    return signal;
  }

  let signal = Rc::new(Cell::new(true));
  let notified = Rc::clone(&signal);
  let subscription = cx.observe(entity, move |_, _| notified.set(true));

  {
    let shared = global(cx);
    let mut state = shared.borrow_mut();
    state.insert_signal(id, signal.clone(), subscription);
  }

  signal
}

pub(crate) struct ScanGlobal {
  state: Rc<RefCell<ScanState>>,
}

thread_local! {
  /// The last state handed out on this thread, keyed by the app it belongs to.
  static STATE: RefCell<Option<(usize, Weak<RefCell<ScanState>>)>> = const { RefCell::new(None) };
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
/// The state is looked up through a per-thread cache keyed by the [`App`] address. The cache holds
/// a [`Weak`] reference, so an app that was dropped and whose address was reused cannot hand out
/// stale state: the weak reference dies with the app's global.
///
/// [`init`]: crate::init
#[inline]
pub(crate) fn global(cx: &mut App) -> Rc<RefCell<ScanState>> {
  let key = cx as *const App as usize;
  let cached = STATE.with(|slot| match &*slot.borrow() {
    Some((cached, weak)) if *cached == key => weak.upgrade(),
    _ => None,
  });
  if let Some(state) = cached {
    return state;
  }

  // `default_global` queues a global-observer effect on every call. The scanner asks for this
  // state once per tracked view per frame, so read the existing global directly and only pay
  // the effect when it has to be created.
  let state = match cx.try_global::<ScanGlobal>() {
    Some(existing) => existing.state.clone(),
    None => {
      cx.set_global(ScanGlobal::default());
      cx.try_global::<ScanGlobal>()
        .expect("the scan global was just installed")
        .state
        .clone()
    }
  };
  STATE.with(|slot| *slot.borrow_mut() = Some((key, Rc::downgrade(&state))));
  state
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

pub(crate) fn note_bounds(cx: &mut App, key: TrackKey, bounds: Bounds<Pixels>) {
  global(cx).borrow_mut().note_bounds(key, bounds);
}

pub(crate) fn mark_notified(cx: &mut App, entity: EntityId) {
  global(cx).borrow_mut().mark_notified(entity);
}

pub(crate) fn note_scroll(cx: &mut App) {
  global(cx).borrow_mut().mark_scroll(Instant::now());
}

fn px_lerp(current: Pixels, target: Pixels) -> Pixels {
  gpui::px(geom::lerp_toward(current.as_f32(), target.as_f32()))
}
