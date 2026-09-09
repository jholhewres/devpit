//! The session's tests, kept beside it.

use super::*;
use std::sync::atomic::Ordering;

fn size() -> PtySize {
    PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }
}

#[tokio::test]
async fn a_command_output_arrives_as_frames() {
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("devpit");

    let mut session = spawn(cmd, size()).expect("spawn");

    let mut seen = Vec::new();
    while let Some(frame) = session.frames.recv().await {
        seen.extend_from_slice(&frame);
    }

    assert!(
        String::from_utf8_lossy(&seen).contains("devpit"),
        "output did not reach the frames: {:?}",
        String::from_utf8_lossy(&seen)
    );
    assert!(session.counters.bytes.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
async fn bytes_written_to_the_pty_come_back_in_frames() {
    let cmd = CommandBuilder::new("cat");
    let mut session = spawn(cmd, size()).expect("spawn");

    session.write(b"devpit-stdin\n").expect("write");
    // Closing stdin is what makes `cat` exit. Dropping the writer is that close.
    let io = session.take_io().expect("io");
    drop(io.writer);

    let mut seen = Vec::new();
    while let Some(frame) = session.frames.recv().await {
        seen.extend_from_slice(&frame);
    }
    assert!(
        String::from_utf8_lossy(&seen).contains("devpit-stdin"),
        "stdin did not echo: {:?}",
        String::from_utf8_lossy(&seen)
    );
}

/// The point of coalescing: a flood must not become one message per read.
#[tokio::test]
async fn a_flood_is_coalesced_into_far_fewer_frames_than_reads() {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("yes devpit | head -c 2000000");

    let mut session = spawn(cmd, size()).expect("spawn");

    let mut bytes = 0usize;
    while let Some(frame) = session.frames.recv().await {
        bytes += frame.len();
    }

    let frames = session.counters.frames.load(Ordering::Relaxed);
    assert!(bytes >= 2_000_000, "only {bytes} bytes arrived");
    assert!(
        frames < 400,
        "{frames} frames for {bytes} bytes — coalescing is not working"
    );
}
