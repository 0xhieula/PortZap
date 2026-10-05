//! The menu bar icon: a three-armed whirlpool, drawn as vector paths.
//!
//! Drawn at runtime rather than shipped as an asset, so there is nothing to bundle and the shape
//! stays sharp at any backing scale factor.
//!
//! The result is a *template* image: macOS ignores its colour and re-tints it from the alpha
//! channel, rendering it white on a dark menu bar and black on a light one. Painting an actually
//! white icon would make it invisible in light mode.

use std::f64::consts::TAU;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_app_kit::{NSBezierPath, NSColor, NSImage};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// Arms sweeping out from the eye of the vortex.
const ARMS: usize = 3;
/// Turn fraction each arm sweeps. Beyond ~0.8 the arms start to overlap illegibly when small.
const TURNS: f64 = 0.7;
/// Polyline resolution of each arm outline. 160 is smooth past any size the menu bar uses.
const STEPS: usize = 160;

/// Build the whirlpool as a square template image `size` points on a side.
pub fn vortex_image(size: f64) -> Retained<NSImage> {
    let handler = RcBlock::new(move |_dirty: NSRect| -> Bool {
        draw_vortex(size);
        Bool::YES
    });

    let image = NSImage::imageWithSize_flipped_drawingHandler(
        NSSize::new(size, size),
        false,
        &handler,
    );
    image.setTemplate(true);
    image
}

/// Write the icon to a PNG, for eyeballing changes to the shape without launching the app.
///
/// Renders through `vortex_image`, so what lands on disk is what the menu bar gets. Template
/// tinting is a system effect and is not captured — the glyph appears black on transparent.
pub fn dump_png(size: f64, path: &str) -> bool {
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep};
    use objc2_foundation::{NSDictionary, NSString};

    let Some(tiff) = vortex_image(size).TIFFRepresentation() else {
        return false;
    };
    let Some(rep) = NSBitmapImageRep::imageRepWithData(&tiff) else {
        return false;
    };
    // SAFETY: an empty properties dictionary is valid for PNG output.
    let png = unsafe {
        rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    };
    png.is_some_and(|data| data.writeToFile_atomically(&NSString::from_str(path), true))
}

/// Paint the arms into the current graphics context.
fn draw_vortex(size: f64) {
    let centre = NSPoint::new(size / 2.0, size / 2.0);
    // Leave a margin so the shape is not clipped by the image bounds.
    let radius = size * 0.42;
    // Stroke weights were tuned against an 18pt icon; scale them with the canvas.
    let unit = size / 18.0;

    // Colour is irrelevant to a template image — only coverage matters — but something must be
    // set for the fill to produce alpha.
    NSColor::blackColor().setFill();

    for arm in 0..ARMS {
        let phase = arm as f64 * TAU / ARMS as f64;
        arm_outline(
            centre,
            radius * 0.16,
            radius,
            0.55 * unit,
            1.9 * unit,
            phase,
        )
        .fill();
    }
}

/// One arm, as a closed outline whose width tapers from the rim toward the eye.
///
/// `NSBezierPath` strokes at a single uniform width, so a taper has to be described as a filled
/// outline: walk the spiral centreline offsetting to one side, then walk back offsetting to the
/// other.
fn arm_outline(
    centre: NSPoint,
    r_inner: f64,
    r_outer: f64,
    w_inner: f64,
    w_outer: f64,
    phase: f64,
) -> Retained<NSBezierPath> {
    let mut left = Vec::with_capacity(STEPS + 1);
    let mut right = Vec::with_capacity(STEPS + 1);

    for step in 0..=STEPS {
        let t = step as f64 / STEPS as f64;
        let point = spiral_point(t, r_inner, r_outer, phase, centre);

        // Numeric tangent; the spiral has no cusps, so a finite difference is ample.
        let delta = 0.002;
        let before = spiral_point((t - delta).max(0.0), r_inner, r_outer, phase, centre);
        let after = spiral_point((t + delta).min(1.0), r_inner, r_outer, phase, centre);
        let (dx, dy) = (after.x - before.x, after.y - before.y);
        let length = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
        // Unit normal: the tangent rotated a quarter turn.
        let (nx, ny) = (-dy / length, dx / length);

        let half = (w_inner + (w_outer - w_inner) * t) / 2.0;
        left.push(NSPoint::new(point.x + nx * half, point.y + ny * half));
        right.push(NSPoint::new(point.x - nx * half, point.y - ny * half));
    }

    let path = NSBezierPath::new();
    path.moveToPoint(left[0]);
    for point in &left[1..] {
        path.lineToPoint(*point);
    }
    for point in right.iter().rev() {
        path.lineToPoint(*point);
    }
    path.closePath();
    path
}

/// Point at `t` (0 at the eye, 1 at the rim) along an Archimedean spiral.
fn spiral_point(t: f64, r_inner: f64, r_outer: f64, phase: f64, centre: NSPoint) -> NSPoint {
    let theta = phase + t * TURNS * TAU;
    let radius = r_inner + (r_outer - r_inner) * t;
    NSPoint::new(
        centre.x + radius * theta.cos(),
        centre.y + radius * theta.sin(),
    )
}
