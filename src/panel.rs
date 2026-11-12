//! The inspector, drawn inside the same widget shell as the toolbar.

use gpui::{
  Context, Empty, Inspector, InspectorElementId, IntoElement, Window, div, prelude::*, px, rgb,
};

use crate::{
  dock::{self, WidgetContent},
  geom::{self, WidgetView},
  icons,
};

/// GPUI reserves this many rems for the inspector while it is open.
const INSPECTOR_REMS: f32 = 30.0;

/// Paints the widget in the inspector column while an element is focused.
///
/// The column is the rightmost `30rem` of the window and is painted after the application, so it
/// is the only thing that can receive clicks there. The dock keeps painting outlines and the
/// pointer listeners; this renderer paints the shell.
pub(crate) fn render(
  inspector: &mut Inspector,
  window: &mut Window,
  cx: &mut Context<Inspector>,
) -> gpui::AnyElement {
  if inspector.is_picking() {
    return Empty.into_any_element();
  }
  let Some(id) = inspector.active_element_id().cloned() else {
    return Empty.into_any_element();
  };

  let now = web_time::Instant::now();
  let viewport = window.viewport_size();
  let column_x = viewport.width.as_f32() - f32::from(window.rem_size()) * INSPECTOR_REMS;
  let dock = dock::dock_for_window(window, cx);

  let (chrome, config) = {
    let shared = crate::state::global(cx);
    let mut scan = shared.borrow_mut();
    scan.widget.note_inspector_paint();
    if scan.widget.view != WidgetView::Inspector {
      scan.widget.set_view(WidgetView::Inspector, now);
    }
    scan
      .widget
      .place(viewport.width.as_f32(), viewport.height.as_f32(), now);
    let chrome = dock::chrome_from(&scan.widget, scan.widget.x - column_x, now);
    let config = scan.config().clone();
    (chrome, config)
  };

  let bar = dock.read(cx).bar(chrome.view, now);
  let source = source_location(&id);
  let name = element_name(&id);
  let states = inspector.render_inspector_states(window, cx);
  let header = render_header(dock.clone(), name, source, bar.copied, &config);
  // Height follows the states. A `size_full` child is exactly the scrollport, so the wheel
  // clamps straight back to the top and the rest of the panel stays clipped.
  let body = div()
    .id("gpui-scan-panel-body")
    .debug_selector(|| "gpui-scan-panel-body".into())
    .w_full()
    .flex_shrink_0()
    .p(px(10.0))
    .flex()
    .flex_col()
    .gap(px(10.0))
    .children(states)
    .into_any_element();

  div()
    .size_full()
    .child(dock::render_chrome(
      dock,
      chrome,
      bar,
      WidgetContent {
        header: Some(header.into_any_element()),
        body: Some(body),
      },
      &config,
    ))
    .into_any_element()
}

fn render_header(
  dock: gpui::Entity<dock::ScanDock>,
  name: String,
  source: String,
  copied: bool,
  config: &crate::config::Config,
) -> gpui::Stateful<gpui::Div> {
  let copy_text = source.clone();
  let icon_color = if copied { 0x22c55e } else { 0xffffff };
  let icon = if copied {
    icons::check()
  } else {
    icons::copy()
  };

  div()
    .id("gpui-scan-panel-header")
    .debug_selector(|| "gpui-scan-panel-header".into())
    .h(px(36.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .pl(px(12.0))
    .pr(px(8.0))
    .gap(px(8.0))
    .border_b_1()
    .border_color(rgb(geom::LINE))
    .child(
      div()
        .flex_1()
        .min_w(px(0.0))
        .overflow_hidden()
        .child(div().text_size(px(13.0)).child(name))
        .child(
          div()
            .truncate()
            .text_size(px(11.0))
            .text_color(rgb(config.muted_text))
            .child(source),
        ),
    )
    .child(
      header_button("gpui-scan-panel-copy", icon, icon_color).on_click({
        let dock = dock.clone();
        move |_, _, cx| dock::copy_from_panel(&dock, copy_text.clone(), cx)
      }),
    )
    .child(
      header_button("gpui-scan-panel-close", icons::close(), 0xffffff).on_click({
        let dock = dock.clone();
        move |_, window, cx| dock::dismiss_from_panel(&dock, window, cx)
      }),
    )
}

fn header_button(id: &'static str, markup: &'static str, color: u32) -> gpui::Stateful<gpui::Div> {
  div()
    .id(id)
    .p(px(4.0))
    .rounded(px(4.0))
    .cursor_pointer()
    .bg(rgb(0xffffff).opacity(0.1))
    .hover(|style| style.bg(rgb(0xffffff).opacity(0.15)))
    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .child(icons::icon(markup, color, 15.0))
}

fn element_name(id: &InspectorElementId) -> String {
  let global = id.path.global_id.to_string();
  let tail = global.rsplit('.').next().unwrap_or(global.as_str());
  let name = tail.rsplit("::").next().unwrap_or(tail);
  if name.is_empty() {
    "Element".to_string()
  } else {
    name.to_string()
  }
}

fn source_location(id: &InspectorElementId) -> String {
  let location = id.path.source_location;
  let file = std::path::Path::new(location.file())
    .components()
    .filter_map(|component| match component {
      std::path::Component::Normal(component) => component.to_str(),
      _ => None,
    })
    .collect::<Vec<_>>();
  let start = file
    .iter()
    .rposition(|component| *component == "src")
    .unwrap_or_else(|| file.len().saturating_sub(1));
  format!(
    "{}:{}:{}",
    file[start..].join("/"),
    location.line(),
    location.column()
  )
}
