//! The gpui-scan landing page, compiled to wasm and rendered with gpui-kit.

use std::{sync::Arc, time::Duration};

use gpui_kit::base::{Button, Tab};
use gpui_kit::{
  ClipboardItem, Context, Entity, FontWeight, Image, ImageFormat, IntoElement, ParentElement,
  Render, Rgba, SharedString, Styled, Window, div, img, prelude::*, px, rgb,
};
use gpui_scan_basic::{Demo, demo};

const GITHUB: &str = "https://github.com/encryptedbusiness/gpui-scan";
const DOCS: &str = "https://docs.rs/gpui-scan";

const ACCENT: u32 = 0x8e61e3;

const INSPECT_ICON: &str = include_str!("../../../assets/icons/inspect.svg");
const GITHUB_ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd"/></svg>"#;

const CARGO_SNIPPET: &str =
  "[dependencies]\ngpui = { package = \"gpui-pre\", version = \"0.3\" }\ngpui-scan = \"0.1\"";
const INIT_SNIPPET: &str = "fn main() {\n    gpui_platform::application().run(|cx| {\n        gpui_scan::init(cx);\n    });\n}";
const TRACK_SNIPPET: &str = "#[gpui_scan::scan]\nimpl Render for Counter {\n    fn render(&mut self, _: &mut Window,\n              _: &mut Context<Self>) -> impl IntoElement {\n        div().child(self.count.to_string())\n    }\n}";

/// One install tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallTab {
  Cargo,
  Init,
  Track,
}

impl InstallTab {
  fn id(self) -> &'static str {
    match self {
      InstallTab::Cargo => "install-cargo",
      InstallTab::Init => "install-init",
      InstallTab::Track => "install-track",
    }
  }

  fn label(self) -> &'static str {
    match self {
      InstallTab::Cargo => "Cargo.toml",
      InstallTab::Init => "main.rs",
      InstallTab::Track => "Track a view",
    }
  }

  fn code(self) -> &'static str {
    match self {
      InstallTab::Cargo => CARGO_SNIPPET,
      InstallTab::Init => INIT_SNIPPET,
      InstallTab::Track => TRACK_SNIPPET,
    }
  }
}

/// The landing page. The install card and the demo panel are tracked content.
pub struct Site {
  demo: Entity<Demo>,
  install: Entity<Install>,
}

impl Site {
  pub fn new(cx: &mut Context<Self>) -> Self {
    Self {
      demo: cx.new(demo),
      install: cx.new(Install::new),
    }
  }

  fn nav(&self, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .items_center()
      .justify_between()
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(10.0))
          .child(
            div()
              .size(px(30.0))
              .rounded(px(8.0))
              .bg(rgb(ACCENT))
              .flex()
              .items_center()
              .justify_center()
              .child(icon(INSPECT_ICON, 0xffffff, 15.0)),
          )
          .child(
            div()
              .text_size(px(17.0))
              .font_weight(FontWeight::SEMIBOLD)
              .child("gpui-scan"),
          ),
      )
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(20.0))
          .child(nav_link("nav-docs", "docs", DOCS))
          .child(nav_link("nav-github", "github", GITHUB)),
      )
  }

  fn hero(&self) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap(px(18.0))
      .child(pill("React Scan for GPUI"))
      .child(
        div()
          .flex()
          .flex_col()
          .child(
            div()
              .text_size(px(46.0))
              .font_weight(FontWeight::BOLD)
              .child("See exactly what"),
          )
          .child(
            div()
              .text_size(px(46.0))
              .font_weight(FontWeight::BOLD)
              .text_color(rgb(ACCENT))
              .child("re-rendered."),
          ),
      )
      .child(
        div()
          .max_w(px(620.0))
          .text_size(px(17.0))
          .text_color(white(0.6))
          .child(
            "gpui-scan is a React Scan-style overlay for GPUI. It outlines the views that called \
             cx.notify(), reports frame rate, and opens a devtools inspector for any element you \
             pick.",
          ),
      )
      .child(
        div()
          .flex()
          .flex_col()
          .gap(px(8.0))
          .child(bullet("No call-site wrappers. Add one attribute."))
          .child(bullet("Outlines only the views that actually changed."))
          .child(bullet("Works with any GPUI app, desktop or web.")),
      )
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(10.0))
          .pt(px(6.0))
          .child(primary_button())
          .child(secondary_button()),
      )
  }

  fn features(&self) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap(px(16.0))
      .child(label("What you get"))
      .child(
        div()
          .w_full()
          .flex()
          .flex_wrap()
          .gap(px(12.0))
          .child(feature(
            "Re-render outlines",
            "A purple outline with a Name ×count label follows every view that called cx.notify().",
          ))
          .child(feature(
            "Live FPS",
            "A frame-rate readout that turns amber under 50 and red under 30.",
          ))
          .child(feature(
            "Element inspector",
            "Click the crosshair, then any element. The full gpui-devtools panel opens in the widget.",
          )),
      )
  }

  fn demo_section(&self) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap(px(14.0))
      .child(label("Demo"))
      .child(
        div()
          .text_size(px(30.0))
          .font_weight(FontWeight::BOLD)
          .child("Try it on a real GPUI app"),
      )
      .child(
        div()
          .max_w(px(620.0))
          .text_size(px(15.0))
          .text_color(white(0.6))
          .child(
            "This is the same demo compiled to wasm. Outlines are on: click Increment or \
             Refresh and watch the views light up.",
          ),
      )
      .child(
        div()
          .rounded(px(12.0))
          .border_1()
          .border_color(white(0.1))
          .bg(rgb(0x0a0a0a))
          .overflow_hidden()
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(8.0))
              .px(px(14.0))
              .py(px(10.0))
              .border_b_1()
              .border_color(white(0.08))
              .child(div().size(px(7.0)).rounded_full().bg(rgb(0xef4444)))
              .child(div().size(px(7.0)).rounded_full().bg(rgb(0xf59e0b)))
              .child(div().size(px(7.0)).rounded_full().bg(rgb(0x22c55e)))
              .child(
                div()
                  .ml(px(6.0))
                  .text_size(px(11.0))
                  .text_color(white(0.4))
                  .child("demo.rs"),
              ),
          )
          .child(div().p(px(16.0)).child(self.demo.clone())),
      )
  }

  fn footer(&self) -> impl IntoElement {
    div()
      .border_t_1()
      .border_color(white(0.08))
      .pt(px(18.0))
      .flex()
      .items_center()
      .justify_between()
      .child(
        div()
          .text_size(px(12.0))
          .text_color(white(0.4))
          .child("MIT licensed · Built with GPUI and gpui-kit"),
      )
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(16.0))
          .child(nav_link("footer-github", "GitHub", GITHUB))
          .child(nav_link("footer-docs", "Docs", DOCS)),
      )
  }
}

/// The install card. Tracking this view is what makes a tab change show a scan outline.
pub struct Install {
  tab: InstallTab,
  copied: bool,
}

impl Install {
  fn new(_cx: &mut Context<Self>) -> Self {
    Self {
      tab: InstallTab::Cargo,
      copied: false,
    }
  }

  fn copy(&mut self, cx: &mut Context<Self>) {
    cx.write_to_clipboard(ClipboardItem::new_string(self.tab.code().to_string()));
    self.copied = true;
    cx.notify();
    cx.spawn(async move |this, cx| {
      cx.background_executor()
        .timer(Duration::from_millis(2000))
        .await;
      let _ = this.update(cx, |this, cx| {
        this.copied = false;
        cx.notify();
      });
    })
    .detach();
  }

  fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .items_center()
      .gap(px(18.0))
      .px(px(16.0))
      .pt(px(12.0))
      .border_b_1()
      .border_color(white(0.1))
      .child(self.tab(InstallTab::Cargo, cx))
      .child(self.tab(InstallTab::Init, cx))
      .child(self.tab(InstallTab::Track, cx))
  }

  fn tab(&self, tab: InstallTab, cx: &mut Context<Self>) -> Tab {
    let install = cx.entity().downgrade();
    let selected = self.tab == tab;
    Tab::new(tab.id())
      .selected(selected)
      .on_click(move |_, _, cx| {
        if let Some(install) = install.upgrade() {
          install.update(cx, |install, cx| {
            install.tab = tab;
            install.copied = false;
            cx.notify();
          });
        }
      })
      .pb(px(10.0))
      .text_size(px(12.0))
      .text_color(if selected { rgb(0xffffff) } else { white(0.5) })
      .border_b_1()
      .border_color(if selected {
        rgb(0xffffff)
      } else {
        rgb(0x000000).opacity(0.0)
      })
      .hover(|style| style.text_color(rgb(0xffffff)))
      .child(tab.label())
  }

  fn copy_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let install = cx.entity().downgrade();
    let copied = self.copied;
    Button::new("copy-snippet")
      .h(px(28.0))
      .px(px(10.0))
      .rounded(px(5.0))
      .border_1()
      .border_color(white(0.12))
      .bg(rgb(0xffffff).opacity(0.04))
      .text_size(px(11.0))
      .text_color(if copied { rgb(0x22c55e) } else { white(0.7) })
      .hover(|style| style.bg(rgb(0xffffff).opacity(0.08)))
      .on_click(move |_, _, cx| {
        if let Some(install) = install.upgrade() {
          install.update(cx, |install, cx| install.copy(cx));
        }
      })
      .child(if copied { "Copied" } else { "Copy" })
  }
}

#[gpui_scan::scan]
impl Render for Install {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap(px(12.0))
      .child(label("Add it to your app"))
      .child(
        div()
          .overflow_hidden()
          .rounded(px(10.0))
          .border_1()
          .border_color(white(0.1))
          .bg(rgb(0xffffff).opacity(0.03))
          .child(self.tabs(cx))
          .child(
            div()
              .bg(rgb(0x000000).opacity(0.6))
              .p(px(16.0))
              .flex()
              .items_start()
              .justify_between()
              .gap(px(16.0))
              .child(
                div()
                  .flex()
                  .flex_col()
                  .gap(px(2.0))
                  .text_size(px(11.5))
                  .text_color(white(0.75))
                  .children(
                    self
                      .tab
                      .code()
                      .lines()
                      .map(|line| div().child(SharedString::from(line.to_string()))),
                  ),
              )
              .child(self.copy_button(cx)),
          ),
      )
  }
}

impl Render for Site {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    gpui_scan::scan_root(
      div()
        .id("site-scroll")
        .size_full()
        .overflow_y_scroll()
        .bg(rgb(0x000000))
        .text_color(rgb(0xf4f4f5))
        .child(
          div()
            .w_full()
            .max_w(px(780.0))
            .mx_auto()
            .px(px(24.0))
            .pt(px(40.0))
            .pb(px(120.0))
            .flex()
            .flex_col()
            .gap(px(56.0))
            .child(self.nav(cx))
            .child(self.hero())
            .child(self.install.clone())
            .child(self.features())
            .child(self.demo_section())
            .child(self.footer()),
        ),
    )
  }
}

fn white(alpha: f32) -> Rgba {
  rgb(0xffffff).opacity(alpha)
}

fn icon(svg: &str, color: u32, size: f32) -> impl IntoElement {
  let color = format!("#{:06x}", color & 0xffffff);
  let svg = svg.replace("currentColor", &color);
  img(Arc::new(Image::from_bytes(
    ImageFormat::Svg,
    svg.into_bytes(),
  )))
  .size(px(size))
}

fn pill(text: &'static str) -> impl IntoElement {
  div()
    .self_start()
    .rounded_full()
    .border_1()
    .border_color(white(0.12))
    .bg(rgb(0xffffff).opacity(0.04))
    .px(px(10.0))
    .py(px(4.0))
    .text_size(px(11.0))
    .text_color(white(0.6))
    .child(text)
}

fn label(text: &'static str) -> impl IntoElement {
  div()
    .text_size(px(12.0))
    .font_weight(FontWeight::SEMIBOLD)
    .text_color(rgb(ACCENT))
    .child(text)
}

fn bullet(text: &'static str) -> impl IntoElement {
  div()
    .flex()
    .items_center()
    .gap(px(10.0))
    .child(div().size(px(5.0)).rounded_full().bg(rgb(ACCENT)))
    .child(div().text_size(px(15.0)).text_color(white(0.7)).child(text))
}

fn nav_link(id: &'static str, label: &'static str, url: &'static str) -> impl IntoElement {
  Button::new(id)
    .h(px(24.0))
    .px(px(0.0))
    .text_size(px(14.0))
    .text_color(white(0.5))
    .underline()
    .hover(|style| style.text_color(rgb(0xffffff)))
    .on_click(move |_, _, cx| cx.open_url(url))
    .child(label)
}

fn primary_button() -> impl IntoElement {
  Button::new("star-github")
    .h(px(34.0))
    .px(px(14.0))
    .rounded(px(6.0))
    .bg(rgb(0xffffff))
    .text_color(rgb(0x000000))
    .text_size(px(14.0))
    .font_weight(FontWeight::MEDIUM)
    .hover(|style| style.bg(rgb(0xf4f4f5)))
    .on_click(|_, _, cx| cx.open_url(GITHUB))
    .child(
      div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(icon(GITHUB_ICON, 0x000000, 15.0))
        .child("Star on GitHub"),
    )
}

fn secondary_button() -> impl IntoElement {
  Button::new("read-docs")
    .h(px(34.0))
    .px(px(14.0))
    .rounded(px(6.0))
    .border_1()
    .border_color(white(0.12))
    .bg(rgb(0xffffff).opacity(0.05))
    .text_size(px(14.0))
    .text_color(rgb(0xffffff))
    .hover(|style| style.bg(rgb(0xffffff).opacity(0.09)))
    .on_click(|_, _, cx| cx.open_url(DOCS))
    .child("Read the docs")
}

fn feature(title: &'static str, body: &'static str) -> impl IntoElement {
  div()
    .flex_1()
    .min_w(px(200.0))
    .min_h(px(120.0))
    .p(px(16.0))
    .rounded(px(10.0))
    .border_1()
    .border_color(white(0.08))
    .bg(rgb(0xffffff).opacity(0.03))
    .flex()
    .flex_col()
    .gap(px(6.0))
    .child(
      div()
        .text_size(px(14.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(title),
    )
    .child(div().text_size(px(13.0)).text_color(white(0.5)).child(body))
}
