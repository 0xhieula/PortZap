//! PortZap — a macOS menu bar app listing listening TCP ports with one-click process kill.

mod app_delegate;
mod feedback;
mod killer;
mod lsof_parser;
mod menu_builder;
mod port_row;
mod port_row_view;
mod ports;
mod status_item;
mod vortex_icon;

use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::MainThreadMarker;

fn main() {
    let mtm = MainThreadMarker::new().expect("PortZap must be started on the main thread");

    // Developer aid: render the menu bar glyph to a file instead of starting the app.
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, path, ..] = args.as_slice()
        && flag == "--dump-icon"
    {
        let size = std::env::var("ICON_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(17.0);
        println!("{}", if vortex_icon::dump_png(size, path) { "ok" } else { "failed" });
        return;
    }

    // Developer aid: render sample rows, normal and hovered, to check layout and truncation.
    if let [_, flag, dir, ..] = args.as_slice()
        && flag == "--dump-rows"
    {
        dump_rows(dir, mtm);
        return;
    }

    let app = NSApplication::sharedApplication(mtm);
    // Accessory: menu bar only — no Dock icon and no app menu.
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Held for the process lifetime: the delegate owns the status item, and dropping it would
    // remove the icon from the menu bar.
    let _delegate = app_delegate::Delegate::new(mtm);

    app.run();
}

/// Render representative rows to `dir`, covering a short name, a name long enough to need
/// truncating, and the hovered state.
fn dump_rows(dir: &str, mtm: MainThreadMarker) {
    use objc2::runtime::NSObject;
    use ports::PortEntry;

    let samples = [
        ("short", PortEntry { port: 3000, pid: 45919, process: "node".into() }, false),
        ("short-hover", PortEntry { port: 3000, pid: 45919, process: "node".into() }, true),
        ("long", PortEntry { port: 5432, pid: 40214, process: "com.docker.backend".into() }, false),
        ("overflow", PortEntry { port: 63342, pid: 50322, process: "an.absurdly.long.process.name.that.must.ellipsise".into() }, false),
    ];

    // Rows need an action target; nothing is clicked here, so a bare object suffices.
    let target = NSObject::new();
    for (name, entry, hovered) in samples {
        let row = port_row::build(&entry, 0, &target, mtm);
        row.force_hover(hovered);
        let path = format!("{dir}/row-{name}.png");
        println!("{} {path}", if row.dump_png(&path) { "ok" } else { "failed" });
    }
}
