//! The React Scan widget: toolbar, outlines, and the shell the inspector grows out of.

use std::time::Duration;

use web_time::Instant;

use gpui::{
  AnyElement, App, AppContext as _, Bounds, BoxShadow, ClickEvent, Context, DispatchPhase, Div,
  Empty, Entity, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
  Point, Render, ScrollEvent, SharedString, WeakEntity, Window, canvas, div, hsla, prelude::*, px,
  rgb,
};

use crate::{
  config::Config,
  geom::{
    self, Collapsed, Edge, TOOLBAR_HEIGHT, WidgetModel, WidgetView, fps_color, label_top,
    outline_alpha, outline_label,
  },
  icons,
  state::{self, FpsMeter},
};

/// How long the copied check stays before the widget closes. React Scan's
/// `COPY_FEEDBACK_DURATION_MS`.
const COPY_FEEDBACK: Duration = Duration::from_millis(600);

/// How often the FPS number repaints when nothing else is moving.
const READOUT_INTERVAL: Duration = Duration::from_millis(200);

const MIN_AREA_WIDTH: f32 = 240.0;
const MIN_AREA_HEIGHT: f32 = 120.0;

/// The floating widget for one window.
///
/// Placement lives on [`WidgetModel`]. This view owns the FPS meter, the outline switch, and the
/// picker, and it is what the per-frame callback wakes up.
pub(crate) struct ScanDock {
  /// The overlay's laid-out size. Ignored until the window has a real viewport.
  measured: Option<gpui::Size<Pixels>>,
  /// True from the click that started picking until an element is selected or picking is cancelled.
  armed: bool,
  outline: bool,
  /// Knob position, 0 at the off stop and 1 at the on stop.
  knob: f32,
  knob_from: f32,
  knob_started: Option<Instant>,
  fps: FpsMeter,
  copied_at: Option<Instant>,
  last_notify: Instant,
}

/// Where and how big the shell is, already in the parent's coordinate space.
pub(crate) struct Chrome {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub opacity: f32,
  pub collapsed: Option<Collapsed>,
  pub hug: bool,
  pub show_resize: bool,
  pub dragging: bool,
  pub view: WidgetView,
}

/// Toolbar state copied out so the shell can be built without borrowing the dock.
pub(crate) struct Bar {
  pub outline: bool,
  pub knob: f32,
  pub inspect_color: u32,
  pub inspect_focused: bool,
  pub fps_text: SharedString,
  pub fps_color: u32,
  pub copied: bool,
}

/// Header and body shown above the toolbar. Both are clipped until the shell is taller than the
/// toolbar, which is how the inspector grows out of the bar.
pub(crate) struct WidgetContent {
  pub header: Option<AnyElement>,
  pub body: Option<AnyElement>,
}

impl ScanDock {
  fn new(config: &Config, _cx: &mut Context<Self>) -> Self {
    let outline = config.outlines;
    let knob = if outline { 1.0 } else { 0.0 };
    Self {
      measured: None,
      armed: false,
      outline,
      knob,
      knob_from: knob,
      knob_started: None,
      fps: FpsMeter::default(),
      copied_at: None,
      last_notify: Instant::now(),
    }
  }

  #[cfg(test)]
  pub(crate) fn is_armed(&self) -> bool {
    self.armed
  }

  pub(crate) fn copied(&self, now: Instant) -> bool {
    self
      .copied_at
      .is_some_and(|at| now.duration_since(at) < COPY_FEEDBACK)
  }

  fn area(&self, window: &Window) -> (f32, f32) {
    if let Some(size) = self.measured {
      let width = size.width.as_f32();
      let height = size.height.as_f32();
      if width >= MIN_AREA_WIDTH && height >= MIN_AREA_HEIGHT {
        return (width, height);
      }
    }
    let viewport = window.viewport_size();
    (viewport.width.as_f32(), viewport.height.as_f32())
  }

  fn with_model<R>(
    &self,
    window: &Window,
    cx: &mut App,
    f: impl FnOnce(&mut WidgetModel) -> R,
  ) -> R {
    let (width, height) = self.area(window);
    let shared = state::global(cx);
    let mut scan = shared.borrow_mut();
    scan.widget.set_area(width, height);
    f(&mut scan.widget)
  }

  fn toggle_outline(&mut self, cx: &mut Context<Self>) {
    self.outline = !self.outline;
    self.knob_from = self.knob;
    self.knob_started = Some(Instant::now());
    cx.notify();
  }

  /// The knob eases across 300ms, matching the CSS transition on React Scan's switch.
  fn step_knob(&mut self, now: Instant) -> bool {
    let target = if self.outline { 1.0 } else { 0.0 };
    let Some(started) = self.knob_started else {
      self.knob = target;
      return false;
    };
    let t = (now.duration_since(started).as_secs_f32() / 0.3).clamp(0.0, 1.0);
    self.knob = self.knob_from + (target - self.knob_from) * geom::ease_widget(t);
    if t >= 1.0 {
      self.knob = target;
      self.knob_started = None;
      false
    } else {
      true
    }
  }

  fn finish_pick(&mut self, now: Instant, window: &Window, cx: &mut Context<Self>) {
    if self.armed && !window.is_inspector_picking(cx) {
      self.armed = false;
      state::global(cx)
        .borrow_mut()
        .widget
        .set_view(WidgetView::Inspector, now);
    }
  }

  /// Starts picking. Clicking again while a pick is in progress cancels it, the way the inspect
  /// button does in React Scan.
  fn pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if window.is_inspector_picking(cx) {
      self.cancel_pick(window, cx);
      return;
    }

    // The overlay cannot stay in the tree during a pick: GPUI hit-tests every element. Park the
    // rectangle on the toolbar so cancelling reveals the bar instead of the open panel.
    state::global(cx).borrow_mut().widget.prepare_for_pick();

    // Toggling twice restarts picking when the panel is already open. If that left it closed, a
    // third toggle opens a fresh inspector, which starts in picking mode.
    window.toggle_inspector(cx);
    window.toggle_inspector(cx);
    if !window.is_inspector_picking(cx) {
      window.toggle_inspector(cx);
    }

    self.armed = true;
    cx.notify();
  }

  fn cancel_pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if !self.armed && !window.is_inspector_picking(cx) {
      return;
    }
    self.armed = false;
    if window.is_inspector_picking(cx) {
      window.toggle_inspector(cx);
    }
    state::global(cx)
      .borrow_mut()
      .widget
      .set_view(WidgetView::Toolbar, Instant::now());
    cx.notify();
  }

  /// Escape leaves picking, or returns a focused element to picking.
  fn on_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if window.is_inspector_picking(cx) || self.armed {
      self.cancel_pick(window, cx);
      return;
    }
    let view = state::global(cx).borrow().widget.view;
    match view {
      WidgetView::Inspector => self.pick(window, cx),
      WidgetView::Toolbar => {}
    }
  }

  fn copy_source(&mut self, text: String, cx: &mut Context<Self>) {
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
    self.copied_at = Some(Instant::now());
    cx.notify();
  }

  /// Closes the open panel. The check icon uses the same path after 600ms.
  fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.copied_at = None;
    let inspector = state::global(cx).borrow().widget.view == WidgetView::Inspector;
    state::global(cx)
      .borrow_mut()
      .widget
      .set_view(WidgetView::Toolbar, Instant::now());
    if inspector {
      window.toggle_inspector(cx);
    }
    cx.notify();
  }

  fn start_drag(&mut self, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
    self.with_model(window, cx, |widget| {
      widget.start_drag(position.x.as_f32(), position.y.as_f32());
    });
    cx.notify();
  }

  fn start_resize(
    &mut self,
    edge: Edge,
    position: Point<Pixels>,
    window: &Window,
    cx: &mut Context<Self>,
  ) {
    self.with_model(window, cx, |widget| {
      widget.start_resize(edge, position.x.as_f32(), position.y.as_f32());
    });
    cx.notify();
  }

  fn pointer_move(&mut self, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
    let now = Instant::now();
    let moved = self.with_model(window, cx, |widget| {
      if widget.has_resize() {
        widget.resize_to(position.x.as_f32(), position.y.as_f32());
        true
      } else if widget.is_dragging() {
        widget.drag_to(position.x.as_f32(), position.y.as_f32(), now);
        true
      } else {
        false
      }
    });
    if moved {
      cx.notify();
    }
  }

  fn pointer_up(&mut self, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
    let now = Instant::now();
    let acted = self.with_model(window, cx, |widget| {
      let active = widget.is_dragging() || widget.has_resize();
      if active {
        widget.end_pointer(position.x.as_f32(), position.y.as_f32(), now);
      }
      active
    });
    if acted {
      cx.notify();
    }
  }

  pub(crate) fn bar(&self, view: WidgetView, now: Instant) -> Bar {
    let (fps_text, fps_color) = match self.fps.fps() {
      Some(fps) => (
        SharedString::from(format!("{}", fps.round() as u32)),
        fps_color(fps),
      ),
      None => (SharedString::from("—"), geom::ICON_IDLE),
    };
    let focused = view == WidgetView::Inspector;
    Bar {
      outline: self.outline,
      knob: self.knob,
      inspect_color: if self.armed || focused {
        geom::INSPECT_ACTIVE
      } else {
        geom::ICON_IDLE
      },
      inspect_focused: focused,
      fps_text,
      fps_color,
      copied: self.copied(now),
    }
  }

  /// Advances the FPS window, the snap, and the outline fade, and decides whether to repaint.
  fn advance(&mut self, now: Instant, window: &mut Window, cx: &mut Context<Self>) {
    self.fps.tick(now);
    let knob = self.step_knob(now);
    self.finish_pick(now, window, cx);

    let dismiss = self
      .copied_at
      .is_some_and(|at| now.duration_since(at) >= COPY_FEEDBACK);
    if dismiss {
      self.dismiss(window, cx);
    }

    let (width, height) = self.area(window);
    let (fading, moving, pointer, opacity) = {
      let shared = state::global(cx);
      let mut scan = shared.borrow_mut();
      let fading = scan.commit_frame(now) > 0;
      scan.step_highlights(now);
      let moving = scan.widget.step(now) || scan.widget.place(width, height, now);
      let pointer = scan.widget.is_dragging() || scan.widget.has_resize();
      let opacity = scan.widget.opacity(now);
      (fading, moving, pointer, opacity)
    };

    let due = now.duration_since(self.last_notify) >= READOUT_INTERVAL;
    if fading || moving || pointer || knob || opacity < 1.0 || self.armed || dismiss || due {
      self.last_notify = now;
      cx.notify();
    }
  }
}

impl Render for ScanDock {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    // Every element takes part in picking, so the overlay leaves the tree until a selection lands.
    if window.is_inspector_picking(cx) {
      return Empty.into_any_element();
    }

    let now = Instant::now();
    let entity = cx.entity();
    self.finish_pick(now, window, cx);

    let (width, height) = self.area(window);
    let (chrome, highlights, paint_shell, config) = {
      let shared = state::global(cx);
      let mut scan = shared.borrow_mut();
      scan.prune(now);
      scan.widget.note_dock_frame(now);
      scan.widget.place(width, height, now);
      let paint_shell = scan.widget.view != WidgetView::Inspector;
      let chrome = chrome_from(&scan.widget, scan.widget.x, now);
      let mut highlights = scan
        .highlights()
        .map(|(key, highlight)| DrawnHighlight {
          label: key.label.clone(),
          bounds: highlight.bounds,
          renders: highlight.renders,
          updated_at: highlight.updated_at,
          area: highlight.bounds.size.width.as_f32() * highlight.bounds.size.height.as_f32(),
        })
        .collect::<Vec<_>>();
      highlights.sort_by(|left, right| right.area.total_cmp(&left.area));
      let config = scan.config().clone();
      (chrome, highlights, paint_shell, config)
    };

    let bar = self.bar(chrome.view, now);
    let shell = paint_shell.then(|| {
      let content = WidgetContent {
        header: None,
        body: None,
      };
      render_chrome(entity.clone(), chrome, bar, content, &config)
    });

    div()
      .absolute()
      .top_0()
      .left_0()
      .size_full()
      .children(
        highlights
          .into_iter()
          .filter(|_| self.outline)
          .filter_map(|highlight| render_highlight(highlight, now)),
      )
      .child(render_listeners(entity))
      .children(shell)
      .into_any_element()
  }
}

struct DrawnHighlight {
  label: SharedString,
  bounds: Bounds<Pixels>,
  renders: u32,
  updated_at: Instant,
  area: f32,
}

pub(crate) fn chrome_from(model: &WidgetModel, x: f32, now: Instant) -> Chrome {
  let collapsed = model.collapsed;
  let hug = collapsed.is_none() && model.view == WidgetView::Toolbar && !model.is_snapping();
  let show_resize = collapsed.is_none()
    && model.view != WidgetView::Toolbar
    && model.height > TOOLBAR_HEIGHT + 48.0;
  Chrome {
    x,
    y: model.y,
    width: model.width,
    height: model.height,
    opacity: model.opacity(now),
    collapsed,
    hug,
    show_resize,
    dragging: model.is_dragging(),
    view: model.view,
  }
}

fn render_highlight(highlight: DrawnHighlight, now: Instant) -> Option<AnyElement> {
  let alpha = outline_alpha(now.duration_since(highlight.updated_at));
  if alpha <= 0.01 {
    return None;
  }
  let color = rgb(geom::OUTLINE_COLOR);
  let top = label_top(highlight.bounds.origin.y.as_f32()) - highlight.bounds.origin.y.as_f32();
  Some(
    div()
      .absolute()
      .left(highlight.bounds.origin.x)
      .top(highlight.bounds.origin.y)
      .w(highlight.bounds.size.width)
      .h(highlight.bounds.size.height)
      .border(px(1.0))
      .border_color(color.opacity(alpha))
      .bg(color.opacity(alpha * 0.1))
      .child(
        div()
          .absolute()
          .left(px(0.0))
          .top(px(top))
          .h(px(15.0))
          .px(px(2.0))
          .flex()
          .items_center()
          .bg(color.opacity(alpha))
          .text_size(px(11.0))
          .text_color(rgb(0xffffff).opacity(alpha))
          .child(SharedString::from(outline_label(
            &highlight.label,
            highlight.renders,
          ))),
      )
      .into_any_element(),
  )
}

fn render_listeners(dock: Entity<ScanDock>) -> impl IntoElement {
  let measure = dock.downgrade();
  let movement = dock.downgrade();
  let release = dock.downgrade();
  let scrolling = dock.downgrade();

  canvas(
    move |bounds, _, cx| {
      if let Some(dock) = measure.upgrade() {
        dock.update(cx, |dock, _| dock.measured = Some(bounds.size));
      }
    },
    move |_, _, window, _| {
      let movement = movement.clone();
      window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
          return;
        }
        let Some(dock) = movement.upgrade() else {
          return;
        };
        let position = event.position;
        dock.update(cx, |dock, cx| dock.pointer_move(position, window, cx));
      });

      let release = release.clone();
      window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
          return;
        }
        let Some(dock) = release.upgrade() else {
          return;
        };
        let position = event.position;
        dock.update(cx, |dock, cx| dock.pointer_up(position, window, cx));
      });

      // Outlines should stick to their content while a scroll is in flight instead of easing
      // behind it. The event is global, so every scrollable area in the window is covered.
      let scrolling = scrolling.clone();
      window.on_mouse_event(move |_: &ScrollEvent, phase, _, cx| {
        if phase == DispatchPhase::Bubble && scrolling.upgrade().is_some() {
          state::note_scroll(cx);
        }
      });
    },
  )
  .absolute()
  .top_0()
  .left_0()
  .size_full()
}

fn shell_shadow() -> Vec<BoxShadow> {
  vec![BoxShadow::new(px(0.0), px(4.0), hsla(0.0, 0.0, 0.0, 0.2)).blur_radius(px(12.0))]
}

/// The widget shell. `chrome.x` is already in the parent element's coordinates: window space for
/// the overlay, inspector-column space when the panel paints it.
pub(crate) fn render_chrome(
  dock: Entity<ScanDock>,
  chrome: Chrome,
  bar: Bar,
  content: WidgetContent,
  config: &Config,
) -> AnyElement {
  if let Some(collapsed) = chrome.collapsed {
    return collapsed_tab(dock, &chrome, collapsed, config);
  }

  let body_height = (chrome.height - TOOLBAR_HEIGHT).max(0.0);
  let reveal = ((body_height - 8.0) / 28.0).clamp(0.0, 1.0);
  let mut shell = div()
    .id("gpui-scan-dock")
    .debug_selector(|| "gpui-scan-dock".into())
    .occlude()
    .absolute()
    .left(px(chrome.x))
    .top(px(chrome.y))
    .h(px(chrome.height))
    .when(!chrome.hug, |this| this.w(px(chrome.width)))
    .overflow_hidden()
    .flex()
    .flex_col()
    .rounded(px(8.0))
    .bg(rgb(config.background))
    .text_color(rgb(config.text))
    .text_size(px(13.0))
    .opacity(chrome.opacity)
    .shadow(shell_shadow())
    .when(chrome.dragging, |this| this.cursor_grabbing())
    .when(!chrome.dragging, |this| this.cursor_move())
    .on_mouse_down(MouseButton::Left, {
      let dock = dock.clone();
      move |event: &MouseDownEvent, window, cx| {
        let position = event.position;
        dock.update(cx, |dock, cx| dock.start_drag(position, window, cx));
      }
    });

  if body_height > 1.0 {
    shell = shell.child(render_body(body_height, reveal, content));
  }
  shell = shell.child(render_toolbar(dock.clone(), bar));
  if chrome.hug {
    shell = shell.child(render_measure(dock.clone()));
  }
  if chrome.show_resize {
    shell = shell.children(resize_handles(dock));
  }
  shell.into_any_element()
}

fn render_body(height: f32, reveal: f32, content: WidgetContent) -> Div {
  let mut column = div()
    .h(px(height))
    .w_full()
    .flex_shrink_0()
    .overflow_hidden()
    .flex()
    .flex_col()
    .bg(rgb(geom::BODY))
    .border_b_1()
    .border_color(rgb(geom::LINE))
    .opacity(reveal);
  if let Some(header) = content.header {
    column = column.child(header);
  }
  if let Some(body) = content.body {
    column = column.child(
      div()
        .id("gpui-scan-body")
        .debug_selector(|| "gpui-scan-body".into())
        .w_full()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .cursor_default()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(body),
    );
  }
  column
}

fn collapsed_tab(
  dock: Entity<ScanDock>,
  chrome: &Chrome,
  collapsed: Collapsed,
  config: &Config,
) -> AnyElement {
  div()
    .id("gpui-scan-dock")
    .debug_selector(|| "gpui-scan-dock".into())
    .occlude()
    .absolute()
    .left(px(chrome.x))
    .top(px(chrome.y))
    .w(px(chrome.width))
    .h(px(chrome.height))
    .rounded(px(8.0))
    .bg(rgb(config.background))
    .opacity(chrome.opacity)
    .shadow(shell_shadow())
    .cursor_pointer()
    .flex()
    .items_center()
    .justify_center()
    .on_mouse_down(MouseButton::Left, {
      let dock = dock.clone();
      move |event: &MouseDownEvent, window, cx| {
        let position = event.position;
        dock.update(cx, |dock, cx| dock.start_drag(position, window, cx));
      }
    })
    .child(icons::icon(icons::chevron(collapsed), 0xffffff, 16.0))
    .into_any_element()
}

fn render_toolbar(dock: Entity<ScanDock>, bar: Bar) -> gpui::Stateful<Div> {
  div()
    .id("gpui-scan-toolbar")
    .h(px(TOOLBAR_HEIGHT))
    .flex_shrink_0()
    .flex()
    .items_center()
    .on_mouse_down(MouseButton::Left, {
      let dock = dock.clone();
      move |event: &MouseDownEvent, window, cx| {
        let position = event.position;
        dock.update(cx, |dock, cx| dock.start_drag(position, window, cx));
      }
    })
    .child(render_inspect(dock.clone(), &bar))
    .child(render_toggle(dock.clone(), &bar))
    .child(render_fps(dock, &bar))
}

fn render_inspect(dock: Entity<ScanDock>, bar: &Bar) -> gpui::Stateful<Div> {
  let markup = if bar.inspect_focused {
    icons::focus()
  } else {
    icons::inspect()
  };
  div()
    .id("gpui-scan-inspect")
    .debug_selector(|| "gpui-scan-inspect".into())
    .h(px(TOOLBAR_HEIGHT))
    .pl(px(12.0))
    .pr(px(10.0))
    .flex()
    .items_center()
    .justify_center()
    .cursor_pointer()
    .hover(|style| style.bg(rgb(0xffffff).opacity(0.1)))
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_click({
      let dock = dock.clone();
      move |_: &ClickEvent, window, cx| {
        dock.update(cx, |dock, cx| dock.pick(window, cx));
      }
    })
    .child(icons::icon(markup, bar.inspect_color, 15.0))
}

fn render_toggle(dock: Entity<ScanDock>, bar: &Bar) -> gpui::Stateful<Div> {
  let track = if bar.outline {
    geom::TOGGLE_ON
  } else {
    geom::TOGGLE_OFF
  };
  // The knob sits 4px from the track's left when off and travels its own width when on.
  let knob_x = 4.0 + bar.knob * 16.0;
  div()
    .id("gpui-scan-outline")
    .w(px(40.0))
    .h(px(24.0))
    .relative()
    .cursor_pointer()
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_click({
      let dock = dock.clone();
      move |_: &ClickEvent, _, cx| {
        dock.update(cx, |dock, cx| dock.toggle_outline(cx));
      }
    })
    .child(
      div()
        .absolute()
        .top(px(4.0))
        .left(px(4.0))
        .w(px(32.0))
        .h(px(16.0))
        .rounded_full()
        .bg(rgb(track)),
    )
    .child(
      div()
        .absolute()
        .top(px(4.0))
        .left(px(knob_x))
        .size(px(16.0))
        .rounded_full()
        .bg(rgb(0xffffff))
        .border(px(2.0))
        .border_color(rgb(track)),
    )
}

fn render_fps(dock: Entity<ScanDock>, bar: &Bar) -> gpui::Stateful<Div> {
  div()
    .id("gpui-scan-fps")
    .debug_selector(|| "gpui-scan-fps".into())
    .ml(px(4.0))
    .mr(px(4.0))
    .w(px(72.0))
    .h(px(24.0))
    .on_mouse_down(MouseButton::Left, {
      let dock = dock.clone();
      move |event: &MouseDownEvent, window, cx| {
        let position = event.position;
        dock.update(cx, |dock, cx| dock.start_drag(position, window, cx));
      }
    })
    .child(
      div()
        .size_full()
        .rounded(px(6.0))
        .bg(rgb(geom::FPS_PILL))
        .border_1()
        .border_color(rgb(0xffffff).opacity(0.08))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
          div()
            .text_size(px(14.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(bar.fps_color))
            .child(bar.fps_text.clone()),
        )
        .child(
          div()
            .text_size(px(11.0))
            .text_color(rgb(0xffffff).opacity(0.3))
            .child("FPS"),
        ),
    )
}

fn render_measure(dock: Entity<ScanDock>) -> impl IntoElement {
  let dock = dock.downgrade();
  canvas(
    move |bounds, _, cx| {
      let Some(dock) = dock.upgrade() else {
        return;
      };
      let width = bounds.size.width.as_f32();
      dock.update(cx, |_, cx| {
        if state::global(cx).borrow_mut().widget.set_bar_width(width) {
          cx.notify();
        }
      });
    },
    |_, _, _, _| {},
  )
  .absolute()
  .top_0()
  .left_0()
  .size_full()
}

fn resize_handles(dock: Entity<ScanDock>) -> [Div; 4] {
  [
    resize_handle(dock.clone(), Edge::Left),
    resize_handle(dock.clone(), Edge::Right),
    resize_handle(dock.clone(), Edge::Top),
    resize_handle(dock, Edge::Bottom),
  ]
}

fn resize_handle(dock: Entity<ScanDock>, edge: Edge) -> Div {
  let horizontal = matches!(edge, Edge::Left | Edge::Right);
  div()
    .absolute()
    .when(edge == Edge::Left, |this| {
      this.left(px(0.0)).top(px(0.0)).bottom_0().w(px(8.0))
    })
    .when(edge == Edge::Right, |this| {
      this.right(px(0.0)).top(px(0.0)).bottom_0().w(px(8.0))
    })
    .when(edge == Edge::Top, |this| {
      this.top(px(0.0)).left(px(0.0)).right_0().h(px(8.0))
    })
    .when(edge == Edge::Bottom, |this| {
      this.bottom(px(0.0)).left(px(0.0)).right_0().h(px(8.0))
    })
    .when(horizontal, |this| this.cursor_ew_resize())
    .when(!horizontal, |this| this.cursor_ns_resize())
    .hover(|style| style.bg(rgb(0xffffff).opacity(0.1)))
    .on_mouse_down(
      MouseButton::Left,
      move |event: &MouseDownEvent, window, cx| {
        cx.stop_propagation();
        let position = event.position;
        dock.update(cx, |dock, cx| dock.start_resize(edge, position, window, cx));
      },
    )
}

/// Returns the window's dock, creating its frame loop the first time it is asked for.
pub(crate) fn dock_for_window(window: &mut Window, cx: &mut App) -> Entity<ScanDock> {
  crate::tracking::enable();
  let window_id = window.window_handle().window_id();

  let existing = state::global(cx).borrow().docks().get(&window_id).cloned();
  if let Some(dock) = existing {
    return dock;
  }

  let config = state::global(cx).borrow().config().clone();
  let dock = cx.new(|cx| ScanDock::new(&config, cx));
  state::global(cx)
    .borrow_mut()
    .register_dock(window_id, dock.clone());
  start_frame_loop(window, cx, dock.downgrade());
  dock
}

/// Toggles the outline switch of the dock in the active window.
pub(crate) fn toggle_outlines(cx: &mut App) {
  let Some(window) = cx.active_window() else {
    return;
  };
  let dock = state::global(cx)
    .borrow()
    .docks()
    .get(&window.window_id())
    .cloned();
  if let Some(dock) = dock {
    dock.update(cx, |dock, cx| dock.toggle_outline(cx));
  }
}

fn start_frame_loop(window: &mut Window, cx: &mut App, dock: WeakEntity<ScanDock>) {
  let window_id = window.window_handle().window_id();
  if !state::global(cx).borrow_mut().start_ticking(window_id) {
    return;
  }
  schedule_frame(window, dock);
}

fn schedule_frame(window: &mut Window, dock: WeakEntity<ScanDock>) {
  window.on_next_frame(move |window, cx| {
    let Some(dock) = dock.upgrade() else {
      return;
    };
    let now = Instant::now();
    dock.update(cx, |dock, cx| dock.advance(now, window, cx));
    schedule_frame(window, dock.downgrade());
  });
}

/// Handles Escape in the active window.
pub(crate) fn cancel_pick_in_active_window(cx: &mut App) {
  let Some(handle) = cx.active_window() else {
    return;
  };
  let dock = state::global(cx)
    .borrow()
    .docks()
    .get(&handle.window_id())
    .cloned();
  let Some(dock) = dock else {
    return;
  };

  // Actions can run while the inspector is detached for a draw, so settle the window state
  // before asking it to leave picking mode.
  cx.defer(move |cx| {
    let _ = handle.update(cx, |_, window, cx| {
      dock.update(cx, |dock, cx| dock.on_escape(window, cx));
    });
  });
}

/// Header buttons call these. They live here so the inspector renderer does not borrow the dock.
pub(crate) fn copy_from_panel(dock: &Entity<ScanDock>, text: String, cx: &mut App) {
  dock.update(cx, |dock, cx| dock.copy_source(text, cx));
}

pub(crate) fn dismiss_from_panel(dock: &Entity<ScanDock>, window: &mut Window, cx: &mut App) {
  dock.update(cx, |dock, cx| dock.dismiss(window, cx));
}
