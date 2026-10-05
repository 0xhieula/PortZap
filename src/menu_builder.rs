//! Assembly of the popup menu. Row rendering lives in [`crate::port_row`].

use objc2::runtime::{AnyObject, Sel};
use objc2::sel;
use objc2_app_kit::{NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};

use crate::port_row;
use crate::ports::PortEntry;

/// Rebuild the menu in place: header, one row per port, then Refresh and Quit.
pub fn build_menu(
    menu: &NSMenu,
    entries: &[PortEntry],
    error: Option<&str>,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    menu.removeAllItems();

    add_header(menu, &header_text(entries.len(), error), mtm);

    if let Some(message) = error {
        add_header(menu, message, mtm);
    } else if entries.is_empty() {
        add_header(menu, "No listening ports", mtm);
    }

    for (index, entry) in entries.iter().enumerate() {
        let item = NSMenuItem::new(mtm);
        item.setView(Some(&port_row::build(entry, index, target, mtm)));
        menu.addItem(&item);
    }

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    add_action_item(menu, "Refresh", sel!(refreshClicked:), "r", target, mtm);
    add_action_item(menu, "Quit PortZap", sel!(quitClicked:), "q", target, mtm);
}

fn header_text(count: usize, error: Option<&str>) -> String {
    if error.is_some() {
        return "Port Monitor".to_string();
    }
    match count {
        1 => "Port Monitor — 1 port".to_string(),
        n => format!("Port Monitor — {n} ports"),
    }
}

/// A non-interactive label row.
fn add_header(menu: &NSMenu, title: &str, mtm: MainThreadMarker) {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(title));
    item.setEnabled(false);
    menu.addItem(&item);
}

fn add_action_item(
    menu: &NSMenu,
    title: &str,
    action: Sel,
    key: &str,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(title));
    item.setKeyEquivalent(&NSString::from_str(key));
    // SAFETY: `target` outlives the menu (held by the delegate, which owns both), and the
    // selector is defined on that class.
    unsafe {
        item.setTarget(Some(target));
        item.setAction(Some(action));
    }
    menu.addItem(&item);
}
