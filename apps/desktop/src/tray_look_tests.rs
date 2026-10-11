use super::{dot, drawn, DOT};
use crate::tray_menu::Look;

const SIDE: u32 = 64;
const BLUE: [u8; 4] = [0x20, 0x40, 0xc0, 0xff];

fn icon() -> Vec<u8> {
    BLUE.repeat((SIDE * SIDE) as usize)
}

fn at(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIDE + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn a_plain_icon_is_the_icon() {
    assert_eq!(drawn(&icon(), SIDE, SIDE, Look::Plain), icon());
}

/// The dot is drawn where `dot` says, cut out by a clear ring, and the rest
/// of the mark is left alone.
#[test]
fn waiting_puts_a_dot_on_the_mark_and_leaves_the_rest() {
    let shown = drawn(&icon(), SIDE, SIDE, Look::Waiting);
    let (cx, cy, radius) = dot(SIDE, SIDE);
    assert_eq!(at(&shown, cx as u32, cy as u32), DOT);
    let just_out = (cx + radius + 1.0) as u32;
    assert_eq!(at(&shown, just_out, cy as u32)[3], 0, "the ring is clear");
    assert_eq!(at(&shown, 2, 2), BLUE);
    assert!(cx + radius < SIDE as f32 && cy + radius < SIDE as f32);
}

/// Paused is grey and fainter, wherever the pixel is.
#[test]
fn paused_is_grey_and_fainter() {
    let shown = drawn(&icon(), SIDE, SIDE, Look::Paused);
    for (x, y) in [(0, 0), (SIDE / 2, SIDE / 2), (SIDE - 1, SIDE - 1)] {
        let [r, g, b, a] = at(&shown, x, y);
        assert!(r == g && g == b, "({x},{y}) is not grey");
        assert!(a < BLUE[3], "({x},{y}) is not fainter");
    }
}
