use crate::kinds::{kind_of, stream_type};
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

#[test]
fn video_and_audio_are_told_by_their_containers() {
    let mp4 = b"\x00\x00\x00\x20ftypisom\x00\x00\x02\x00";
    assert_eq!(kind_of("clip.mp4", mp4), FileKind::Video);
    assert_eq!(
        kind_of("clip.mov", b"\x00\x00\x00\x14ftypqt  \x00\x00"),
        FileKind::Video
    );
    assert_eq!(
        kind_of("voice.m4a", b"\x00\x00\x00\x20ftypM4A \x00\x00"),
        FileKind::Audio
    );
    assert_eq!(
        kind_of("clip.webm", b"\x1a\x45\xdf\xa3\x9f\x42\x86\x81"),
        FileKind::Video
    );
    assert_eq!(
        kind_of("voice.ogg", b"OggS\x00\x02\x00\x00"),
        FileKind::Audio
    );
    assert_eq!(
        kind_of("clip.ogv", b"OggS\x00\x02\x00\x00"),
        FileKind::Video
    );
    assert_eq!(
        kind_of("a.wav", b"RIFF\x24\x08\x00\x00WAVEfmt "),
        FileKind::Audio
    );
    assert_eq!(
        kind_of("a.mp3", b"ID3\x04\x00\x00\x00\x00"),
        FileKind::Audio
    );
    assert_eq!(kind_of("a.mp3", b"\xff\xfb\x90\x64\x00"), FileKind::Audio);
    // A WebP is RIFF too, and stays a picture.
    assert_eq!(
        kind_of("a.webp", b"RIFF\x24\x08\x00\x00WEBPVP8 "),
        FileKind::Image
    );
    // Named like media, written as text: text.
    assert_eq!(kind_of("notes.mp3", b"just words"), FileKind::Text);
}

#[test]
fn media_is_streamed_as_its_own_type() {
    assert_eq!(
        stream_type("clip.mov", b"\x00\x00\x00\x14ftypqt  \x00\x00"),
        "video/quicktime"
    );
    assert_eq!(
        stream_type("clip.webm", b"\x1a\x45\xdf\xa3\x9f\x42"),
        "video/webm"
    );
    assert_eq!(stream_type("a.mp3", b"ID3\x04\x00"), "audio/mpeg");
    assert_eq!(stream_type("shot.png", b"\x89PNG\r\n\x1a\n"), "image/png");
}
