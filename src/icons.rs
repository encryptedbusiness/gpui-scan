//! Icons copied from React Scan's sprite (`icon-inspect`, `icon-focus`, `icon-close`,
//! `icon-copy`, `icon-check`, and `icon-chevron-right`).

use std::sync::Arc;

use gpui::{Image, ImageFormat, img, prelude::*, px};

use crate::geom::{Axis, Collapsed, Corner};

const INSPECT: &str = include_str!("../assets/icons/inspect.svg");

const FOCUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.034 12.681a.498.498 0 0 1 .647-.647l9 3.5a.5.5 0 0 1-.033.943l-3.444 1.068a1 1 0 0 0-.66.66l-1.067 3.443a.5.5 0 0 1-.943.033z"/><path d="M21 11V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6"/></svg>"#;

const CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;

const COPY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>"#;

const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;

const CHEVRON_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>"#;
const CHEVRON_LEFT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>"#;
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
const CHEVRON_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 15 6-6 6 6"/></svg>"#;

pub(crate) fn inspect() -> &'static str {
  INSPECT
}

pub(crate) fn focus() -> &'static str {
  FOCUS
}

pub(crate) fn close() -> &'static str {
  CLOSE
}

pub(crate) fn copy() -> &'static str {
  COPY
}

pub(crate) fn check() -> &'static str {
  CHECK
}

/// The chevron points back into the window.
pub(crate) fn chevron(collapsed: Collapsed) -> &'static str {
  match (collapsed.axis, collapsed.corner) {
    (Axis::Horizontal, Corner::TopLeft | Corner::BottomLeft) => CHEVRON_RIGHT,
    (Axis::Horizontal, Corner::TopRight | Corner::BottomRight) => CHEVRON_LEFT,
    (Axis::Vertical, Corner::TopLeft | Corner::TopRight) => CHEVRON_DOWN,
    (Axis::Vertical, Corner::BottomLeft | Corner::BottomRight) => CHEVRON_UP,
  }
}

pub(crate) fn recolor(svg: &str, color: u32) -> String {
  let color = format!("#{:06x}", color & 0xffffff);
  svg.replace("currentColor", &color)
}

pub(crate) fn icon(svg: &str, color: u32, size: f32) -> gpui::Img {
  img(Arc::new(Image::from_bytes(
    ImageFormat::Svg,
    recolor(svg, color).into_bytes(),
  )))
  .size(px(size))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recoloring_replaces_current_color() {
    assert_eq!(
      recolor(r#"<svg stroke="currentColor" />"#, 0x12abef),
      "<svg stroke=\"#12abef\" />"
    );
  }
}
