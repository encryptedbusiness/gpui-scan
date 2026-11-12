//! Geometry and motion taken from React Scan's widget and outline canvas.
//!
//! The numbers match `packages/scan/src/web/widget` and `packages/scan/src/new-outlines/canvas.ts`:
//! a 24px safe area, a 36px toolbar, a 550×400 inspector, a 250ms
//! `cubic-bezier(0, 0, 0.2, 1)` snap, and outlines that lerp at 0.2 and fade across 45 frames.

use web_time::Instant;

use std::time::Duration;

/// Inset between the widget and the window edge. React Scan's `SAFE_AREA`.
pub(crate) const SAFE_AREA: f32 = 24.0;

/// Toolbar row height. React Scan's `min-h-9` / `max-h-9`.
pub(crate) const TOOLBAR_HEIGHT: f32 = 36.0;

/// Open inspector width. React Scan's `MIN_SIZE.width`.
pub(crate) const EXPANDED_WIDTH: f32 = 550.0;

/// Open inspector height. `calculateBoundedSize` floors at `MIN_SIZE.initialHeight`.
pub(crate) const EXPANDED_HEIGHT: f32 = 400.0;

/// How long a corner snap takes.
pub(crate) const SNAP_DURATION: Duration = Duration::from_millis(250);

/// A drag shorter than this stays where it was released.
pub(crate) const DRAG_COMMIT: f32 = 60.0;

/// How far a tucked tab must be pulled back into the window before it opens.
pub(crate) const COLLAPSE_DRAG: f32 = 50.0;

/// Share of the widget that has to leave the window before it tucks into the edge.
const COLLAPSE_FRACTION: f32 = 0.35;

/// Collapsed tab thickness and length.
const COLLAPSED_SHORT: f32 = 20.0;
const COLLAPSED_LONG: f32 = 48.0;

/// Outline position chases its target by this fraction each frame.
const OUTLINE_LERP: f32 = 0.2;

/// A gap smaller than this snaps to the target instead of lerping.
const OUTLINE_SNAP: f32 = 0.5;

/// React Scan draws an outline for 45 frames. At 60fps that is 750ms.
pub(crate) const OUTLINE_LIFETIME: Duration = Duration::from_millis(750);

/// `rgba(115, 97, 230)` — the outline color in `new-outlines/canvas.ts`.
pub(crate) const OUTLINE_COLOR: u32 = 0x7361e6;

/// Inspect icon while picking or focused. `#8e61e3`.
pub(crate) const INSPECT_ACTIVE: u32 = 0x8e61e3;

/// Idle icon. `#999`.
pub(crate) const ICON_IDLE: u32 = 0x999999;

/// Toggle track when outlines are on. `#5f3f9a`.
pub(crate) const TOGGLE_ON: u32 = 0x5f3f9a;

/// Toggle track when outlines are off. Tailwind `neutral-700`.
pub(crate) const TOGGLE_OFF: u32 = 0x404040;

/// FPS pill background.
pub(crate) const FPS_PILL: u32 = 0x141414;

/// Healthy FPS. `rgb(214, 132, 245)`.
pub(crate) const FPS_GOOD: u32 = 0xd684f5;

/// FPS under 50.
pub(crate) const FPS_WARN: u32 = 0xf59e0b;

/// FPS under 30.
pub(crate) const FPS_BAD: u32 = 0xef4444;

/// Inspector body. `#0a0a0a`.
pub(crate) const BODY: u32 = 0x0a0a0a;

/// Hairline used between the header, body, and toolbar.
pub(crate) const LINE: u32 = 0x222222;

/// Which corner the widget is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Corner {
  TopLeft,
  TopRight,
  BottomLeft,
  BottomRight,
}

/// Whether a tucked tab sits on a vertical or horizontal edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
  Horizontal,
  Vertical,
}

/// The widget is a tab flush with one edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Collapsed {
  pub corner: Corner,
  pub axis: Axis,
}

/// What the widget is showing above the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WidgetView {
  Toolbar,
  Inspector,
}

/// An edge the open widget can be resized from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
  Left,
  Right,
  Top,
  Bottom,
}

/// A rectangle in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

/// CSS `cubic-bezier(0, 0, 0.2, 1)`, the curve React Scan uses for widget motion.
pub(crate) fn ease_widget(x: f32) -> f32 {
  if x <= 0.0 {
    return 0.0;
  }
  if x >= 1.0 {
    return 1.0;
  }

  // B(t) with control points (0, 0), (0, 0), (0.2, 1), (1, 1).
  let mut t = x;
  for _ in 0..8 {
    let inv = 1.0 - t;
    let x_at = 3.0 * inv * t * t * 0.2 + t * t * t;
    let slope = 1.2 * inv * t + 2.4 * t * t;
    if slope.abs() < 1.0e-6 {
      break;
    }
    t = (t - (x_at - x) / slope).clamp(0.0, 1.0);
  }

  let inv = 1.0 - t;
  3.0 * inv * t * t + t * t * t
}

/// One step of the outline canvas lerp. Snaps when the gap is under half a pixel.
pub(crate) fn lerp_toward(start: f32, end: f32) -> f32 {
  let delta = end - start;
  if delta.abs() < OUTLINE_SNAP {
    end
  } else {
    start + delta * OUTLINE_LERP
  }
}

/// Outline opacity. `1` when the element just rendered, `0` after [`OUTLINE_LIFETIME`].
pub(crate) fn outline_alpha(age: Duration) -> f32 {
  let t = age.as_secs_f32() / OUTLINE_LIFETIME.as_secs_f32();
  (1.0 - t).clamp(0.0, 1.0)
}

/// `Name ×count`, truncated the way the outline canvas truncates a label.
pub(crate) fn outline_label(name: &str, count: u32) -> String {
  let part = format!("{name} ×{count}");
  let mut chars = part.chars();
  let mut truncated = String::new();
  for _ in 0..40 {
    match chars.next() {
      Some(ch) => truncated.push(ch),
      None => return part,
    }
  }
  if chars.next().is_some() {
    truncated.push('…');
  }
  truncated
}

/// Top of the 15px label. It sits on the element's top edge, or on the screen top.
pub(crate) fn label_top(element_y: f32) -> f32 {
  let y = element_y - 15.0;
  if y < 0.0 { 0.0 } else { y }
}

/// FPS text color. Under 30 is red, under 50 is amber, otherwise the purple readout.
pub(crate) fn fps_color(fps: f32) -> u32 {
  if fps < 30.0 {
    FPS_BAD
  } else if fps < 50.0 {
    FPS_WARN
  } else {
    FPS_GOOD
  }
}

/// Where a widget of `width`×`height` sits when anchored to `corner`.
pub(crate) fn corner_origin(
  corner: Corner,
  view_w: f32,
  view_h: f32,
  width: f32,
  height: f32,
) -> (f32, f32) {
  let x = match corner {
    Corner::TopLeft | Corner::BottomLeft => SAFE_AREA,
    Corner::TopRight | Corner::BottomRight => view_w - width - SAFE_AREA,
  };
  let y = match corner {
    Corner::TopLeft | Corner::TopRight => SAFE_AREA,
    Corner::BottomLeft | Corner::BottomRight => view_h - height - SAFE_AREA,
  };
  let max_x = (view_w - width).max(0.0);
  let max_y = (view_h - height).max(0.0);
  (x.clamp(0.0, max_x), y.clamp(0.0, max_y))
}

/// The tucked tab's rectangle. It hangs one pixel past the edge, like React Scan.
pub(crate) fn collapsed_rect(collapsed: Collapsed, view_w: f32, view_h: f32) -> Rect {
  let (width, height) = match collapsed.axis {
    Axis::Horizontal => (COLLAPSED_SHORT, COLLAPSED_LONG),
    Axis::Vertical => (COLLAPSED_LONG, COLLAPSED_SHORT),
  };
  let (x, y) = match (collapsed.corner, collapsed.axis) {
    (Corner::TopLeft, Axis::Horizontal) => (-1.0, SAFE_AREA),
    (Corner::BottomLeft, Axis::Horizontal) => (-1.0, view_h - height - SAFE_AREA),
    (Corner::TopRight, Axis::Horizontal) => (view_w - width + 1.0, SAFE_AREA),
    (Corner::BottomRight, Axis::Horizontal) => (view_w - width + 1.0, view_h - height - SAFE_AREA),
    (Corner::TopLeft, Axis::Vertical) => (SAFE_AREA, -1.0),
    (Corner::TopRight, Axis::Vertical) => (view_w - width - SAFE_AREA, -1.0),
    (Corner::BottomLeft, Axis::Vertical) => (SAFE_AREA, view_h - height + 1.0),
    (Corner::BottomRight, Axis::Vertical) => (view_w - width - SAFE_AREA, view_h - height + 1.0),
  };
  Rect {
    x,
    y,
    width,
    height,
  }
}

/// Which corner a drag should settle on. Horizontal motion wins when both axes pass `threshold`.
pub(crate) fn best_corner(
  mouse_x: f32,
  mouse_y: f32,
  start_x: f32,
  start_y: f32,
  view_w: f32,
  view_h: f32,
  threshold: f32,
) -> Corner {
  let dx = mouse_x - start_x;
  let dy = mouse_y - start_y;
  let moving_right = dx > threshold;
  let moving_left = dx < -threshold;
  let moving_down = dy > threshold;
  let moving_up = dy < -threshold;
  let below = mouse_y > view_h / 2.0;
  let right = mouse_x > view_w / 2.0;

  if moving_right || moving_left {
    return match (moving_right, below) {
      (true, true) => Corner::BottomRight,
      (true, false) => Corner::TopRight,
      (false, true) => Corner::BottomLeft,
      (false, false) => Corner::TopLeft,
    };
  }
  if moving_down || moving_up {
    return match (moving_down, right) {
      (true, true) => Corner::BottomRight,
      (true, false) => Corner::BottomLeft,
      (false, true) => Corner::TopRight,
      (false, false) => Corner::TopLeft,
    };
  }
  match (right, below) {
    (true, true) => Corner::BottomRight,
    (true, false) => Corner::TopRight,
    (false, true) => Corner::BottomLeft,
    (false, false) => Corner::TopLeft,
  }
}

/// `Some` when more than 35% of the widget is outside the window.
pub(crate) fn should_collapse(rect: Rect, view_w: f32, view_h: f32) -> Option<Collapsed> {
  let outside_left = (-rect.x).max(0.0);
  let outside_right = (rect.x + rect.width - view_w).max(0.0);
  let outside_top = (-rect.y).max(0.0);
  let outside_bottom = (rect.y + rect.height - view_h).max(0.0);
  let horizontal = (outside_left + outside_right).min(rect.width);
  let vertical = (outside_top + outside_bottom).min(rect.height);
  let area = horizontal * rect.height + vertical * rect.width - horizontal * vertical;
  let total = rect.width * rect.height;
  if total <= 0.0 || area <= total * COLLAPSE_FRACTION {
    return None;
  }

  let center_x = rect.x + rect.width / 2.0;
  let center_y = rect.y + rect.height / 2.0;
  let corner = match (center_x < view_w / 2.0, center_y < view_h / 2.0) {
    (true, true) => Corner::TopLeft,
    (false, true) => Corner::TopRight,
    (true, false) => Corner::BottomLeft,
    (false, false) => Corner::BottomRight,
  };
  let axis = if outside_left.max(outside_right) > outside_top.max(outside_bottom) {
    Axis::Horizontal
  } else {
    Axis::Vertical
  };
  Some(Collapsed { corner, axis })
}

/// Whether a drag on a tucked tab is pulling it back into the window.
pub(crate) fn collapsed_should_expand(collapsed: Collapsed, dx: f32, dy: f32) -> bool {
  match collapsed.axis {
    Axis::Horizontal => match collapsed.corner {
      Corner::TopLeft | Corner::BottomLeft => dx > COLLAPSE_DRAG,
      Corner::TopRight | Corner::BottomRight => dx < -COLLAPSE_DRAG,
    },
    Axis::Vertical => match collapsed.corner {
      Corner::TopLeft | Corner::TopRight => dy > COLLAPSE_DRAG,
      Corner::BottomLeft | Corner::BottomRight => dy < -COLLAPSE_DRAG,
    },
  }
}

/// Widget opacity for the fade-in: hidden for 300ms, then fading for 300ms.
pub(crate) fn fade_opacity(appeared_at: Instant, now: Instant) -> f32 {
  let elapsed = now.duration_since(appeared_at).as_secs_f32() * 1000.0;
  if elapsed < 300.0 {
    0.0
  } else {
    ((elapsed - 300.0) / 300.0).clamp(0.0, 1.0)
  }
}

struct Drag {
  grab_x: f32,
  grab_y: f32,
  start_pointer_x: f32,
  start_pointer_y: f32,
  from_collapsed: bool,
}

struct Resize {
  edge: Edge,
  start_pointer_x: f32,
  start_pointer_y: f32,
  start: Rect,
}

struct Snap {
  from: Rect,
  to: Rect,
  started_at: Instant,
}

/// Placement, drag, snap, and collapse for one window's widget.
pub(crate) struct WidgetModel {
  pub corner: Corner,
  pub view: WidgetView,
  pub collapsed: Option<Collapsed>,
  /// Remembered size of the open inspector, after the user resizes it.
  custom_size: Option<(f32, f32)>,
  anchored: bool,
  placed: bool,
  pub bar_width: f32,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  area_w: f32,
  area_h: f32,
  drag: Option<Drag>,
  resize: Option<Resize>,
  snap: Option<Snap>,
  appeared_at: Option<Instant>,
  inspector_seen: bool,
  inspector_misses: u8,
}

impl Default for WidgetModel {
  fn default() -> Self {
    Self {
      corner: Corner::BottomRight,
      view: WidgetView::Toolbar,
      collapsed: None,
      custom_size: None,
      anchored: true,
      placed: false,
      bar_width: 196.0,
      x: 0.0,
      y: 0.0,
      width: 196.0,
      height: TOOLBAR_HEIGHT,
      area_w: 0.0,
      area_h: 0.0,
      drag: None,
      resize: None,
      snap: None,
      appeared_at: None,
      inspector_seen: false,
      inspector_misses: 0,
    }
  }
}

impl WidgetModel {
  pub fn is_snapping(&self) -> bool {
    self.snap.is_some()
  }

  pub fn is_dragging(&self) -> bool {
    self.drag.is_some()
  }

  pub fn opacity(&self, now: Instant) -> f32 {
    self
      .appeared_at
      .map(|appeared| fade_opacity(appeared, now))
      .unwrap_or(1.0)
  }

  /// Current rectangle in window coordinates.
  pub fn rect(&self) -> Rect {
    Rect {
      x: self.x,
      y: self.y,
      width: self.width,
      height: self.height,
    }
  }

  fn target(&self) -> Rect {
    if self.area_w < 1.0 || self.area_h < 1.0 {
      return self.rect();
    }
    if let Some(collapsed) = self.collapsed {
      return collapsed_rect(collapsed, self.area_w, self.area_h);
    }
    let (width, height) = match self.view {
      WidgetView::Toolbar => (self.bar_width.max(120.0), TOOLBAR_HEIGHT),
      WidgetView::Inspector => self.expanded_size(),
    };
    let (x, y) = corner_origin(self.corner, self.area_w, self.area_h, width, height);
    Rect {
      x,
      y,
      width,
      height,
    }
  }

  fn expanded_size(&self) -> (f32, f32) {
    let available_w = (self.area_w - SAFE_AREA * 2.0).max(TOOLBAR_HEIGHT);
    let available_h = (self.area_h - SAFE_AREA * 2.0).max(TOOLBAR_HEIGHT);
    let (width, height) = self
      .custom_size
      .unwrap_or((EXPANDED_WIDTH, EXPANDED_HEIGHT));
    (width.min(available_w), height.min(available_h))
  }

  fn apply(&mut self, rect: Rect) {
    self.x = rect.x;
    self.y = rect.y;
    self.width = rect.width;
    self.height = rect.height;
  }

  fn begin_snap(&mut self, to: Rect, now: Instant) {
    let from = self.rect();
    let close = (from.x - to.x).abs() < 0.5
      && (from.y - to.y).abs() < 0.5
      && (from.width - to.width).abs() < 0.5
      && (from.height - to.height).abs() < 0.5;
    if close {
      self.apply(to);
      self.snap = None;
      return;
    }
    self.snap = Some(Snap {
      from,
      to,
      started_at: now,
    });
  }

  /// Puts the widget on screen once the window has a real size, and keeps an anchored widget in
  /// its corner when the window or the toolbar width changes.
  pub fn place(&mut self, view_w: f32, view_h: f32, now: Instant) -> bool {
    if view_w < 240.0 || view_h < 120.0 {
      return self.snap.is_some();
    }
    self.area_w = view_w;
    self.area_h = view_h;
    if self.drag.is_some() || self.resize.is_some() {
      return false;
    }
    let to = self.target();
    if !self.placed {
      self.apply(to);
      self.placed = true;
      self.appeared_at.get_or_insert(now);
      return false;
    }
    if self.snap.is_some() {
      return true;
    }
    if !self.anchored {
      return false;
    }
    let delta = (self.x - to.x).abs()
      + (self.y - to.y).abs()
      + (self.width - to.width).abs()
      + (self.height - to.height).abs();
    if delta < 0.5 {
      return false;
    }
    if delta < 48.0 {
      self.apply(to);
      return false;
    }
    self.begin_snap(to, now);
    true
  }

  /// Advances a snap. Returns true while the widget is still moving.
  pub fn step(&mut self, now: Instant) -> bool {
    let Some(snap) = &self.snap else {
      return false;
    };
    let t = now.duration_since(snap.started_at).as_secs_f32() / SNAP_DURATION.as_secs_f32();
    let eased = ease_widget(t);
    let from = snap.from;
    let to = snap.to;
    if t >= 1.0 {
      self.apply(to);
      self.snap = None;
      return false;
    }
    self.x = from.x + (to.x - from.x) * eased;
    self.y = from.y + (to.y - from.y) * eased;
    self.width = from.width + (to.width - from.width) * eased;
    self.height = from.height + (to.height - from.height) * eased;
    true
  }

  pub fn set_area(&mut self, width: f32, height: f32) {
    self.area_w = width;
    self.area_h = height;
  }

  pub fn set_bar_width(&mut self, width: f32) -> bool {
    if (self.bar_width - width).abs() < 0.5 {
      return false;
    }
    self.bar_width = width;
    true
  }

  /// Drops the open panel and parks the widget on the toolbar while an element is being picked.
  ///
  /// The toolbar itself is not drawn during the pick: GPUI's inspector hit-tests every element,
  /// so the overlay has to leave the tree. The rectangle is already the toolbar's, ready for when
  /// picking ends.
  pub fn prepare_for_pick(&mut self) {
    self.view = WidgetView::Toolbar;
    self.collapsed = None;
    self.drag = None;
    self.resize = None;
    self.anchored = true;
    self.snap = None;
    if self.placed {
      let target = self.target();
      self.apply(target);
    }
  }

  pub fn has_resize(&self) -> bool {
    self.resize.is_some()
  }

  pub fn set_view(&mut self, view: WidgetView, now: Instant) {
    if self.view == view && self.collapsed.is_none() {
      return;
    }
    self.view = view;
    self.collapsed = None;
    self.anchored = true;
    self.drag = None;
    self.resize = None;
    if self.placed {
      self.begin_snap(self.target(), now);
    }
  }

  /// Notices when the inspector renderer stopped painting, so a shortcut that closes it does not
  /// leave the toolbar hidden. Returns true when the widget fell back to the toolbar.
  pub fn note_dock_frame(&mut self, now: Instant) -> bool {
    if self.view != WidgetView::Inspector {
      self.inspector_seen = false;
      self.inspector_misses = 0;
      return false;
    }
    let reset = if self.inspector_seen {
      self.inspector_misses = 0;
      false
    } else {
      self.inspector_misses = self.inspector_misses.saturating_add(1);
      self.inspector_misses > 1
    };
    self.inspector_seen = false;
    if !reset {
      return false;
    }
    self.view = WidgetView::Toolbar;
    self.inspector_misses = 0;
    self.anchored = true;
    if self.placed {
      self.begin_snap(self.target(), now);
    }
    true
  }

  pub fn note_inspector_paint(&mut self) {
    self.inspector_seen = true;
  }

  pub fn start_drag(&mut self, pointer_x: f32, pointer_y: f32) {
    if self.drag.is_some() || self.resize.is_some() {
      return;
    }
    self.snap = None;
    self.drag = Some(Drag {
      grab_x: pointer_x - self.x,
      grab_y: pointer_y - self.y,
      start_pointer_x: pointer_x,
      start_pointer_y: pointer_y,
      from_collapsed: self.collapsed.is_some(),
    });
  }

  pub fn drag_to(&mut self, pointer_x: f32, pointer_y: f32, now: Instant) {
    let (from_collapsed, grab_x, grab_y) = {
      let Some(drag) = &self.drag else {
        return;
      };
      (drag.from_collapsed, drag.grab_x, drag.grab_y)
    };
    if from_collapsed {
      return;
    }
    self.x = pointer_x - grab_x;
    self.y = pointer_y - grab_y;
    let rect = self.rect();
    if let Some(collapsed) = should_collapse(rect, self.area_w, self.area_h) {
      self.drag = None;
      self.collapsed = Some(collapsed);
      self.corner = collapsed.corner;
      self.anchored = true;
      self.begin_snap(self.target(), now);
    }
  }

  pub fn end_drag(&mut self, pointer_x: f32, pointer_y: f32, now: Instant) {
    let Some(drag) = self.drag.take() else {
      return;
    };
    let dx = pointer_x - drag.start_pointer_x;
    let dy = pointer_y - drag.start_pointer_y;
    let distance = (dx * dx + dy * dy).sqrt();

    if drag.from_collapsed {
      let expand = distance < 10.0
        || self
          .collapsed
          .is_some_and(|collapsed| collapsed_should_expand(collapsed, dx, dy));
      if expand {
        self.collapsed = None;
        self.anchored = true;
        self.begin_snap(self.target(), now);
      }
      return;
    }

    self.x = pointer_x - drag.grab_x;
    self.y = pointer_y - drag.grab_y;
    if let Some(collapsed) = should_collapse(self.rect(), self.area_w, self.area_h) {
      self.collapsed = Some(collapsed);
      self.corner = collapsed.corner;
      self.anchored = true;
      self.begin_snap(self.target(), now);
      return;
    }
    if distance < DRAG_COMMIT {
      self.anchored = false;
      return;
    }

    let threshold = if self.view == WidgetView::Inspector {
      80.0
    } else {
      40.0
    };
    self.corner = best_corner(
      pointer_x,
      pointer_y,
      drag.start_pointer_x,
      drag.start_pointer_y,
      self.area_w,
      self.area_h,
      threshold,
    );
    self.anchored = true;
    self.begin_snap(self.target(), now);
  }

  pub fn start_resize(&mut self, edge: Edge, pointer_x: f32, pointer_y: f32) {
    if self.view == WidgetView::Toolbar || self.collapsed.is_some() {
      return;
    }
    self.snap = None;
    self.drag = None;
    self.anchored = false;
    self.resize = Some(Resize {
      edge,
      start_pointer_x: pointer_x,
      start_pointer_y: pointer_y,
      start: self.rect(),
    });
  }

  pub fn resize_to(&mut self, pointer_x: f32, pointer_y: f32) {
    let (dx, dy, start, edge) = {
      let Some(resize) = &self.resize else {
        return;
      };
      (
        pointer_x - resize.start_pointer_x,
        pointer_y - resize.start_pointer_y,
        resize.start,
        resize.edge,
      )
    };
    let min_w = EXPANDED_WIDTH.min((self.area_w - SAFE_AREA * 2.0).max(160.0));
    let min_h = EXPANDED_HEIGHT.min((self.area_h - SAFE_AREA * 2.0).max(120.0));
    let max_w = (self.area_w - SAFE_AREA).max(min_w);
    let max_h = (self.area_h - SAFE_AREA).max(min_h);

    let mut rect = start;
    match edge {
      Edge::Right => {
        rect.width = (start.width + dx).clamp(min_w, (max_w - start.x).max(min_w));
      }
      Edge::Left => {
        let right = start.x + start.width;
        let x = (start.x + dx).clamp(SAFE_AREA.min(right - min_w), right - min_w);
        rect.x = x;
        rect.width = right - x;
      }
      Edge::Bottom => {
        rect.height = (start.height + dy).clamp(min_h, (max_h - start.y).max(min_h));
      }
      Edge::Top => {
        let bottom = start.y + start.height;
        let y = (start.y + dy).clamp(SAFE_AREA.min(bottom - min_h), bottom - min_h);
        rect.y = y;
        rect.height = bottom - y;
      }
    }
    self.apply(rect);
    self.custom_size = Some((rect.width, rect.height));
  }

  pub fn end_pointer(&mut self, pointer_x: f32, pointer_y: f32, now: Instant) {
    if self.resize.take().is_some() {
      return;
    }
    self.end_drag(pointer_x, pointer_y, now);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_widget_curve_starts_fast_and_lands_on_the_ends() {
    assert_eq!(ease_widget(0.0), 0.0);
    assert_eq!(ease_widget(1.0), 1.0);
    let mid = ease_widget(0.5);
    assert!(
      mid > 0.7 && mid < 1.0,
      "ease-out should be ahead of linear, got {mid}"
    );
  }

  #[test]
  fn outline_lerp_chases_then_snaps() {
    assert!((lerp_toward(0.0, 10.0) - 2.0).abs() < 0.001);
    assert_eq!(lerp_toward(0.0, 0.4), 0.4);
  }

  #[test]
  fn outline_alpha_fades_across_the_lifetime() {
    assert_eq!(outline_alpha(Duration::ZERO), 1.0);
    assert!((outline_alpha(OUTLINE_LIFETIME / 2) - 0.5).abs() < 0.001);
    assert_eq!(outline_alpha(OUTLINE_LIFETIME), 0.0);
  }

  #[test]
  fn outline_labels_use_the_canvas_format() {
    assert_eq!(outline_label("Counter", 3), "Counter ×3");
    let long = outline_label(&"A".repeat(50), 12);
    assert!(long.ends_with('…'));
    assert!(long.chars().count() <= 41);
  }

  #[test]
  fn fps_color_follows_the_react_scan_thresholds() {
    assert_eq!(fps_color(29.0), FPS_BAD);
    assert_eq!(fps_color(49.0), FPS_WARN);
    assert_eq!(fps_color(60.0), FPS_GOOD);
  }

  #[test]
  fn the_default_corner_is_the_bottom_right_inset() {
    let (x, y) = corner_origin(Corner::BottomRight, 800.0, 600.0, 196.0, 36.0);
    assert_eq!(x, 800.0 - 196.0 - SAFE_AREA);
    assert_eq!(y, 600.0 - 36.0 - SAFE_AREA);
  }

  #[test]
  fn a_mostly_offscreen_widget_tucks_into_the_edge() {
    let collapsed = should_collapse(
      Rect {
        x: -80.0,
        y: 200.0,
        width: 100.0,
        height: 36.0,
      },
      800.0,
      600.0,
    );
    assert_eq!(
      collapsed,
      Some(Collapsed {
        corner: Corner::TopLeft,
        axis: Axis::Horizontal,
      })
    );
  }

  #[test]
  fn a_long_drag_snaps_to_the_corner_the_pointer_moved_toward() {
    let mut widget = WidgetModel::default();
    let now = Instant::now();
    widget.place(1000.0, 800.0, now);
    let start_x = widget.x + 20.0;
    let start_y = widget.y + 10.0;
    widget.start_drag(start_x, start_y);
    widget.drag_to(180.0, 120.0, now);
    widget.end_drag(180.0, 120.0, now);
    assert_eq!(widget.corner, Corner::TopLeft);
    assert!(widget.is_snapping());
  }

  #[test]
  fn a_short_drag_stays_where_it_was_released() {
    let mut widget = WidgetModel::default();
    let now = Instant::now();
    widget.place(1000.0, 800.0, now);
    let origin_x = widget.x;
    let origin_y = widget.y;
    widget.start_drag(origin_x, origin_y);
    widget.drag_to(origin_x + 30.0, origin_y - 10.0, now);
    widget.end_drag(origin_x + 30.0, origin_y - 10.0, now);
    assert!((widget.x - (origin_x + 30.0)).abs() < 0.1);
    assert!(!widget.is_snapping());
    assert!(!widget.anchored);
  }
}
