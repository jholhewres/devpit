//! A disk slow to sync, and the store's other connections.
//!
//! On Linux a sync waits for the filesystem's journal, and a build writing
//! beside devpit can stretch that to seconds. A sync made while holding a lock
//! another connection waits on becomes that connection's wait, and past
//! `busy_timeout` its "database is locked" — reported from 0.1.38, when reading
//! the live sessions began to write.
//!
//! The disk here is SQLite's own with its next sync stalled past what a store
//! waits, so the race is decided by the code rather than by the machine.

use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Mutex, MutexGuard, Once, OnceLock};
use std::time::Duration;

use devpit_core::store::SessionSeen;
use devpit_core::Store;
use rusqlite::ffi;

/// Longer than a store waits for a lock.
const STALL: Duration = Duration::from_millis(5_500);

static ARMED: AtomicBool = AtomicBool::new(false);
static STALLED: AtomicBool = AtomicBool::new(false);

type SyncFn = unsafe extern "C" fn(*mut ffi::sqlite3_file, c_int) -> c_int;

/// The platform's VFS, which does the work.
static REAL: OnceLock<usize> = OnceLock::new();
/// For each kind of file — the WAL's methods are not the database's — its own
/// methods, the copy that stalls, and the sync that copy calls on to.
static METHODS: Mutex<Vec<(usize, usize, SyncFn)>> = Mutex::new(Vec::new());

unsafe extern "C" fn sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    if ARMED.swap(false, SeqCst) {
        STALLED.store(true, SeqCst);
        std::thread::sleep(STALL);
    }
    let ours = unsafe { (*file).pMethods } as usize;
    let real = METHODS
        .lock()
        .expect("methods")
        .iter()
        .find(|(_, copy, _)| *copy == ours)
        .map(|(_, _, real)| *real)
        .expect("a file opened here");
    unsafe { real(file, flags) }
}

unsafe extern "C" fn open(
    _: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out: *mut c_int,
) -> c_int {
    let real = *REAL.get().expect("installed") as *mut ffi::sqlite3_vfs;
    unsafe {
        let opened = (*real).xOpen.expect("xOpen")(real, name, file, flags, out);
        let theirs = (*file).pMethods;
        if opened == ffi::SQLITE_OK && !theirs.is_null() {
            let mut known = METHODS.lock().expect("methods");
            let ours = match known.iter().find(|(own, _, _)| *own == theirs as usize) {
                Some((_, ours, _)) => *ours,
                None => {
                    let mut ours = *theirs;
                    let real = ours.xSync.expect("xSync");
                    ours.xSync = Some(sync);
                    let ours = Box::into_raw(Box::new(ours)) as usize;
                    known.push((theirs as usize, ours, real));
                    ours
                }
            };
            (*file).pMethods = ours as *const ffi::sqlite3_io_methods;
        }
        opened
    }
}

/// Makes the slow disk this binary's default, before any store opens.
fn slow_disk() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| unsafe {
        let real = ffi::sqlite3_vfs_find(std::ptr::null());
        REAL.set(real as usize).expect("once");
        let mut slow = *real;
        slow.zName = c"slow-disk".as_ptr();
        slow.xOpen = Some(open);
        let registered = ffi::sqlite3_vfs_register(Box::into_raw(Box::new(slow)), 1);
        assert_eq!(registered, ffi::SQLITE_OK);
    });
}

/// One at a time: the stall belongs to the whole binary.
fn alone() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs `stuck` with the next sync stalled, and `meanwhile` once that sync is
/// stalled — or once `stuck` is done, having made none.
fn while_stuck<T>(stuck: impl FnOnce() + Send + 'static, meanwhile: impl FnOnce() -> T) -> T {
    STALLED.store(false, SeqCst);
    ARMED.store(true, SeqCst);
    let stuck = std::thread::spawn(stuck);
    while !STALLED.load(SeqCst) && !stuck.is_finished() {
        std::thread::sleep(Duration::from_millis(5));
    }
    ARMED.store(false, SeqCst);
    let said = meanwhile();
    stuck.join().expect("the stuck side");
    said
}

fn running(id: &str) -> [SessionSeen<'_>; 1] {
    [SessionSeen {
        session_id: id,
        name: id,
        cwd: "/w",
        project_id: None,
        card_id: None,
        status: "busy",
    }]
}

#[test]
fn a_write_stuck_on_the_disk_refuses_no_other_write() {
    let _alone = alone();
    slow_disk();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");
    drop(Store::open(&path).expect("open"));

    let writing = path.clone();
    let other = while_stuck(
        move || {
            let store = Store::open(&writing).expect("open");
            store
                .saw_sessions("claude", &running("s1"), 1)
                .expect("write");
        },
        || {
            let store = Store::open(&path).expect("open");
            store.saw_sessions("codex", &running("s2"), 1)
        },
    );
    assert!(other.is_ok(), "a write waited on another's sync: {other:?}");
}

/// What every command does first is open the store.
#[test]
fn a_close_stuck_on_the_disk_refuses_no_open() {
    let _alone = alone();
    slow_disk();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");
    let store = Store::open(&path).expect("open");
    store
        .saw_sessions("claude", &running("s1"), 1)
        .expect("write");

    let opened = while_stuck(move || drop(store), || Store::open(&path).map(drop));
    assert!(
        opened.is_ok(),
        "opening waited on another's close: {opened:?}"
    );
}

/// The automatic checkpoint copies the whole WAL into the database, and the
/// write after it starts the WAL over: SQLite syncs the new header under the
/// write lock, however `synchronous` is set. Reported from 0.1.42, with polls
/// of the live sessions rewriting what they had kept making most of the WAL.
#[test]
fn a_write_starting_the_wal_over_refuses_no_poll() {
    let _alone = alone();
    slow_disk();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");
    // Another connection open, as the app always has one: the WAL's index
    // then remembers the checkpoint, rather than being rebuilt by the next open.
    let open = Store::open(&path).expect("open");
    open.saw_sessions("claude", &running("s1"), 1)
        .expect("write");
    open.saw_sessions("codex", &running("s2"), 1)
        .expect("write");
    let copied: i64 = open
        .conn()
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| row.get(0))
        .expect("checkpoint");
    assert_eq!(copied, 0, "the checkpoint was refused");

    let writing = path.clone();
    let polled = while_stuck(
        move || {
            let [one] = running("s1");
            let [started] = running("s3");
            let store = Store::open(&writing).expect("open");
            store
                .saw_sessions("claude", &[one, started], 2)
                .expect("write");
        },
        || {
            let store = Store::open(&path).expect("open");
            store.saw_sessions("codex", &running("s2"), 2)
        },
    );
    drop(open);
    assert!(STALLED.load(SeqCst), "the new WAL's header was not synced");
    assert!(
        polled.is_ok(),
        "a poll waited on the WAL starting over: {polled:?}"
    );
}
