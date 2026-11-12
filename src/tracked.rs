use std::{
  cell::Cell,
  hash::{DefaultHasher, Hash, Hasher},
  rc::Rc,
};

use gpui::{
  AnyElement, App, Bounds, Element, ElementId, Entity, GlobalElementId, InspectorElementId,
  IntoElement, LayoutId, Pixels, Render, SharedString, Window,
};

use crate::state::TrackKey;

/// A view whose updates are reported to the toolbar.
///
/// Create it with [`track_view`] or [`ScanTrackExt::scan`] and embed the result like any other
/// element. It reports only when the view was notified, which is the GPUI equivalent of a React
/// component re-rendering.
pub struct Tracked {
  element: Option<AnyElement>,
  key: TrackKey,
  notified: Rc<Cell<bool>>,
}

impl Tracked {
  pub(crate) fn new<V: Render>(
    location: &'static std::panic::Location<'static>,
    label: SharedString,
    entity: &Entity<V>,
    notified: Rc<Cell<bool>>,
  ) -> Self {
    Self {
      element: Some(entity.clone().into_any_element()),
      key: TrackKey::new(location, label, 0),
      notified,
    }
  }
}

impl IntoElement for Tracked {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl Element for Tracked {
  type RequestLayoutState = ();
  type PrepaintState = ();

  fn id(&self) -> Option<ElementId> {
    None
  }

  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }

  fn request_layout(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, ()) {
    let layout_id = self.element.as_mut().unwrap().request_layout(window, cx);
    (layout_id, ())
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    let visible = bounds.size.width > gpui::px(0.0) || bounds.size.height > gpui::px(0.0);
    if visible && self.notified.take() {
      crate::state::note_render(cx, self.key.clone(), bounds);
    }

    self.element.as_mut().unwrap().prepaint(window, cx);
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    _bounds: Bounds<Pixels>,
    _request_layout: &mut (),
    _prepaint: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    self.element.as_mut().unwrap().paint(window, cx);
  }
}

/// Tracks a view, labeling it with the view's type name.
///
/// ```
/// # use gpui::{div, prelude::*, Context, Entity, IntoElement, Render, Window};
/// # struct Sidebar;
/// # impl Render for Sidebar {
/// # fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
/// # div()
/// # }
/// # }
/// # struct App { sidebar: Entity<Sidebar> }
/// # impl Render for App {
/// fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
///   div()
///     .relative()
///     .size_full()
///     .child(gpui_scan::track_view(&self.sidebar, cx))
///     .child(gpui_scan::scan_layer())
/// }
/// # }
/// ```
#[track_caller]
pub fn track_view<V: Render>(entity: &Entity<V>, cx: &mut App) -> Tracked {
  track_view_as(type_label::<V>(), entity, cx)
}

/// Tracks a view under the given name.
#[track_caller]
pub fn track_view_as<V: Render>(
  label: impl Into<SharedString>,
  entity: &Entity<V>,
  cx: &mut App,
) -> Tracked {
  let location = std::panic::Location::caller();
  let notified = crate::state::observe_view(entity, cx);
  Tracked::new(location, label.into(), entity, notified)
}

/// Tracks a view under the given name and instance key, for lists built from one call site.
#[track_caller]
pub fn track_view_keyed<V: Render>(
  label: impl Into<SharedString>,
  instance: impl Hash,
  entity: &Entity<V>,
  cx: &mut App,
) -> Tracked {
  let location = std::panic::Location::caller();
  let notified = crate::state::observe_view(entity, cx);
  let mut tracked = Tracked::new(location, label.into(), entity, notified);
  tracked.key.instance = hash(&instance);
  tracked
}

/// Adds the tracking constructors to [`Entity`] values.
pub trait ScanTrackExt: Sized {
  /// Tracks this view, labeling it with the view's type name.
  #[track_caller]
  fn scan(&self, cx: &mut App) -> Tracked;

  /// Tracks this view under the given name.
  #[track_caller]
  fn scan_as(&self, label: impl Into<SharedString>, cx: &mut App) -> Tracked;

  /// Tracks this view under the given name and instance key.
  #[track_caller]
  fn scan_keyed(
    &self,
    label: impl Into<SharedString>,
    instance: impl Hash,
    cx: &mut App,
  ) -> Tracked;
}

impl<V: Render> ScanTrackExt for Entity<V> {
  #[track_caller]
  fn scan(&self, cx: &mut App) -> Tracked {
    track_view(self, cx)
  }

  #[track_caller]
  fn scan_as(&self, label: impl Into<SharedString>, cx: &mut App) -> Tracked {
    track_view_as(label, self, cx)
  }

  #[track_caller]
  fn scan_keyed(
    &self,
    label: impl Into<SharedString>,
    instance: impl Hash,
    cx: &mut App,
  ) -> Tracked {
    track_view_keyed(label, instance, self, cx)
  }
}

fn hash(instance: &impl Hash) -> u64 {
  let mut hasher = DefaultHasher::new();
  instance.hash(&mut hasher);
  hasher.finish()
}

fn type_label<V>() -> SharedString {
  let name = std::any::type_name::<V>();
  let name = name.split('<').next().unwrap_or(name);
  let name = name.rsplit("::").next().unwrap_or(name);
  name.into()
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, Instant};

  use gpui::{
    Bounds, Context, IntoElement, Modifiers, Render, TestAppContext, Window, div, point,
    prelude::*, px, size,
  };

  use crate::{
    ScanTrackExt as _,
    state::{self, FpsMeter, HIGHLIGHT_LIFETIME},
  };

  struct Counter {
    count: u32,
  }

  impl Render for Counter {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div().size_full().child(self.count.to_string())
    }
  }

  struct Fixture {
    counter: gpui::Entity<Counter>,
  }

  impl Render for Fixture {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      div().size_full().child(self.counter.scan(cx))
    }
  }

  fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| {
      window.draw(cx).clear(cx);
    });
  }

  fn commit(cx: &mut gpui::VisualTestContext) -> usize {
    cx.update(|_, cx| {
      let shared = state::global(cx);
      let mut state = shared.borrow_mut();
      state.commit_frame(Instant::now())
    })
  }

  fn with_dock<R>(
    cx: &mut gpui::VisualTestContext,
    f: impl FnOnce(&crate::dock::ScanDock) -> R,
  ) -> R {
    cx.update(|window, cx| {
      let id = window.window_handle().window_id();
      let dock = state::global(cx)
        .borrow()
        .docks()
        .get(&id)
        .cloned()
        .expect("the dock exists");
      f(dock.read(cx))
    })
  }

  fn pending(cx: &mut gpui::VisualTestContext) -> usize {
    cx.update(|_, cx| {
      let shared = state::global(cx);
      let state = shared.borrow();
      state.pending_count()
    })
  }

  fn labels(cx: &mut gpui::VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| {
      let shared = state::global(cx);
      let state = shared.borrow();
      let mut labels = state
        .highlights()
        .map(|(key, _)| key.label.to_string())
        .collect::<Vec<_>>();
      labels.sort();
      labels
    })
  }

  fn renders(cx: &mut gpui::VisualTestContext, label: &str) -> u32 {
    cx.update(|_, cx| {
      let shared = state::global(cx);
      let state = shared.borrow();
      state
        .highlights()
        .find(|(key, _)| key.label.as_ref() == label)
        .map(|(_, highlight)| highlight.renders)
        .unwrap_or(0)
    })
  }

  fn bounds() -> Bounds<gpui::Pixels> {
    Bounds::new(point(px(0.0), px(0.0)), size(px(10.0), px(10.0)))
  }

  #[gpui::test]
  fn a_tracked_view_is_reported_when_it_mounts(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Fixture { counter });

    draw(cx);
    assert_eq!(pending(cx), 1);
    commit(cx);
    assert_eq!(labels(cx), vec!["Counter"]);
  }

  #[gpui::test]
  fn an_idle_frame_reports_nothing(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Fixture { counter });

    draw(cx);
    commit(cx);

    draw(cx);
    assert_eq!(pending(cx), 0, "a frame without a notification is quiet");
  }

  #[gpui::test]
  fn notifying_the_view_reports_it_again(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Fixture {
      counter: counter.clone(),
    });

    draw(cx);
    commit(cx);

    draw(cx);
    assert_eq!(pending(cx), 0);

    cx.update(|_, cx| {
      counter.update(cx, |counter, cx| {
        counter.count += 1;
        cx.notify();
      });
    });
    cx.run_until_parked();

    draw(cx);
    assert_eq!(pending(cx), 1, "the notification is reported once");
  }

  struct Attributed {
    count: u32,
  }

  #[crate::scan]
  impl Render for Attributed {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div().size_full().child(self.count.to_string())
    }
  }

  struct AttributedFixture {
    view: gpui::Entity<Attributed>,
  }

  impl Render for AttributedFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div().size_full().child(self.view.clone())
    }
  }

  #[gpui::test]
  fn the_scan_attribute_tracks_renders(cx: &mut TestAppContext) {
    crate::tracking::enable();
    let view = cx.new(|_| Attributed { count: 0 });
    let (_root, cx) = cx.add_window_view(|_, _| AttributedFixture { view: view.clone() });

    draw(cx);
    commit(cx);
    assert_eq!(renders(cx, "Attributed"), 1);

    draw(cx);
    assert_eq!(
      pending(cx),
      1,
      "an active outline keeps refreshing its bounds"
    );
    commit(cx);

    cx.update(|_, cx| {
      view.update(cx, |view, cx| {
        view.count += 1;
        cx.notify();
      });
    });
    cx.run_until_parked();

    draw(cx);
    commit(cx);
    assert_eq!(
      renders(cx, "Attributed"),
      2,
      "the notification is reported once"
    );
  }

  #[test]
  fn scrolling_pins_outlines_to_their_target() {
    let mut state = state::ScanState::new(crate::Config::default());
    let key = state::TrackKey::new(std::panic::Location::caller(), "Counter".into(), 0);
    let start = Instant::now();
    state.note_render(key.clone(), bounds());
    state.commit_frame(start);

    let moved = Bounds::new(point(px(80.0), px(80.0)), size(px(10.0), px(10.0)));
    state.note_bounds(key, moved);
    state.commit_frame(start + Duration::from_millis(10));

    state.mark_scroll(start + Duration::from_millis(20));
    state.step_highlights(start + Duration::from_millis(30));

    let highlight = state.highlights().next().unwrap().1;
    assert_eq!(
      highlight.bounds, moved,
      "a scroll pins the outline to the target instead of easing"
    );
  }

  #[test]
  fn bounds_only_updates_do_not_refresh_the_outline() {
    let mut state = state::ScanState::new(crate::Config::default());
    let key = state::TrackKey::new(std::panic::Location::caller(), "Counter".into(), 0);
    let start = Instant::now();
    state.note_render(key.clone(), bounds());
    state.commit_frame(start);

    let before = state.highlights().next().unwrap().1;
    let updated_at = before.updated_at;
    let renders = before.renders;

    let moved = Bounds::new(point(px(0.0), px(40.0)), size(px(10.0), px(10.0)));
    state.note_bounds(key, moved);
    state.commit_frame(start + HIGHLIGHT_LIFETIME / 2);

    let after = state.highlights().next().unwrap().1;
    assert_eq!(after.target, moved, "the outline follows the new bounds");
    assert_eq!(after.updated_at, updated_at, "the fade is not refreshed");
    assert_eq!(after.renders, renders, "the update count is not raised");
  }

  #[test]
  fn an_outline_tracks_bounds_until_its_lifetime_expires() {
    let mut state = state::ScanState::new(crate::Config::default());
    let entity = gpui::EntityId::from(1);
    let now = Instant::now();

    let (known, fresh, active) = state.take_notified(entity);
    assert!(!known && fresh && active, "the first sighting is a mount");
    state.activate(entity, now + HIGHLIGHT_LIFETIME);

    let (known, fresh, active) = state.take_notified(entity);
    assert!(
      known && !fresh && active,
      "an outlined view keeps refreshing its bounds"
    );

    state.prune(now + HIGHLIGHT_LIFETIME + Duration::from_millis(1));
    let (_, fresh, active) = state.take_notified(entity);
    assert!(
      !fresh && !active,
      "the outline stops tracking after its lifetime"
    );
  }

  struct Shell {
    counter: gpui::Entity<Counter>,
  }

  impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      div()
        .relative()
        .size_full()
        .child(self.counter.scan(cx))
        .child(crate::scan_layer())
    }
  }

  #[gpui::test]
  fn the_layer_renders_the_dock_and_ticks(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });

    draw(cx);
    let dock = cx
      .debug_bounds("gpui-scan-dock")
      .expect("the dock is rendered");
    assert!(dock.size.width > px(0.0) && dock.size.height > px(0.0));

    // The frame loop schedules itself again on every frame.
    for _ in 0..3 {
      cx.update(|window, cx| {
        window.simulate_next_frame(cx);
      });
    }

    assert!(cx.debug_bounds("gpui-scan-dock").is_some());
  }

  #[gpui::test]
  fn the_fps_pill_is_rendered(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let fps = cx
      .debug_bounds("gpui-scan-fps")
      .expect("fps pill is rendered");
    assert!(fps.size.width > px(0.0) && fps.size.height > px(0.0));
  }

  #[gpui::test]
  fn dragging_the_dock_moves_it(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();

    draw(cx);
    let before = cx.debug_bounds("gpui-scan-dock").unwrap();
    // The inspect button owns the left edge and does not drag. The FPS pill does.
    let grab = cx.debug_bounds("gpui-scan-fps").expect("fps pill").center();
    let moved = grab + point(px(40.0), px(-30.0));

    cx.simulate_mouse_down(grab, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    cx.simulate_mouse_move(moved, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    let after = cx.debug_bounds("gpui-scan-dock").unwrap();
    assert!(
      (after.origin.x - before.origin.x - px(40.0)).abs() < px(1.5),
      "expected the dock to follow the pointer: {before:?} -> {after:?}"
    );

    cx.simulate_mouse_up(moved, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    let released = cx.debug_bounds("gpui-scan-dock").unwrap();
    assert!((released.origin.x - after.origin.x).abs() < px(0.5));
  }

  fn window_center(cx: &mut gpui::VisualTestContext) -> gpui::Point<gpui::Pixels> {
    let viewport = cx.update(|window, _| window.viewport_size());
    gpui::point(viewport.width / 2.0, viewport.height / 2.0)
  }

  #[gpui::test]
  fn the_picker_starts_inspector_picking(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let inspect = cx.debug_bounds("gpui-scan-inspect").expect("the picker");
    cx.simulate_click(inspect.center(), Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    assert!(
      cx.update(|window, cx| window.is_inspector_picking(cx)),
      "picking is what lets GPUI highlight any element"
    );
    assert!(with_dock(cx, |dock| dock.is_armed()));
    assert!(
      cx.debug_bounds("gpui-scan-dock").is_none(),
      "the dock steps aside while picking"
    );
  }

  #[gpui::test]
  fn escape_puts_the_picker_back_to_normal(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let inspect = cx.debug_bounds("gpui-scan-inspect").expect("the picker");
    cx.simulate_click(inspect.center(), Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    assert!(cx.update(|window, cx| window.is_inspector_picking(cx)));

    cx.dispatch_action(crate::CancelPick);
    cx.run_until_parked();
    draw(cx);

    assert!(
      !cx.update(|window, cx| window.is_inspector_picking(cx)),
      "escaping leaves the picker"
    );
    assert!(!with_dock(cx, |dock| dock.is_armed()));
    assert!(
      cx.debug_bounds("gpui-scan-dock").is_some(),
      "the dock is back"
    );
  }

  #[gpui::test]
  fn clicking_an_element_selects_it_and_opens_the_panel(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let inspect = cx.debug_bounds("gpui-scan-inspect").expect("the picker");
    cx.simulate_click(inspect.center(), Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    let target = window_center(cx);
    cx.simulate_mouse_move(target, None, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_click(target, Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    assert!(
      !cx.update(|window, cx| window.is_inspector_picking(cx)),
      "selecting an element ends picking and leaves the panel open"
    );
    assert!(
      cx.debug_bounds("gpui-scan-dock").is_some(),
      "the dock is back"
    );
  }

  #[gpui::test]
  fn the_inspector_body_scrolls(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let inspect = cx.debug_bounds("gpui-scan-inspect").expect("the picker");
    cx.simulate_click(inspect.center(), Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    let target = window_center(cx);
    cx.simulate_mouse_move(target, None, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_click(target, Modifiers::none());
    cx.run_until_parked();
    draw(cx);

    // The shell grows out of the toolbar. Wait out the 250ms snap before measuring.
    std::thread::sleep(std::time::Duration::from_millis(400));
    cx.update(|window, cx| {
      window.simulate_next_frame(cx);
    });
    draw(cx);

    let viewport = cx
      .debug_bounds("gpui-scan-body")
      .expect("the inspector scrollport");
    let before = cx
      .debug_bounds("gpui-scan-panel-body")
      .expect("the inspector content");
    assert!(
      before.size.height > viewport.size.height + px(8.0),
      "the panel has to be taller than its window to scroll: content {:?}, window {:?}",
      before.size,
      viewport.size
    );

    cx.simulate_event(gpui::ScrollEvent {
      position: viewport.center(),
      delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
      modifiers: Modifiers::none(),
      touch_phase: gpui::TouchPhase::Moved,
      source: gpui::ScrollSource::Pointer,
    });
    draw(cx);

    let after = cx
      .debug_bounds("gpui-scan-panel-body")
      .expect("the inspector content");
    assert!(
      before.origin.y - after.origin.y > px(20.0),
      "scrolling should move the panel content up: {before:?} -> {after:?}"
    );
  }

  struct Wrapped {
    counter: gpui::Entity<Counter>,
  }

  impl Render for Wrapped {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      crate::scan_root(
        div()
          .id("app-content")
          .debug_selector(|| "app-content".into())
          .size_full()
          .child(self.counter.scan(cx)),
      )
    }
  }

  #[gpui::test]
  fn scan_root_keeps_the_application_width_when_the_panel_opens(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Wrapped { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let before = cx.debug_bounds("app-content").expect("the app").size.width;
    cx.update(|window, cx| window.toggle_inspector(cx));
    cx.run_until_parked();
    draw(cx);
    let after = cx.debug_bounds("app-content").expect("the app").size.width;

    assert_eq!(
      before, after,
      "the panel overlays instead of shrinking the app"
    );
  }

  #[gpui::test]
  fn releasing_a_drag_inside_scan_root_glides_to_a_corner(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Wrapped { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let before = cx.debug_bounds("gpui-scan-dock").expect("the dock");
    let viewport = cx.update(|window, _| window.viewport_size());
    let target = gpui::point(px(viewport.width.as_f32() * 0.7), px(150.0));

    let grab = before.origin + gpui::point(before.size.width - px(6.0), px(17.0));
    cx.simulate_mouse_down(grab, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_mouse_move(target, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_mouse_up(target, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    let released = cx.debug_bounds("gpui-scan-dock").expect("the dock");
    assert!(released.origin.x > px(0.0), "released away from the edge");
    assert!(is_snapping(cx), "the glide starts on release");

    std::thread::sleep(std::time::Duration::from_millis(400));
    cx.update(|window, cx| {
      window.simulate_next_frame(cx);
    });
    draw(cx);

    assert_settled_in_corner(cx, grab, target);
  }

  #[gpui::test]
  fn releasing_a_drag_glides_the_dock_to_the_nearest_edge(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter { count: 0 });
    let (_view, cx) = cx.add_window_view(|_, _| Shell { counter });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);

    let before = cx.debug_bounds("gpui-scan-dock").expect("the dock");
    let viewport = cx.update(|window, _| window.viewport_size());
    let far_left = gpui::point(px(viewport.width.as_f32() * 0.25), px(180.0));

    let grab = before.origin + gpui::point(before.size.width - px(8.0), px(17.0));
    cx.simulate_mouse_down(grab, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_mouse_move(far_left, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    draw(cx);
    cx.simulate_mouse_up(far_left, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    let released = cx.debug_bounds("gpui-scan-dock").expect("the dock");
    assert!(released.origin.x > px(0.0), "released away from the edge");
    assert!(is_snapping(cx));

    std::thread::sleep(std::time::Duration::from_millis(400));
    cx.update(|window, cx| {
      window.simulate_next_frame(cx);
    });
    draw(cx);

    assert_settled_in_corner(cx, grab, far_left);
  }

  fn is_snapping(cx: &mut gpui::VisualTestContext) -> bool {
    cx.update(|_, cx| state::global(cx).borrow().widget.is_snapping())
  }

  /// The corner React Scan would pick for this drag, using the window the widget actually settled in.
  fn assert_settled_in_corner(
    cx: &mut gpui::VisualTestContext,
    grab: gpui::Point<gpui::Pixels>,
    release: gpui::Point<gpui::Pixels>,
  ) {
    let viewport = cx.update(|window, _| window.viewport_size());
    let settled = cx.debug_bounds("gpui-scan-dock").expect("the dock");
    let corner = crate::geom::best_corner(
      release.x.as_f32(),
      release.y.as_f32(),
      grab.x.as_f32(),
      grab.y.as_f32(),
      viewport.width.as_f32(),
      viewport.height.as_f32(),
      40.0,
    );
    let (x, y) = crate::geom::corner_origin(
      corner,
      viewport.width.as_f32(),
      viewport.height.as_f32(),
      settled.size.width.as_f32(),
      settled.size.height.as_f32(),
    );
    assert!(
      (settled.origin.x.as_f32() - x).abs() < 1.5 && (settled.origin.y.as_f32() - y).abs() < 1.5,
      "expected the {corner:?} corner at ({x}, {y}), settled at {:?}",
      settled.origin
    );
  }

  #[test]
  fn fps_is_measured_over_the_last_second() {
    let mut meter = FpsMeter::default();
    let start = Instant::now();
    for frame in 0..3 {
      meter.tick(start + Duration::from_micros(frame * 8_333));
    }
    let fps = meter.fps().unwrap();
    assert!((fps - 120.0).abs() < 5.0, "unexpected fps: {fps}");

    meter.tick(start + Duration::from_secs(2));
    assert_eq!(
      meter.fps(),
      None,
      "everything outside the window is dropped"
    );
  }

  #[test]
  fn a_fresh_clock_does_not_underflow() {
    // `Instant - Duration` panics on the web clock during the first second of a page.
    let start = Instant::now();
    let mut meter = FpsMeter::default();
    meter.tick(start);
    meter.tick(start + Duration::from_millis(5));
    assert!(meter.fps().is_some());

    let mut state = state::ScanState::new(crate::Config::default());
    state.prune(start);
  }

  #[test]
  fn a_reading_needs_at_least_two_frames() {
    let mut meter = FpsMeter::default();
    assert_eq!(meter.fps(), None);
    meter.tick(Instant::now());
    assert_eq!(meter.fps(), None);
  }

  #[test]
  fn expired_highlights_are_pruned() {
    let mut state = state::ScanState::new(crate::Config::default());
    let key = state::TrackKey::new(std::panic::Location::caller(), "Counter".into(), 0);

    let start = Instant::now();
    state.note_render(key.clone(), bounds());
    assert_eq!(state.commit_frame(start), 1);

    state.note_render(key, bounds());
    assert_eq!(state.commit_frame(start + HIGHLIGHT_LIFETIME / 2), 1);

    assert_eq!(
      state.commit_frame(
        start + HIGHLIGHT_LIFETIME / 2 + HIGHLIGHT_LIFETIME + Duration::from_millis(1)
      ),
      0
    );
  }

  #[test]
  fn repeated_updates_raise_the_render_count() {
    let mut state = state::ScanState::new(crate::Config::default());
    let key = state::TrackKey::new(std::panic::Location::caller(), "Counter".into(), 0);
    let now = Instant::now();

    for _ in 0..3 {
      state.note_render(key.clone(), bounds());
      state.commit_frame(now);
    }

    assert_eq!(state.highlights().next().unwrap().1.renders, 3);
  }
}
