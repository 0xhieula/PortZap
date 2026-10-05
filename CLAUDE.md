# PortZap

## Project Overview

macOS menu bar app listing listening TCP ports with one-click process kill. Background-only
(no Dock icon, no window). Optimised for size and memory: 400 KB binary, 12.8 MB footprint.

## Tech Stack

- **Rust** 2024 edition, 1.85+
- **`objc2` 0.6 / `objc2-app-kit` 0.3** for Cocoa. The older `cocoa` crate is deprecated —
  do not reintroduce it.
- **`libc`** for signals. No async runtime; no UI framework beyond raw AppKit.

## Important Notes

- **Everything is main-thread.** `NSMenuDelegate` requires `MainThreadOnly`, so state uses
  `RefCell`, not locks. Thread a `MainThreadMarker` rather than reaching for `unsafe`.
- **`lsof` field mode (`-F`) is mandatory.** Table output truncates command names to 9 chars
  (`com.docker.backend` → `com.docke`). Dedupe by `(pid, port)` — one socket emits an IPv4 and
  an IPv6 row.
- **Kill-button tags index the snapshot.** Rows and the `entries` snapshot must be replaced in
  the same operation, or a tag will point at a stale entry.
- **The 5-minute timer must not rebuild the menu.** Rows are rebuilt on `menuNeedsUpdate:`;
  building them on a tick constructs a view per port that nobody is looking at.
- **Avoid `NSTextField` for static text.** Draw with `drawInRect:` instead — a text field is a
  full `NSControl` with a backing cell.
- **Verify UI changes with `--dump-rows`.** macOS hides the status item area from
  `screencapture`, so the menu cannot be screenshotted. `cargo run -- --dump-rows /tmp` renders
  real `PortRow` views offscreen, hover state included.
- **Check API signatures against the vendored crate source**, not memory — `objc2` renamed much
  between 0.5 and 0.6 (`declare_class!` → `define_class!`, `Id` → `Retained`, `msg_send_id!`
  merged into `msg_send!`). Sources live under `~/.cargo/registry/src/*/objc2-*/`.

## Conventions

- Every file under 200 lines. Large test modules go in a sibling file via
  `#[cfg(test)] #[path = "..."] mod tests;`.
- `snake_case` filenames (Rust convention).
- Run `cargo test` and `cargo build` before committing.

## Documentation

- Read `./README.md` for project context
- Docs: `./docs/` (generate with `/docs init`)
