use crate::kinds::kind_of;
use devpit_rpc::FileKind;

/// The bytes decide before the extension does. A PNG called `notes.md` is
/// a PNG, and drawing it as markdown fills the pane with mojibake.
#[test]
fn the_first_bytes_beat_the_extension() {
    assert_eq!(kind_of("notes.md", b"\x89PNG\r\n\x1a\n"), FileKind::Image);
    assert_eq!(kind_of("notes.md", b"%PDF-1.7"), FileKind::Pdf);
}

#[test]
fn the_extension_settles_what_the_bytes_leave_open() {
    assert_eq!(kind_of("readme.md", b"# hello\n"), FileKind::Markdown);
    assert_eq!(kind_of("main.rs", b"fn main() {}"), FileKind::Text);
    assert_eq!(kind_of("logo.svg", b"<svg xmlns=\"...\">"), FileKind::Image);
}

/// A NUL in the first block is the oldest and most reliable sign that this
/// is not something to put in an editor.
#[test]
fn a_nul_byte_means_this_is_not_text() {
    assert_eq!(kind_of("thing.md", b"abc\0def"), FileKind::Binary);
}

#[test]
fn a_file_with_no_extension_is_text_when_its_bytes_are() {
    assert_eq!(kind_of("Makefile", b"all:\n\tcargo test\n"), FileKind::Text);
}

#[test]
fn every_image_format_the_screen_draws_is_recognised() {
    assert_eq!(kind_of("a", b"\xff\xd8\xff\xe0"), FileKind::Image);
    assert_eq!(kind_of("a", b"GIF89a...."), FileKind::Image);
    assert_eq!(kind_of("a", b"RIFF\x00\x00\x00\x00WEBP"), FileKind::Image);
}
