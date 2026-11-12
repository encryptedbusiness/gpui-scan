use gpui::{App, AppContext as _, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_scan_basic::demo;

fn main() {
  gpui_platform::application().run(|cx: &mut App| {
    gpui_scan::init(cx);

    let bounds = Bounds::centered(None, size(px(880.0), px(560.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| cx.new(demo),
    )
    .unwrap();

    cx.activate(true);
  });
}
