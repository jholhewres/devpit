use super::*;

#[test]
fn a_width_within_the_limits_is_kept() {
    assert_eq!(
        Widths {
            sidebar: 300,
            files: 420
        }
        .held(),
        Widths {
            sidebar: 300,
            files: 420
        }
    );
}

#[test]
fn a_width_from_somewhere_else_cannot_make_a_panel_nobody_can_drag_back() {
    // An older build with other limits, or somebody editing the row by hand.
    let silly = Widths {
        sidebar: 0,
        files: 99_999,
    }
    .held();
    assert_eq!(silly.sidebar, LEAST.sidebar);
    assert_eq!(silly.files, MOST.files);
}

#[test]
fn a_preference_nobody_can_parse_reads_as_the_width_we_ship() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("open");
    store
        .set_preference(preference::SIDEBAR_WIDTH, "not a number")
        .expect("write");
    assert_eq!(
        read(&store, preference::SIDEBAR_WIDTH, WIDE.sidebar),
        WIDE.sidebar
    );
}

#[test]
fn a_width_survives_closing_the_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");
    let store = devpit_core::Store::open(&path).expect("open");
    store
        .set_preference(preference::FILES_WIDTH, "420")
        .expect("write");
    drop(store);

    let store = devpit_core::Store::open(&path).expect("reopen");
    assert_eq!(read(&store, preference::FILES_WIDTH, WIDE.files), 420);
}
