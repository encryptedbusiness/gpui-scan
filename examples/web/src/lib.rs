use gpui::{App, AppContext as _, Bounds, WindowBounds, WindowOptions, px, size};
use wasm_bindgen::prelude::*;

mod site;

use site::Site;

#[wasm_bindgen(start)]
pub fn start() {
  #[cfg(target_family = "wasm")]
  gpui_platform::web_init();

  #[cfg(target_family = "wasm")]
  let application = gpui_platform::single_threaded_web();
  #[cfg(not(target_family = "wasm"))]
  let application = gpui_platform::application();

  application.run(|cx: &mut App| {
    gpui_kit::init(cx);
    gpui_scan::init_with(gpui_scan::Config::default().outlines(true), cx);

    #[cfg(target_family = "wasm")]
    add_fonts(cx);

    let bounds = Bounds::centered(None, size(px(1000.0), px(760.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| cx.new(Site::new),
    )
    .unwrap();

    cx.activate(true);
  });
}

#[cfg(target_family = "wasm")]
fn add_fonts(cx: &mut App) {
  use std::borrow::Cow;

  // The web platform resolves GPUI's system font to "IBM Plex Sans" and ships no fonts of its
  // own, so that family has to be registered or the first text measurement panics.
  let system = Cow::Borrowed(include_bytes!("../assets/IBMPlexSans-Regular.ttf").as_slice());
  cx.text_system()
    .add_fonts(vec![system])
    .expect("the bundled font loads");
}
