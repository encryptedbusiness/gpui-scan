# Contributing

Thanks for taking the time to improve `gpui-scan`. This guide covers the local workflow and the
checks a pull request is expected to pass.

## Prerequisites

- Rust 1.98 or newer (`rustup toolchain install stable`).
- macOS or Linux. The crate is a GPUI overlay, so a working `gpui-pre` toolchain is required.
- For the web demo: `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli`.

## Local checks

Run the same gates as CI before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

`cargo test --workspace` covers the library, the macro crate, and both examples. The web example is
compiled for the host as part of that command; build it for wasm with `examples/web/build.sh`.

## Project layout

| Path | Contents |
| --- | --- |
| `src/` | The library: widget, outlines, tracking, state, and the inspector shell. |
| `gpui-scan-macros/` | The `#[scan]` attribute. |
| `examples/basic/` | The native demo and the shared `demo` view. |
| `examples/web/` | The wasm entry point and page for the web demo. |
| `assets/` | Icons and the README screenshots. |

## Making changes

- Keep pull requests focused. One behavior change per PR is easier to review than a batch of
  unrelated cleanups.
- Add a test for behavior changes. The existing tests in `src/tracked.rs` show how to drive the
  scanner with `gpui::TestAppContext`.
- Update `CHANGELOG.md` under `## [Unreleased]`.
- Run `cargo fmt` and the full check list above.

## Releasing

The workspace publishes two crates. `gpui-scan` depends on `gpui-scan-macros` by version, so the
macro crate has to be published first:

1. Bump `version` in `gpui-scan-macros/Cargo.toml` and `Cargo.toml`.
2. Move the `Unreleased` changelog entries under the new version and date.
3. `cargo publish -p gpui-scan-macros`
4. `cargo publish -p gpui-scan`
5. Tag the release and push the tag.

## License

By contributing, you agree that your contributions are licensed under the MIT license.
