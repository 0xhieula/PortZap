//! The `NSView` subclass backing a port row: hover tracking and drawing.
//!
//! A menu item showing a custom view loses the standard highlight AppKit draws for plain items,
//! so the hover state has to be tracked and painted here.
//!
//! Text is drawn directly rather than hosted in `NSTextField` subviews. A text field is a full
//! `NSControl` with a backing cell, and the four per row are static and non-interactive — with a
//! dozen ports that is ~48 control objects built, laid out and torn down on every menu open, for
//! something `drawInRect:` does in one call. Row content and layout live in [`crate::port_row`].

use std::cell::Cell;

use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAttributedStringNSStringDrawing, NSBezierPath, NSColor, NSEvent, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSAttributedString, NSObjectProtocol, NSPoint, NSRect, NSSize,
};

/// A pre-built run of text and the box to draw it in.
///
/// Built once when the row is created, so repeated redraws (hover in, hover out) do not rebuild
/// attribute dictionaries.
pub struct DrawnText {
    pub text: Retained<NSAttributedString>,
    pub rect: NSRect,
}

pub struct RowIvars {
    hovered: Cell<bool>,
    texts: Vec<DrawnText>,
    /// Box of the green "running" dot.
    dot: NSRect,
}

define_class!(
    // SAFETY:
    // - NSView imposes no subclassing requirements beyond main-thread use.
    // - PortRow does not implement Drop.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "PortZapRow"]
    #[ivars = RowIvars]
    pub struct PortRow;

    impl PortRow {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let ivars = self.ivars();

            if ivars.hovered.get() {
                let bounds = self.bounds();
                // Inset so the highlight reads as a pill inside the menu, matching how AppKit
                // draws selection for ordinary menu items.
                let pill = NSRect::new(
                    NSPoint::new(bounds.origin.x + 5.0, bounds.origin.y + 1.0),
                    NSSize::new(bounds.size.width - 10.0, bounds.size.height - 2.0),
                );
                NSColor::unemphasizedSelectedContentBackgroundColor().setFill();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(pill, 5.0, 5.0).fill();
            }

            for drawn in &ivars.texts {
                drawn.text.drawInRect(drawn.rect);
            }

            NSColor::systemGreenColor().setFill();
            NSBezierPath::bezierPathWithOvalInRect(ivars.dot).fill();
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.set_hovered(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.set_hovered(false);
        }
    }

    unsafe impl NSObjectProtocol for PortRow {}
);

impl PortRow {
    pub fn new(
        frame: NSRect,
        texts: Vec<DrawnText>,
        dot: NSRect,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RowIvars {
            hovered: Cell::new(false),
            texts,
            dot,
        });
        let row: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        // `InVisibleRect` keeps the tracked area pinned to the view's visible bounds, so it never
        // needs rebuilding when the menu resizes. `ActiveAlways` is required because a menu is
        // not the key window while it is open.
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                frame,
                NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect,
                Some(&row),
                None,
            )
        };
        row.addTrackingArea(&area);
        row
    }

    fn set_hovered(&self, hovered: bool) {
        if self.ivars().hovered.replace(hovered) != hovered {
            self.setNeedsDisplay(true);
        }
    }

    /// Force the hover state, for rendering a preview of it outside a live menu.
    pub fn force_hover(&self, hovered: bool) {
        self.ivars().hovered.set(hovered);
    }

    /// Render this row to a PNG, so layout changes can be checked without opening the menu.
    pub fn dump_png(&self, path: &str) -> bool {
        use objc2_app_kit::NSBitmapImageFileType;
        use objc2_foundation::{NSDictionary, NSString};

        let bounds = self.bounds();
        let Some(rep) = self.bitmapImageRepForCachingDisplayInRect(bounds) else {
            return false;
        };
        self.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
        // SAFETY: an empty properties dictionary is valid for PNG output.
        let png = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
        };
        png.is_some_and(|data| data.writeToFile_atomically(&NSString::from_str(path), true))
    }
}
