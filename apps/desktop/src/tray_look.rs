//! The tray icon in each look: as it is, with a dot while something waits on
//! the person, and faded while devpit is paused.
//!
//! Drawn from the app's own icon rather than kept as more image files: a
//! second copy of the mark is one that drifts from the first.

use crate::tray_menu::Look;

/// The board's colour for a session waiting on you (`--warning`).
pub(crate) const DOT: [u8; 4] = [0xe0, 0xb3, 0x6a, 0xff];

/// Where the dot sits on a `width` × `height` icon: its centre and radius.
/// Large, because a tray draws the icon at 16 to 22 pixels.
pub(crate) fn dot(width: u32, height: u32) -> (f32, f32, f32) {
    let side = width.min(height) as f32;
    let radius = side * 0.24;
    let gap = ring(side);
    (
        width as f32 - radius - gap,
        height as f32 - radius - gap,
        radius,
    )
}

/// The clear ring that cuts the dot out of the mark, so it reads on any bar.
fn ring(side: f32) -> f32 {
    side * 0.07
}

/// `rgba`, `width` × `height`, as `look` draws it.
pub(crate) fn drawn(rgba: &[u8], width: u32, height: u32, look: Look) -> Vec<u8> {
    let mut out = rgba.to_vec();
    match look {
        Look::Plain => {}
        Look::Paused => faded(&mut out),
        Look::Waiting => dotted(&mut out, width, height),
    }
    out
}

fn faded(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let grey =
            (u32::from(px[0]) * 299 + u32::from(px[1]) * 587 + u32::from(px[2]) * 114) / 1000;
        px[..3].fill(grey as u8);
        px[3] = (u32::from(px[3]) * 3 / 5) as u8;
    }
}

fn dotted(rgba: &mut [u8], width: u32, height: u32) {
    let (cx, cy, radius) = dot(width, height);
    let edge = radius + ring(width.min(height) as f32);
    for (at, px) in rgba.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((at as u32 % width) as f32, (at as u32 / width) as f32);
        let far = (x + 0.5 - cx).hypot(y + 0.5 - cy);
        if far <= radius {
            px.copy_from_slice(&DOT);
        } else if far <= edge {
            px[3] = 0;
        }
    }
}

#[cfg(test)]
#[path = "tray_look_tests.rs"]
mod tests;
