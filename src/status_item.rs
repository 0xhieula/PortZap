//! The menu bar item itself: creation and title updates.

use objc2::rc::Retained;
use objc2_app_kit::{
    NSCellImagePosition, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSString};

use crate::vortex_icon;

/// Side length of the whirlpool glyph, in points. The menu bar is 22pt tall, so this leaves a
/// little breathing room above and below.
const ICON_SIZE: f64 = 17.0;

/// Create the status bar item.
///
/// The returned value must be retained for as long as the icon should stay visible — dropping it
/// removes the item from the menu bar immediately.
pub fn create_status_item(mtm: MainThreadMarker) -> Retained<NSStatusItem> {
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = item.button(mtm) {
        button.setImage(Some(&vortex_icon::vortex_image(ICON_SIZE)));
        // Icon first, count after it.
        button.setImagePosition(NSCellImagePosition::ImageLeading);
    }
    item
}

/// Show the number of listening ports beside the icon.
pub fn set_count(item: &NSStatusItem, count: usize, mtm: MainThreadMarker) {
    set_title(item, &format!(" {count}"), mtm);
}

/// Show a warning glyph in place of the count, for states the count cannot express.
pub fn set_warning(item: &NSStatusItem, mtm: MainThreadMarker) {
    set_title(item, " \u{26A0}", mtm); // ⚠
}

fn set_title(item: &NSStatusItem, title: &str, mtm: MainThreadMarker) {
    if let Some(button) = item.button(mtm) {
        button.setTitle(&NSString::from_str(title));
    }
}
