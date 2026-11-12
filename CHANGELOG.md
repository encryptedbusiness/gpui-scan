# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-12

### Fixed

- Outlines follow their view when an ancestor scrolls, and stay pinned to it while a scroll is
  in flight instead of easing behind the content.

## [0.1.0] - 2026-11-12

### Added

- `#[gpui_scan::scan]` attribute for zero-boilerplate view tracking.
- React Scan style toolbar with element picking, an outline switch, and a live FPS readout.
- Re-render outlines with `Name ×count` labels that fade over 750ms and chase moved elements.
- Element picker backed by GPUI's inspector, opening gpui-devtools in the same widget shell.
- `Config` for key bindings, the starting outline state, and colors.
- `track_view`, `track_view_as`, `track_view_keyed`, and the `ScanTrackExt` methods.
- `scan_root` and `scan_layer` overlays for native and web targets.
- Native (`examples/basic`) and wasm (`examples/web`) demos.

[Unreleased]: https://github.com/encryptedbusiness/gpui-scan/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/encryptedbusiness/gpui-scan/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/encryptedbusiness/gpui-scan/releases/tag/v0.1.0
