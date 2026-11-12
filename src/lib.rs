//! React Scan for [GPUI](https://gpui.rs).
//!
//! `gpui-scan` adds a React Scan style toolbar to a GPUI application: an element picker, a switch
//! that outlines everything that updated just now, and a live FPS readout. Selecting an element
//! opens the [gpui-devtools](https://github.com/joris-gallot/gpui-devtools) inspector for that
//! element.
//!
//! ```no_run
//! # use gpui::App;
//! fn main() {
//!   gpui_platform::application().run(|cx: &mut App| {
//!     gpui_scan::init(cx);
//!     // Open your window as usual.
//!   });
//! }
//! ```
//!
//! Then, in the root view's `render`, add [`scan_layer`] and track the views whose updates you
//! want attributed:
//!
//! ```
//! # use gpui::{div, prelude::*, Context, Entity, IntoElement, Render, Window};
//! # struct Sidebar;
//! # impl Render for Sidebar {
//! # fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
//! # div()
//! # }
//! # }
//! # struct App { sidebar: Entity<Sidebar> }
//! # impl Render for App {
//! fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!   div()
//!     .relative()
//!     .size_full()
//!     .child(gpui_scan::track_view(&self.sidebar, cx))
//!     .child(gpui_scan::scan_layer())
//! }
//! # }
//! ```
//!
//! [`init`] installs gpui-devtools' inspector renderer, so call it after any library that
//! registers its own inspector renderer, and do not also call `gpui_devtools::init`.
//!
//! GPUI only builds its inspector with the `inspector` feature, so gate the scanner behind a
//! feature of your own to keep it out of release builds.

#![warn(missing_docs)]

mod config;
mod dock;
mod geom;
mod icons;
mod layer;
mod panel;
mod state;
mod tracked;

pub use config::Config;
pub use layer::{ScanLayer, ScanRoot, scan_root};
pub use tracked::{ScanTrackExt, Tracked, track_view, track_view_as, track_view_keyed};

use gpui::{App, KeyBinding, actions};

actions!(
  gpui_scan,
  [
    /// Turns the re-render outlines on or off in the active window.
    ToggleOutlines,
    /// Leaves element picking. While an element is focused, Escape returns to picking the way
    /// React Scan does.
    CancelPick
  ]
);

/// Installs the scanner with the default configuration.
pub fn init(cx: &mut App) {
  init_with(Config::default(), cx);
}

/// Installs the scanner with the given configuration.
///
/// This binds [`ToggleOutlines`], handles it, and installs the gpui-devtools inspector renderer.
pub fn init_with(config: Config, cx: &mut App) {
  if let Some(key_binding) = config.key_binding {
    cx.bind_keys([KeyBinding::new(key_binding, ToggleOutlines, None)]);
  }
  if let Some(key_binding) = config.cancel_key_binding {
    cx.bind_keys([KeyBinding::new(key_binding, CancelPick, None)]);
  }

  cx.on_action(|_: &ToggleOutlines, cx| dock::toggle_outlines(cx));
  cx.on_action(|_: &CancelPick, cx| dock::cancel_pick_in_active_window(cx));

  state::install(config, cx);
  gpui_devtools::init(cx);

  // gpui-devtools registers the per-element panels; the chrome around them is ours, so the
  // inspector shows a floating card instead of a docked sidebar.
  cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
    panel::render(inspector, window, cx)
  }));
}

/// The floating toolbar and re-render outlines.
///
/// Use this without [`init`] to keep the default key bindings and inspector untouched.
pub fn scan_layer() -> ScanLayer {
  ScanLayer::new()
}
