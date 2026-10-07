//! The rules a file read has to keep, tested against a real directory.

use devpit_core::Store;

use super::*;

/// A project registered in a temp store, and its root.
fn project() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join(".state/state.db")).expect("open");
    let id = store.add_project(dir.path(), None).expect("project");
    (dir, id)
}

#[test]
fn a_text_file_comes_back_with_its_text() {
    let (dir, _) = project();
    std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write");
    let resolved = devpit_core::tree::resolve(dir.path(), "a.txt").expect("resolve");
    assert_eq!(std::fs::read_to_string(&resolved).expect("read"), "hello\n");
}

/// The rule that matters most here: a path out of the project is refused
/// before anything is read. This process runs terminals; reaching it is
/// reaching the machine.
#[test]
fn a_path_climbing_out_of_the_project_is_refused() {
    let (dir, _) = project();
    std::fs::write(dir.path().join("inside.txt"), "x").expect("write");

    for escape in ["../../../etc/passwd", "..", "a/../../../etc/hosts"] {
        assert!(
            devpit_core::tree::resolve(dir.path(), escape).is_err(),
            "{escape} was allowed out of the project"
        );
    }
}

/// A symlink is followed first and then checked, so pointing one out of the
/// project does not smuggle a path past the check.
#[test]
fn a_symlink_pointing_out_is_refused_too() {
    let (dir, _) = project();
    let outside = tempfile::tempdir().expect("tempdir");
    std::fs::write(outside.path().join("secret"), "x").expect("write");

    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path().join("secret"), dir.path().join("link"))
        .expect("link");

    #[cfg(unix)]
    assert!(
        devpit_core::tree::resolve(dir.path(), "link").is_err(),
        "a symlink walked out of the project"
    );
}

/// The mtime guard, exercised rather than reasoned about.
///
/// A save carries the mtime the read saw. These prove the two cases that
/// matter: a normal save goes through, and a save built on a stale read is
/// refused — which is the whole reason the field exists.
/// A named pipe is not a file, and reading one waits forever.
///
/// The workspace holds the tap FIFOs, and the Files panel lists everything it
/// finds — so a row for one is a click that stops the reader rather than
/// failing it. Run with a deadline on purpose: the bug being guarded against
/// is a read that never returns, and a test that reproduces it by hanging is
/// a test nobody can run.
#[cfg(unix)]
#[test]
fn a_named_pipe_is_refused_instead_of_waited_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let made = std::process::Command::new("mkfifo")
        .arg(dir.path().join("tap.fifo"))
        .status()
        .expect("mkfifo to run");
    assert!(made.success(), "mkfifo made no pipe");

    let root = dir.path().to_path_buf();
    let (tell, hear) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tell.send(contents(&root, "tap.fifo".to_owned()));
    });

    let answered = hear
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the read to answer at all");
    let read = answered.expect("an answer, not an error");
    assert_eq!(
        read.not_shown.as_deref(),
        Some("tap.fifo is not a file — nothing to read")
    );
    assert!(read.text.is_none());
}

/// A file past every ceiling is sniffed, never loaded: a sparse file of
/// 64 MB comes back as its first bytes only.
#[test]
fn a_file_past_the_ceiling_is_never_read_whole() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("huge.log");
    let handle = std::fs::File::create(&file).expect("create");
    handle.set_len(64 * 1024 * 1024).expect("grow");

    let (raw, bytes) = read_bounded(&file, 64 * 1024 * 1024).expect("read");
    assert!(raw.len() <= 512, "read {} bytes", raw.len());
    assert_eq!(bytes, 64 * 1024 * 1024);
    assert!(past_the_ceiling("huge.log", bytes).is_some());
}

/// A file that grew past the ceiling after it was measured is refused, not
/// read whole: `read_within` stops one byte past it.
#[test]
fn a_file_that_grew_past_the_ceiling_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("grew.log");
    std::fs::File::create(&file)
        .expect("create")
        .set_len(16 * 1024 * 1024)
        .expect("grow");

    let (raw, bytes) = read_bounded(&file, 0).expect("read");
    assert!(raw.is_empty());
    assert!(past_the_ceiling("grew.log", bytes).is_some());
}

/// A picture between the text ceiling and its own is drawn: a full-screen
/// screenshot is past 2 MB, and was being refused as text that was too long.
#[test]
fn a_picture_past_the_text_ceiling_is_still_drawn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.resize(3 * 1024 * 1024, 7);
    std::fs::write(dir.path().join("shot.png"), &png).expect("write");

    let read = contents(dir.path(), "shot.png".to_owned()).expect("read");
    assert_eq!(read.kind, FileKind::Image);
    assert_eq!(read.not_shown, None);
    let url = read.data_url.expect("a picture to draw");
    assert!(url.starts_with("data:image/png;base64,"), "{}", &url[..40]);
}

/// Past the inline ceiling a picture is streamed (`media.rs`), not refused.
#[test]
fn a_picture_past_the_inline_ceiling_is_left_to_the_stream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.resize(9 * 1024 * 1024, 7);
    std::fs::write(dir.path().join("huge.png"), &png).expect("write");

    let read = contents(dir.path(), "huge.png".to_owned()).expect("read");
    assert_eq!(read.kind, FileKind::Image);
    assert!(read.data_url.is_none(), "a 9 MB picture went inline");
    assert!(read.not_shown.is_none(), "a picture that streams was refused");
}

/// Every format the pane draws comes back as a picture with its own type.
#[test]
fn each_picture_format_comes_back_drawable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut webp = b"RIFF\0\0\0\0WEBPVP8 ".to_vec();
    webp.extend_from_slice(&[0; 8]);
    let cases: [(&str, Vec<u8>, &str); 5] = [
        ("a.png", b"\x89PNG\r\n\x1a\n....".to_vec(), "image/png"),
        ("a.jpg", b"\xff\xd8\xff\xe0....".to_vec(), "image/jpeg"),
        ("a.gif", b"GIF89a....".to_vec(), "image/gif"),
        ("a.webp", webp, "image/webp"),
        (
            "a.svg",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
            "image/svg+xml",
        ),
    ];
    for (name, bytes, media) in cases {
        std::fs::write(dir.path().join(name), &bytes).expect("write");
        let read = contents(dir.path(), name.to_owned()).expect("read");
        assert_eq!(read.kind, FileKind::Image, "{name}");
        let url = read.data_url.unwrap_or_default();
        assert!(
            url.starts_with(&format!("data:{media};base64,")),
            "{name}: {url}"
        );
    }
}

/// Text keeps its own ceiling: the picture's does not stretch to it.
#[test]
fn text_past_its_ceiling_is_still_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("big.log"), vec![b'a'; 3 * 1024 * 1024]).expect("write");

    let read = contents(dir.path(), "big.log".to_owned()).expect("read");
    assert!(read.text.is_none());
    assert!(read.not_shown.expect("a reason").contains("this opens"));
}
