use gpui::{
  App, AppContext as _, Bounds, Context, Entity, IntoElement, Render, Window, WindowBounds,
  WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_scan_basic::{Demo, demo};

/// The native demo: the shared [`Demo`] panel with the standalone page chrome.
struct DemoRoot {
  demo: Entity<Demo>,
}

impl Render for DemoRoot {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    gpui_scan::scan_root(
      div()
        .relative()
        .size_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .bg(rgb(0x0a0a0a))
        .text_color(rgb(0xf4f4f5))
        .child(
          div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
              div()
                .text_xl()
                .font_weight(gpui::FontWeight::BOLD)
                .child("gpui-scan"),
            )
            .child(
              div()
                .text_sm()
                .text_color(rgb(0x8b8b8b))
                .child(
                  "Outlines fade the way React Scan's do. Drag the toolbar to a corner, or off an edge to tuck it away.",
                ),
            ),
        )
        .child(self.demo.clone())
        .child(div().flex_1())
        .child(
          div()
            .text_sm()
            .text_color(rgb(0x8b8b8b))
            .child("Press the crosshair to pick any element and open the devtools inspector."),
        ),
    )
  }
}

fn main() {
  gpui_platform::application().run(|cx: &mut App| {
    gpui_scan::init(cx);

    let bounds = Bounds::centered(None, size(px(880.0), px(560.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| {
        let demo = cx.new(demo);
        cx.new(|_| DemoRoot { demo })
      },
    )
    .unwrap();

    cx.activate(true);
  });
}
