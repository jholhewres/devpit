use crate::checkup::{Check, Disk, Facts, Sessions};

use super::*;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

#[test]
fn devpit_on_a_terminal_shows_the_menu() {
    assert_eq!(start(&[], true, false), Start::Menu);
}

#[test]
fn with_no_terminal_devpit_opens_the_app_as_before() {
    assert_eq!(start(&[], false, false), Start::Window);
}

#[test]
fn a_subcommand_runs_its_item_in_plain_lines_without_the_menu() {
    for (word, item) in [
        ("open", Item::Open),
        ("status", Item::Status),
        ("pair", Item::Pair),
        ("web", Item::Web),
        ("update", Item::Update),
        ("diagnostics", Item::Diagnostics),
    ] {
        assert_eq!(start(&args(&[word]), true, false), Start::Run(item));
        assert_eq!(start(&args(&[word]), false, false), Start::Run(item));
    }
}

#[test]
fn arguments_devpit_never_read_still_open_the_app() {
    assert_eq!(start(&args(&["--minimized"]), true, false), Start::Window);
    assert_eq!(start(&args(&["quit"]), false, false), Start::Window);
}

#[test]
fn a_debug_build_opens_its_window_on_a_terminal_and_shows_the_menu_when_asked() {
    assert_eq!(start(&[], true, true), Start::Window);
    assert_eq!(start(&args(&["menu"]), true, true), Start::Menu);
    assert_eq!(start(&args(&["menu"]), false, false), Start::NoTerminal);
}

#[test]
fn arrows_move_round_the_menu_and_enter_picks_the_line() {
    assert_eq!(key(b"\x1b[A"), Key::Up);
    assert_eq!(key(b"\x1bOB"), Key::Down);
    assert_eq!(pressed(0, Key::Up), (6, None));
    assert_eq!(pressed(6, key(b"\x1b[B")), (0, None));
    assert_eq!(pressed(1, Key::Down), (2, None));
    assert_eq!(pressed(2, key(b"\r")), (2, Some(Item::Pair)));
    assert_eq!(pressed(0, key(b"\n")), (0, Some(Item::Open)));
}

#[test]
fn a_number_picks_its_item_at_once() {
    assert_eq!(pressed(0, key(b"5")), (4, Some(Item::Update)));
    assert_eq!(pressed(3, key(b"2")), (1, Some(Item::Status)));
    assert_eq!(pressed(3, key(b"9")), (3, None));
}

#[test]
fn q_escape_ctrl_c_and_ctrl_d_quit() {
    for bytes in [&b"q"[..], &[0x1b], &[0x03], &[0x04]] {
        assert_eq!(pressed(2, key(bytes)), (2, Some(Item::Quit)));
    }
    assert_eq!(pressed(2, key(b"x")), (2, None));
}

#[test]
fn the_menu_lists_its_items_in_order_and_web_waits_for_remote() {
    let drawn = menu(&Look::PLAIN, 0);
    let lines: Vec<&str> = drawn.lines().collect();
    assert_eq!(
        &lines[..7],
        &[
            "  > 1  Open devpit",
            "    2  Status",
            "    3  Pair a device",
            "    4  Web version  coming with Remote",
            "    5  Update",
            "    6  Diagnostics",
            "    7  Quit",
        ]
    );
    assert!(menu(&Look::PLAIN, 4).contains("  > 5  Update\n"));
}

#[test]
fn plain_output_carries_no_escape_codes() {
    let check = Check {
        name: "devpit is open",
        ok: false,
        detail: "it does not answer".to_owned(),
        fix: Some("run `devpit open`".to_owned()),
    };
    for drawn in [
        banner(&Look::PLAIN),
        menu(&Look::PLAIN, 3),
        check_lines(&Look::PLAIN, &[check]),
    ] {
        assert!(!drawn.contains('\x1b'), "{drawn:?}");
    }
}

#[test]
fn colour_follows_the_installers_rules() {
    let truecolor = Look::of(
        env(&[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LANG", "en_US.UTF-8"),
        ]),
        120,
    );
    assert_eq!(truecolor.accent, Some("\x1b[38;2;226;121;91m"));
    assert!(truecolor.utf);
    assert_eq!(truecolor.cols, 100);
    let no_color = Look::of(env(&[("TERM", "xterm-256color"), ("NO_COLOR", "1")]), 80);
    assert_eq!(no_color.accent, None);
    assert_eq!(Look::of(env(&[("TERM", "dumb")]), 80), Look::PLAIN);
    assert_eq!(
        Look::of(env(&[("TERM", "xterm"), ("CI", "true")]), 80),
        Look::PLAIN
    );
}

#[test]
fn the_banner_is_the_installers_on_a_wide_utf_terminal() {
    let wide = Look {
        accent: None,
        utf: true,
        cols: 80,
    };
    let drawn = banner(&wide);
    let installer = include_str!("../../../install.sh");
    for art in [
        "▄▄▄▄██  ▄▄▄▄▄▄  ▄▄  ▄▄  ▄▄▄▄▄▄  ▄▄  ▄▄██▄▄",
        "██▄▄██  ██▄▄▄▄   ▀██▀   ██▄▄██  ██    ██▄▄",
    ] {
        assert!(drawn.contains(art) && installer.contains(art), "{art}");
    }
    assert!(drawn.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    let narrow = banner(&Look { cols: 60, ..wide });
    assert_eq!(
        narrow.trim(),
        format!("devpit  v{}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn status_says_the_version_the_sessions_and_the_machine() {
    let facts = Facts {
        sessions: Some(Sessions {
            running: 3,
            waiting: 1,
        }),
        disk: Some(Disk {
            free: 25_000_000_000,
            total: 465_000_000_000,
        }),
        load: Some(15.59),
        cpus: 16,
        ..Facts::default()
    };
    assert_eq!(
        status_lines(&facts),
        format!(
            "version: {}\nsessions running: 3\nwaiting on you: 1\ndisk: 25.0 GB free of 465.0 GB\nload: 15.59 on 16 CPUs\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn status_says_when_devpit_is_not_open() {
    let drawn = status_lines(&Facts::default());
    assert!(drawn.contains("sessions running: devpit is not open\n"));
    assert!(drawn.contains("disk: unknown\n"));
}

#[test]
fn diagnostics_prints_each_check_and_the_fix_under_one_that_failed() {
    let checks = [
        Check {
            name: "devpit is open",
            ok: false,
            detail: "it does not answer".to_owned(),
            fix: Some("run `devpit open`".to_owned()),
        },
        Check {
            name: "tmux is installed",
            ok: true,
            detail: "/usr/bin/tmux".to_owned(),
            fix: None,
        },
    ];
    assert_eq!(
        check_lines(&Look::PLAIN, &checks),
        "FAIL devpit is open: it does not answer\n     fix: run `devpit open`\nok   tmux is installed: /usr/bin/tmux\n"
    );
}

#[test]
fn pairing_shows_the_code_the_qr_code_and_the_link() {
    let url = "https://devpit.example/#pair=ABCD2345";
    let drawn = pairing_lines("ABCD2345", url, 10, true);
    assert!(drawn.starts_with("code: ABCD2345  (good for 10 minutes)\n"));
    assert!(drawn.contains(&format!("link: {url}\n")));
    assert!(drawn.contains('█') || drawn.contains('▀'), "{drawn}");
    let piped = pairing_lines("ABCD2345", url, 10, false);
    assert!(!piped.contains('█') && !piped.contains('▀'));
}

#[test]
fn update_runs_the_installer_the_way_its_own_header_says() {
    let installer = include_str!("../../../install.sh");
    assert!(installer.contains(&format!("#     {INSTALLER}\n")));
}

#[test]
fn what_the_door_says_is_told_to_the_person_at_the_terminal() {
    assert_eq!(
        told(&devpit_agentapi::client::closed()),
        "devpit is not open: run `devpit open` first"
    );
    assert_eq!(
        told("turn Remote on first"),
        "turn Remote on first, in Settings → Remote"
    );
    assert_eq!(
        told("no pairing code could be made"),
        "no pairing code could be made"
    );
}

#[test]
fn an_items_output_keeps_the_menus_margin() {
    assert_eq!(
        indented("version: 1\nload: 2\n", "  "),
        "  version: 1\n  load: 2\n"
    );
    assert_eq!(indented("plain\n", ""), "plain\n");
}

#[test]
fn a_devpit_started_in_the_background_leaves_the_terminal_and_opens_the_app() {
    assert!(owns_terminal(true, true, Some(4242), 4242));
    assert!(!owns_terminal(true, true, Some(4242), 4300), "`devpit &`");
    assert!(!owns_terminal(true, true, None, 4242));
    assert!(!owns_terminal(true, false, Some(4242), 4242));
    assert_eq!(
        start(&[], owns_terminal(true, true, Some(1), 2), false),
        Start::Window
    );
}
