/// Installation options for the scanner.
///
/// Colors are `0xRRGGBB` values, matching [`gpui::rgb`]. The defaults reproduce the React Scan
/// toolbar, so an application that does not configure anything still gets the familiar look.
#[derive(Clone, Debug)]
pub struct Config {
  /// Key binding for [`ToggleOutlines`](crate::ToggleOutlines), or `None` to register none.
  ///
  /// Defaults to `cmd-alt-s` on macOS and `ctrl-alt-s` elsewhere.
  pub key_binding: Option<&'static str>,
  /// Key binding for [`CancelPick`](crate::CancelPick), or `None` to register none.
  ///
  /// Defaults to `escape`. While picking, this leaves the picker. While an element is focused, it
  /// starts picking again.
  pub cancel_key_binding: Option<&'static str>,
  /// Whether re-render outlines start enabled.
  pub outlines: bool,
  /// Background of the toolbar.
  pub background: u32,
  /// Background of pressed and hovered controls.
  pub control: u32,
  /// Color of borders and separators.
  pub border: u32,
  /// Color of primary text.
  pub text: u32,
  /// Color of labels and secondary text.
  pub muted_text: u32,
  /// Color of enabled controls.
  pub accent: u32,
  /// Color of the FPS readout.
  pub fps: u32,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      key_binding: Some(default_key_binding()),
      cancel_key_binding: Some("escape"),
      outlines: false,
      background: 0x000000,
      control: 0x141414,
      border: 0x222222,
      text: 0xffffff,
      muted_text: 0x999999,
      accent: 0x8e61e3,
      fps: 0xd684f5,
    }
  }
}

impl Config {
  /// Sets the key binding for [`ToggleOutlines`](crate::ToggleOutlines), or removes it with `None`.
  pub fn key_binding(mut self, key_binding: Option<&'static str>) -> Self {
    self.key_binding = key_binding;
    self
  }

  /// Sets the key binding for [`CancelPick`](crate::CancelPick), or removes it with `None`.
  pub fn cancel_key_binding(mut self, key_binding: Option<&'static str>) -> Self {
    self.cancel_key_binding = key_binding;
    self
  }

  /// Sets whether re-render outlines start enabled.
  pub fn outlines(mut self, outlines: bool) -> Self {
    self.outlines = outlines;
    self
  }
}

const fn default_key_binding() -> &'static str {
  if cfg!(target_os = "macos") {
    "cmd-alt-s"
  } else {
    "ctrl-alt-s"
  }
}
