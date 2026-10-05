//! Ways of telling the user something: the clipboard, and modal errors.

use objc2_app_kit::{NSAlert, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{MainThreadMarker, NSString};

/// Replace the clipboard contents with `text`. Returns false if the write was rejected.
pub fn copy_to_clipboard(text: &str) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    // Declaring ownership of the pasteboard is required before writing; without it
    // `setString:forType:` fails.
    pasteboard.clearContents();
    let value = NSString::from_str(text);
    // SAFETY: `NSPasteboardTypeString` is a framework constant, and `value` outlives the call.
    unsafe { pasteboard.setString_forType(&value, NSPasteboardTypeString) }
}

/// Modal error report.
///
/// A menu bar app would normally use a notification, but `UNUserNotificationCenter` requires a
/// signed bundle. A modal needs neither, and these paths are rare and worth interrupting for.
pub fn alert(title: &str, body: &str, mtm: MainThreadMarker) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(body));
    alert.runModal();
}
