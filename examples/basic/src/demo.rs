//! The demo application shown by the native and web examples.

use gpui::{Context, Entity, IntoElement, Render, Window, div, prelude::*, px, rgb};

const CARDS: [(&str, u32); 3] = [("Requests", 1284), ("Errors", 3), ("Latency", 42)];

struct Counter {
  count: u32,
}

#[gpui_scan::scan]
impl Render for Counter {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .justify_between()
      .gap_3()
      .p_4()
      .w_1_2()
      .h(px(140.0))
      .rounded_lg()
      .bg(rgb(0x18181b))
      .border_1()
      .border_color(rgb(0x27272a))
      .child(
        div()
          .text_sm()
          .text_color(rgb(0x8b8b8b))
          .child("Counter — only notifies itself"),
      )
      .child(
        div()
          .flex()
          .items_center()
          .gap_4()
          .child(div().text_2xl().child(self.count.to_string()))
          .child(
            div()
              .id("increment")
              .px_3()
              .py_1()
              .rounded_md()
              .bg(rgb(0x8e61e3))
              .cursor_pointer()
              .child("Increment")
              .on_click(cx.listener(|counter, _, _, cx| {
                counter.count += 1;
                cx.notify();
              })),
          ),
      )
  }
}

struct Pulse {
  phase: f32,
}

#[gpui_scan::scan]
impl Render for Pulse {
  fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    window.request_animation_frame();
    self.phase = (self.phase + 0.04) % std::f32::consts::TAU;
    let glow = ((self.phase.sin() + 1.0) / 2.0 * 0x6c as f32) as u32;

    div()
      .flex()
      .flex_col()
      .justify_between()
      .p_4()
      .w_1_2()
      .h(px(140.0))
      .rounded_lg()
      .bg(rgb(glow << 8 | 0x2c))
      .border_1()
      .border_color(rgb(0x27272a))
      .child(
        div()
          .text_sm()
          .text_color(rgb(0xd684f5))
          .child("Pulse — repaints every frame"),
      )
      .child(
        div()
          .size(px(18.0))
          .rounded_full()
          .bg(rgb(0xd684f5))
          .shadow_lg(),
      )
  }
}

struct Stats {
  refreshes: u32,
}

#[gpui_scan::scan]
impl Render for Stats {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .gap_3()
      .children(CARDS.iter().map(|(label, value)| {
        div()
          .flex_1()
          .p_4()
          .rounded_lg()
          .bg(rgb(0x18181b))
          .border_1()
          .border_color(rgb(0x27272a))
          .child(div().text_sm().text_color(rgb(0x8b8b8b)).child(*label))
          .child(div().text_xl().child((value + self.refreshes).to_string()))
      }))
      .child(
        div()
          .id("refresh")
          .px_3()
          .py_1()
          .self_center()
          .rounded_md()
          .bg(rgb(0x18181b))
          .border_1()
          .border_color(rgb(0x3f3f46))
          .cursor_pointer()
          .child("Refresh")
          .on_click(cx.listener(|stats, _, _, cx| {
            stats.refreshes += 1;
            cx.notify();
          })),
      )
  }
}

/// The demo's root view, holding the views the scanner tracks.
pub struct Demo {
  counter: Entity<Counter>,
  pulse: Entity<Pulse>,
  stats: Entity<Stats>,
}

impl Render for Demo {
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
        .child(
          div()
            .flex()
            .gap_4()
            .child(self.counter.clone())
            .child(self.pulse.clone()),
        )
        .child(self.stats.clone())
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

/// Creates the demo application view.
pub fn demo(cx: &mut Context<Demo>) -> Demo {
  Demo {
    counter: cx.new(|_| Counter { count: 0 }),
    pulse: cx.new(|_| Pulse { phase: 0.0 }),
    stats: cx.new(|_| Stats { refreshes: 0 }),
  }
}
