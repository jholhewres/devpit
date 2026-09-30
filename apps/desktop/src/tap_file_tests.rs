use super::*;

use std::io::Write;
use std::sync::atomic::AtomicUsize;
use std::sync::{mpsc, Arc};
use std::time::Instant;

fn append(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open to append");
    file.write_all(bytes).expect("append");
}

fn read_all(tail: &mut Tail) -> Vec<u8> {
    let mut buffer = [0u8; 4];
    let mut heard = Vec::new();
    loop {
        match tail.read(&mut buffer).expect("read") {
            0 => return heard,
            read => heard.extend_from_slice(&buffer[..read]),
        }
    }
}

#[test]
fn a_tail_hears_only_what_was_written_since_it_last_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("leaf.log");
    assert_eq!(emptied(&path), Some(0));
    let mut tail = Tail::open(&path, 0).expect("open");

    assert!(read_all(&mut tail).is_empty());
    append(&path, b"first prompt");
    assert_eq!(read_all(&mut tail), b"first prompt");
    append(&path, b", then a command");
    assert_eq!(read_all(&mut tail), b", then a command");
}

#[test]
fn a_copy_opened_anew_is_read_from_its_start() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("leaf.log");
    std::fs::write(&path, b"output from before the arming").expect("write");
    let mut tail = Tail::open(&path, 0).expect("open");
    assert_eq!(read_all(&mut tail).len(), 29);

    // What an arming does: `cat > file` truncates, then the pane writes.
    std::fs::write(&path, b"fresh").expect("rewrite");
    assert_eq!(read_all(&mut tail), b"fresh");
}

#[test]
fn a_copy_is_emptied_and_one_that_will_not_be_is_read_from_its_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("leaf.log");
    std::fs::write(&path, b"already heard").expect("write");
    assert_eq!(emptied(&path), Some(0));
    assert_eq!(std::fs::metadata(&path).expect("meta").len(), 0);

    // A directory stands in for a file psmux will not let go of.
    let held = dir.path().join("held");
    std::fs::create_dir(&held).expect("dir");
    let len = std::fs::metadata(&held).expect("meta").len();
    assert_eq!(emptied(&held), Some(len));
    assert_eq!(emptied(&dir.path().join("missing/leaf.log")), None);
}

#[test]
fn a_follower_asks_for_one_rotation_per_ceiling_and_stops_when_told() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("leaf.log");
    emptied(&path).expect("made");
    let tail = Tail::open(&path, 0).expect("open");
    let listening = Arc::new(AtomicBool::new(true));
    let rotations = Arc::new(AtomicUsize::new(0));
    let (heard, hearing) = mpsc::channel::<Vec<u8>>();

    let following = {
        let listening = Arc::clone(&listening);
        let rotations = Arc::clone(&rotations);
        std::thread::spawn(move || {
            follow(
                tail,
                &listening,
                4,
                |bytes| heard.send(bytes.to_vec()).expect("send"),
                || {
                    rotations.fetch_add(1, Ordering::SeqCst);
                },
            );
        })
    };

    let wait_for = |wanted: usize| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while rotations.load(Ordering::SeqCst) < wanted && Instant::now() < deadline {
            std::thread::sleep(BETWEEN_LOOKS);
        }
        rotations.load(Ordering::SeqCst)
    };

    append(&path, b"past it");
    let chunk = hearing.recv_timeout(Duration::from_secs(5)).expect("heard");
    assert_eq!(chunk, b"past it");
    assert_eq!(wait_for(1), 1);
    // Still past the ceiling and nothing new: not asked again.
    std::thread::sleep(BETWEEN_LOOKS * 4);
    assert_eq!(rotations.load(Ordering::SeqCst), 1);

    // The arming opened it anew; past the ceiling again is another rotation.
    std::fs::write(&path, b"x").expect("rewrite");
    assert_eq!(
        hearing.recv_timeout(Duration::from_secs(5)).expect("heard"),
        b"x"
    );
    append(&path, b"and again");
    assert_eq!(
        hearing.recv_timeout(Duration::from_secs(5)).expect("heard"),
        b"and again"
    );
    assert_eq!(wait_for(2), 2);

    listening.store(false, Ordering::Relaxed);
    following.join().expect("the follower ends");
}
