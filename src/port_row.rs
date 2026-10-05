//! Layout of one port's row in the menu.
//!
//! Produces the text runs and their boxes; the view that draws them — and tracks hover — lives
//! in [`crate::port_row_view`].

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::{
    NSBezelStyle, NSButton, NSColor, NSControlSize, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSLineBreakMode, NSMutableParagraphStyle,
    NSParagraphStyleAttributeName,
};
use objc2_foundation::{
    MainThreadMarker, NSAttributedString, NSDictionary, NSPoint, NSRect, NSSize, NSString,
};

use crate::port_row_view::{DrawnText, PortRow};
use crate::ports::PortEntry;

/// Row geometry, in points. Cocoa's origin is bottom-left, so larger `y` is higher up.
///
/// The width is driven by the longest process names in practice (`com.docker.backend` and
/// friends); anything narrower truncates them to meaninglessness.
const ROW_WIDTH: f64 = 340.0;
const ROW_HEIGHT: f64 = 44.0;

const PAD: f64 = 14.0;
/// Box of the upper line (port and process name).
const LINE_1_Y: f64 = 20.0;
const LINE_1_H: f64 = 18.0;
/// Box of the lower line (PID).
const LINE_2_Y: f64 = 5.0;
const LINE_2_H: f64 = 14.0;

const PORT_W: f64 = 70.0;
const NAME_X: f64 = PAD + PORT_W + 4.0;
const DOT_SIZE: f64 = 8.0;
const BUTTON_W: f64 = 52.0;
const BUTTON_H: f64 = 22.0;

const BUTTON_X: f64 = ROW_WIDTH - PAD - BUTTON_W;
const DOT_X: f64 = BUTTON_X - 16.0;
/// The name gets whatever horizontal space the status dot does not claim.
const NAME_W: f64 = DOT_X - 8.0 - NAME_X;

/// Build the row for `entry`.
///
/// The Kill button carries `index` as its `tag`, which the action handler maps back to a
/// `PortEntry`. Passing an index rather than a boxed pointer keeps process state owned by Rust
/// and leaves nothing for Objective-C to outlive.
pub fn build(
    entry: &PortEntry,
    index: usize,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<PortRow> {
    let texts = vec![
        DrawnText {
            text: attributed(
                &format!(":{}", entry.port),
                NSFont::boldSystemFontOfSize(13.0),
                NSColor::labelColor(),
            ),
            rect: NSRect::new(NSPoint::new(PAD, LINE_1_Y), NSSize::new(PORT_W, LINE_1_H)),
        },
        DrawnText {
            text: attributed(
                &entry.process,
                NSFont::systemFontOfSize(13.0),
                NSColor::labelColor(),
            ),
            rect: NSRect::new(NSPoint::new(NAME_X, LINE_1_Y), NSSize::new(NAME_W, LINE_1_H)),
        },
        DrawnText {
            text: attributed(
                &format!("PID {}", entry.pid),
                NSFont::systemFontOfSize(11.0),
                NSColor::secondaryLabelColor(),
            ),
            rect: NSRect::new(NSPoint::new(PAD, LINE_2_Y), NSSize::new(200.0, LINE_2_H)),
        },
    ];

    // Centred on the upper line, so the dot reads as belonging to the process name beside it.
    let dot = NSRect::new(
        NSPoint::new(DOT_X, LINE_1_Y + (LINE_1_H - DOT_SIZE) / 2.0),
        NSSize::new(DOT_SIZE, DOT_SIZE),
    );

    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(ROW_WIDTH, ROW_HEIGHT));
    let row = PortRow::new(frame, texts, dot, mtm);
    // The button is the only part that must stay a real view: it has to be clickable.
    row.addSubview(&kill_button(index, target, mtm));
    row
}

fn kill_button(index: usize, target: &AnyObject, mtm: MainThreadMarker) -> Retained<NSButton> {
    // SAFETY: `target` outlives the row, and `killClicked:` is defined on its class.
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("Kill"),
            Some(target),
            Some(sel!(killClicked:)),
            mtm,
        )
    };
    button.setFrame(NSRect::new(
        NSPoint::new(BUTTON_X, (ROW_HEIGHT - BUTTON_H) / 2.0),
        NSSize::new(BUTTON_W, BUTTON_H),
    ));
    button.setBezelStyle(NSBezelStyle::Push);
    button.setControlSize(NSControlSize::Small);
    button.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    button.setTag(index as isize);
    button
}

/// Compose a text run that ellipsises rather than clipping when it overflows its box.
fn attributed(
    text: &str,
    font: Retained<NSFont>,
    color: Retained<NSColor>,
) -> Retained<NSAttributedString> {
    let style = NSMutableParagraphStyle::new();
    style.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);

    // Deref coercion down each class hierarchy to the untyped value the dictionary expects.
    let font_value: &AnyObject = &font;
    let color_value: &AnyObject = &color;
    let style_value: &AnyObject = &style;

    // SAFETY: the three keys are framework constants, and every value outlives the dictionary,
    // which copies its contents.
    let attributes = unsafe {
        NSDictionary::from_slices(
            &[
                NSFontAttributeName,
                NSForegroundColorAttributeName,
                NSParagraphStyleAttributeName,
            ],
            &[font_value, color_value, style_value],
        )
    };

    // SAFETY: the attribute dictionary is well-formed for text rendering.
    unsafe {
        NSAttributedString::initWithString_attributes(
            NSAttributedString::alloc(),
            &NSString::from_str(text),
            Some(&attributes),
        )
    }
}
