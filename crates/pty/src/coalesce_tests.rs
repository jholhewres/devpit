use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;

/// The frames `run` hands on, and the thread running it.
fn started() -> (
    std::sync::mpsc::SyncSender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    std::thread::JoinHandle<()>,
) {
    let (chunks_tx, chunks) = std::sync::mpsc::sync_channel(16);
    let (frames_tx, frames) = mpsc::channel(16);
    let running = std::thread::spawn(move || run(chunks, frames_tx, Arc::new(Counters::default())));
    (chunks_tx, frames, running)
}

/// What arrives within `within`, all of it.
fn received(frames: &mut mpsc::Receiver<Vec<u8>>, within: Duration) -> Vec<u8> {
    let until = Instant::now() + within;
    let mut all = Vec::new();
    while Instant::now() < until {
        match frames.try_recv() {
            Ok(frame) => all.extend(frame),
            Err(_) => std::thread::sleep(Duration::from_millis(1)),
        }
    }
    all
}

#[test]
fn the_tail_of_a_burst_reaches_the_screen_while_the_program_stays_quiet() {
    let (chunks, mut frames, _running) = started();
    chunks.send(b"working 2s".to_vec()).expect("send");
    // Inside the same frame: before, this waited for a read that never came.
    chunks.send(b"\rdone      ".to_vec()).expect("send");

    // Nothing else is ever said, and the sender stays open.
    assert_eq!(received(&mut frames, FRAME * 10), b"working 2s\rdone      ");
}

#[test]
fn a_chunk_after_a_pause_goes_out_at_once() {
    let (chunks, mut frames, _running) = started();
    chunks.send(b"k".to_vec()).expect("send");
    // A few frames of slack, not half of one: a shared CI machine can take
    // that long to wake a thread. What is guarded is that the chunk is not
    // held for a read that never comes, which no amount of slack would hide.
    assert_eq!(received(&mut frames, FRAME * 4), b"k");
}

#[test]
fn a_flood_is_handed_on_in_order_and_in_few_frames() {
    let (chunks, mut frames, running) = started();
    let mut sent = Vec::new();
    for at in 0..2000u32 {
        let chunk = at.to_string().into_bytes();
        sent.extend_from_slice(&chunk);
        chunks.send(chunk).expect("send");
    }
    drop(chunks);
    let mut all = Vec::new();
    let mut count = 0;
    while let Some(frame) = frames.blocking_recv() {
        all.extend(frame);
        count += 1;
    }
    running.join().expect("run");
    assert_eq!(all, sent);
    assert!(count < 2000, "{count} frames for 2000 chunks");
}

#[test]
fn what_was_read_before_the_reader_went_is_still_handed_on() {
    let (chunks, mut frames, running) = started();
    chunks.send(b"a".to_vec()).expect("send");
    chunks.send(b"b".to_vec()).expect("send");
    drop(chunks);
    running.join().expect("run");
    let mut all = Vec::new();
    while let Ok(frame) = frames.try_recv() {
        all.extend(frame);
    }
    assert_eq!(all, b"ab");
}
