use gpui::{AnyElement, App, IntoElement, RenderOnce, Window, div, prelude::*};

/// The scanner overlay.
///
/// Add it once, as the last child of the element your root view returns:
///
/// ```
/// # use gpui::{div, prelude::*, Context, Window};
/// # struct App;
/// # impl gpui::Render for App {
/// fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
///   div()
///     .relative()
///     .size_full()
///     .child("your application")
///     .child(gpui_scan::scan_layer())
/// }
/// # }
/// ```
///
/// The layer is absolutely positioned over the whole window, so it draws above the sibling
/// elements and keeps its hitboxes on top without taking part in the surrounding layout.
#[derive(IntoElement)]
pub struct ScanLayer;

impl ScanLayer {
  /// Creates the overlay; the scanner installs one per window on first use.
  pub fn new() -> Self {
    Self
  }
}

impl Default for ScanLayer {
  fn default() -> Self {
    Self::new()
  }
}

impl RenderOnce for ScanLayer {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    // Every element takes part in the inspector's picking, so the overlay has to step out of the
    // way while the user is choosing an element to inspect.
    if window.is_inspector_picking(cx) {
      return gpui::Empty.into_any_element();
    }

    let dock = crate::dock::dock_for_window(window, cx);

    div()
      .absolute()
      .top_0()
      .left_0()
      .size_full()
      .child(dock)
      .into_any_element()
  }
}

/// Wraps the application so the scanner can overlay an inspector panel without reflowing it.
///
/// ```
/// # use gpui::{div, prelude::*, Context, Window};
/// # struct App;
/// # impl gpui::Render for App {
/// fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
///   gpui_scan::scan_root(div().size_full().child("your application"))
/// }
/// # }
/// ```
///
/// GPUI reserves `30rem` of the window for the inspector while it is open. This wrapper pins the
/// application to the window's full width, so the inspector panel overlaps the application
/// instead of shrinking it. Use [`scan_layer`](crate::scan_layer) instead if you want the panel
/// to dock and the application to make room for it.
#[derive(IntoElement)]
pub struct ScanRoot {
  inner: AnyElement,
}

/// Wraps the application root; see [`ScanRoot`].
pub fn scan_root(inner: impl IntoElement) -> ScanRoot {
  ScanRoot {
    inner: inner.into_any_element(),
  }
}

impl RenderOnce for ScanRoot {
  fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
    let width = window.viewport_size().width;

    div()
      .w(width)
      .h_full()
      .child(self.inner)
      .child(ScanLayer::new())
  }
}
