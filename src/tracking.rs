//! Render tracking for the [`scan`](crate::scan) attribute.
//!
//! The attribute calls [`begin`] before a view renders and [`finish`] with the rendered element.
//! While scanning is off that is a single relaxed atomic load. While it is on, [`begin`] reads and
//! clears the view's notification flag in one map access, and [`finish`] only builds a tracking
//! element on the frame the view was notified. Idle frames return the view's own element
//! untouched, so a tracked view costs the same as an untracked one apart from that lookup.

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
  AnyElement, App, Bounds, Context, Element, ElementId, EntityId, GlobalElementId,
  InspectorElementId, IntoElement, LayoutId, Pixels, Window,
};

use crate::state::{self, TrackKey};

/// Whether anything ever asked for scanning. Set by [`init`](crate::init) and by the overlay.
static ENABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn enable() {
  ENABLED.store(true, Ordering::Relaxed);
}

/// Marks the view as changed and returns its id when it should be wrapped this frame.
///
/// Returns `None` while scanning is disabled, and on frames the view did not change.
#[doc(hidden)]
#[inline]
pub fn begin<T: 'static>(cx: &mut Context<'_, T>) -> Option<EntityId> {
  if !ENABLED.load(Ordering::Relaxed) {
    return None;
  }

  let entity = cx.entity_id();
  let (known, notified) = {
    let shared = state::global(cx);
    let mut state = shared.borrow_mut();
    state.take_notified(entity)
  };

  if !known {
    let handle = cx.entity();
    let subscription = cx.observe(&handle, move |_, _, cx| state::mark_notified(cx, entity));
    let shared = state::global(cx);
    let mut state = shared.borrow_mut();
    state.insert_notify_subscription(entity, subscription);
  }

  notified.then_some(entity)
}

/// Wraps the rendered element in a bounds recorder when the view was notified this frame.
#[doc(hidden)]
#[inline]
pub fn finish<E: IntoElement>(
  signal: Option<EntityId>,
  element: E,
  label: &'static str,
  file: &'static str,
  line: u32,
  column: u32,
) -> AnyElement {
  match signal {
    Some(entity) => TrackedRender {
      element: element.into_any_element(),
      entity,
      label,
      file,
      line,
      column,
    }
    .into_any_element(),
    None => element.into_any_element(),
  }
}

struct TrackedRender {
  element: AnyElement,
  entity: EntityId,
  label: &'static str,
  file: &'static str,
  line: u32,
  column: u32,
}

impl IntoElement for TrackedRender {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl Element for TrackedRender {
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
    (self.element.request_layout(window, cx), ())
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
    state::note_render(
      cx,
      TrackKey::at(
        self.file,
        self.line,
        self.column,
        self.label.into(),
        self.entity.as_u64(),
      ),
      bounds,
    );
    self.element.prepaint(window, cx);
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
    self.element.paint(window, cx);
  }
}
