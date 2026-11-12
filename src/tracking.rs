//! Render tracking for the [`scan`](crate::scan) attribute.
//!
//! The attribute calls [`begin`] before a view renders and [`finish`] with the rendered element.
//! While scanning is off that is a single relaxed atomic load. While it is on, [`begin`] reads and
//! clears the view's notification flag in one map access. While the view has an outline on screen,
//! [`begin`] keeps returning a signal so the outline follows layout changes such as an ancestor
//! scrolling. Idle frames return the view's own element untouched.

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
  AnyElement, App, Bounds, Context, Element, ElementId, EntityId, GlobalElementId,
  InspectorElementId, IntoElement, LayoutId, Pixels, Window,
};
use web_time::Instant;

use crate::state::{self, HIGHLIGHT_LIFETIME, TrackKey};

/// Whether anything ever asked for scanning. Set by [`init`](crate::init) and by the overlay.
static ENABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn enable() {
  ENABLED.store(true, Ordering::Relaxed);
}

/// The state of one view for the current frame.
///
/// `fresh` marks a consumed notification. When it is false the wrapper exists only because the
/// view still has a visible outline, so its bounds are refreshed without extending the fade.
#[doc(hidden)]
pub struct Signal {
  entity: EntityId,
  fresh: bool,
}

/// Resolves the tracking signal of the view being rendered.
///
/// Returns `None` while scanning is disabled, and on frames when the view neither changed nor
/// still has an outline on screen.
#[doc(hidden)]
#[inline]
pub fn begin<T: 'static>(cx: &mut Context<'_, T>) -> Option<Signal> {
  if !ENABLED.load(Ordering::Relaxed) {
    return None;
  }

  let entity = cx.entity_id();
  let (known, fresh, active) = {
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

  if fresh {
    let until = Instant::now() + HIGHLIGHT_LIFETIME;
    let shared = state::global(cx);
    let mut state = shared.borrow_mut();
    state.activate(entity, until);
  }

  (fresh || active).then_some(Signal { entity, fresh })
}

/// Wraps the rendered element in a bounds recorder while the view is fresh or still outlined.
#[doc(hidden)]
#[inline]
pub fn finish<E: IntoElement>(
  signal: Option<Signal>,
  element: E,
  label: &'static str,
  file: &'static str,
  line: u32,
  column: u32,
) -> AnyElement {
  match signal {
    Some(signal) => TrackedRender {
      element: element.into_any_element(),
      entity: signal.entity,
      fresh: signal.fresh,
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
  fresh: bool,
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
    let key = TrackKey::at(
      self.file,
      self.line,
      self.column,
      self.label.into(),
      self.entity.as_u64(),
    );
    if self.fresh {
      state::note_render(cx, key, bounds);
    } else {
      state::note_bounds(cx, key, bounds);
    }
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
