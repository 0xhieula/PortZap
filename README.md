# PortZap

A macOS menu bar app that lists every listening TCP port and kills the process holding one in a
single click. Native Rust against the Cocoa APIs — no Electron, no WebView. 400 KB binary,
12.8 MB resident.

```
 ◉ 9                                    ← menu bar
┌──────────────────────────────────────┐
│ Port Monitor — 9 ports               │
├──────────────────────────────────────┤
│ :3000    node                ●  Kill │
│          PID 45919                   │
├──────────────────────────────────────┤
│ :5432    com.docker.backend  ●  Kill │
│          PID 40214                   │
├──────────────────────────────────────┤
│ Refresh                           ⌘R │
│ Quit PortZap                      ⌘Q │
└──────────────────────────────────────┘
```

---

## Requirements

| | |
|---|---|
| macOS | 12 (Monterey) or later |
| Architecture | Apple Silicon or Intel |
| Rust | 1.85 or later (edition 2024) |
| `lsof` | ships with macOS — nothing to install |

No Xcode needed, only the Command Line Tools that `rustup` already depends on.

## Install

### 1. Install Rust, if you do not have it

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version      # expect 1.85.0 or later
```

Already have Rust but an older version? `rustup update stable`.

### 2. Build

```sh
git clone https://github.com/0xhieula/PortZap.git
cd PortZap
./build-app-bundle.sh
```

This produces `PortZap.app` in the repo root. First build takes a minute or two while the
`objc2` crates compile; later builds are seconds.

### 3. Install and launch

```sh
cp -r PortZap.app /Applications/
open /Applications/PortZap.app
```

A whirlpool icon with a port count appears at the right of your menu bar. There is no Dock icon
and no window — that is intended.

### 4. First launch: Gatekeeper

The bundle is ad-hoc signed, not notarized, so macOS will refuse the first launch with
*"PortZap cannot be opened because the developer cannot be verified."*

**Right-click the app → Open → Open.** Once only; normal launches work afterwards.

If the dialog gives you no Open button, allow it from
**System Settings → Privacy & Security**, where a *"PortZap was blocked"* row appears with an
**Open Anyway** button.

### 5. Start at login (optional)

A port monitor is more useful if it is always there:

**System Settings → General → Login Items & Extensions → Open at Login → `+`**, then pick
`/Applications/PortZap.app`.

## Usage

- **Click the menu bar icon** to see every listening TCP port, sorted by port number.
- **Kill** terminates the process holding that port.
- **Refresh** (⌘R) re-reads the list immediately.
- **Quit** (⌘Q) exits; the icon disappears from the menu bar.

The list rebuilds every time you open the menu, so it is never stale. A 5-minute timer keeps the
count beside the icon current while the menu is closed.

## Killing processes

PortZap runs unprivileged and can only kill processes you own — which covers essentially every
dev server. Killing something owned by another user fails with `EPERM`; PortZap then copies
`sudo kill -9 <pid>` to your clipboard and tells you, rather than silently doing nothing.

SIGTERM is sent before SIGKILL so dev servers get to run their cleanup handlers and do not orphan
child processes. SIGKILL follows only if the process is still alive after 500 ms.

## Uninstall

```sh
rm -rf /Applications/PortZap.app
```

Remove the login item too, if you added one. PortZap writes no preferences, caches, or support
files, so nothing else is left behind.

## Troubleshooting

**No icon appears.** Check it is running: `pgrep -l portzap`. If it is, your menu bar may be
full — macOS silently drops status items when space runs out. Quit another menu bar app, or use
an app like Bartender/Ice to manage overflow.

**The icon shows ⚠ instead of a number.** `lsof` failed. Run
`lsof -nP -iTCP -sTCP:LISTEN` yourself to see the error; the menu shows the message too.

**Kill does nothing.** The process is owned by another user. A dialog says so and puts
`sudo kill -9 <pid>` on your clipboard — paste it into a terminal.

**A port is missing.** PortZap lists TCP listeners only. UDP and Unix sockets are not shown.

## How it works

| Concern | Approach |
|---|---|
| Port discovery | `lsof -nP -iTCP -sTCP:LISTEN -F pcn` (~30 ms) |
| Killing | `libc::kill` directly — SIGTERM, then SIGKILL after 500 ms |
| Refresh | On menu open, plus a 5-minute timer for the count |
| Icon | Vector-drawn template image, adapts to light/dark menu bar |

`lsof` field mode (`-F`) is used rather than the default table output because the table form
truncates command names to 9 characters (`com.docker.backend` → `com.docke`). A socket bound on
both IPv4 and IPv6 produces two rows, so results are deduplicated by `(pid, port)`.

## Memory

`ps` reports ~21 MB resident, but RSS counts shared framework pages mapped into every Cocoa
process. The figure macOS actually charges to PortZap — what Activity Monitor shows, and what
`vmmap` calls the physical footprint — is **12.8 MB**, of which ~6 MB is dirty.

For scale, measured on the same machine: Raycast 64 MB, ControlCenter 79 MB, Dock 123 MB.

Most of that 12.8 MB is AppKit itself, so there is little left to win. Two things keep it down:

- **No `NSTextField` for static text.** A text field is a full `NSControl` with a backing cell;
  four per row across a dozen ports is ~48 control objects rebuilt on every menu open. The row
  view draws its text with `drawInRect:` instead, leaving the Kill button as the only subview.
- **The 5-minute timer does not touch the menu.** It refreshes only the count, because rows are
  rebuilt when the menu opens — building them on a tick would construct a view per port that
  nobody is looking at.

## Known limitations

- **TCP only.** UDP listeners are not shown.
- **PID reuse.** A PID could in principle be recycled between the menu rendering and your click,
  killing the wrong process. The window is seconds at most, since rows rebuild on every open.
- **No notifications.** Errors use a modal dialog; `UNUserNotificationCenter` would require a
  signed, notarized bundle.

## Development

```sh
cargo test                                   # 16 unit tests
cargo run                                    # run without bundling
cargo run -- --dump-icon /tmp/icon.png       # render the menu bar glyph
ICON_SIZE=128 cargo run -- --dump-icon /tmp/big.png
cargo run -- --dump-rows /tmp                # render sample rows, incl. hover state
```

`cargo run` shows the menu bar icon but briefly flashes a Dock icon, because `LSUIElement` only
applies to a real bundle.

`--dump-rows` renders real `PortRow` views offscreen — normal, hovered, and with a process name
long enough to need ellipsising. Layout changes can be checked from the PNGs without opening the
menu, which matters because macOS hides the status item area from `screencapture`.

| File | Role |
|---|---|
| `src/ports.rs` | `lsof` execution and error types |
| `src/lsof_parser.rs` | field-mode parsing, dedupe, sort |
| `src/killer.rs` | signal escalation, errno classification |
| `src/vortex_icon.rs` | the whirlpool glyph, drawn with `NSBezierPath` |
| `src/port_row.rs` | a row's layout and text runs |
| `src/port_row_view.rs` | the row `NSView`: hover tracking and drawing |
| `src/menu_builder.rs` | menu assembly |
| `src/app_delegate.rs` | state ownership and all UI callbacks |
| `src/status_item.rs` | menu bar item and its title |
| `src/feedback.rs` | clipboard writes and modal errors |

Every file stays under 200 lines. `lsof_parser.rs` keeps its tests in `lsof_parser_tests.rs`,
included via `#[cfg(test)] #[path = …] mod tests;`.

Bindings are `objc2` 0.6 / `objc2-app-kit` 0.3. The older `cocoa` crate is deprecated — do not
reintroduce it.
