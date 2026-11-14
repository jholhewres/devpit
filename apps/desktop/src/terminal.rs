//! `devpit` typed in a terminal: a menu on screen, picked with the arrow keys
//! and Enter, or a number. A subcommand, or no terminal at all, answers in
//! plain lines instead, so a script reads the same thing every time. What it
//! draws is here; `terminal_run` reads the keys and does what was picked.

use crate::checkup::{Check, Facts};

/// What one start of the binary does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    /// The app, as every start was before the menu.
    Window,
    Menu,
    Run(Item),
    /// `devpit menu` with no terminal to draw it on.
    NoTerminal,
}

/// A line of the menu, and what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    Open,
    Status,
    Pair,
    Web,
    Update,
    Diagnostics,
    Quit,
}

/// The menu, in the order it is drawn and numbered.
pub(crate) const ITEMS: [(Item, &str); 7] = [
    (Item::Open, "Open devpit"),
    (Item::Status, "Status"),
    (Item::Pair, "Pair a device"),
    (Item::Web, "Web version"),
    (Item::Update, "Update"),
    (Item::Diagnostics, "Diagnostics"),
    (Item::Quit, "Quit"),
];

/// The web version is the online one, reached through pairing.
pub(crate) const WEB_LATER: &str = "coming with Remote";

/// The installer, which updates an installed devpit in place.
pub(crate) const INSTALLER: &str = "curl -fsSL https://devpit.app/install.sh | sh";

/// What these arguments start. `terminal`: stdin and stdout are both one.
/// `dev`: a debug build, which `make dev` starts on a terminal and which must
/// open its window there.
pub(crate) fn start(args: &[String], terminal: bool, dev: bool) -> Start {
    let Some(first) = args.first() else {
        return if terminal && !dev {
            Start::Menu
        } else {
            Start::Window
        };
    };
    match first.as_str() {
        "menu" if terminal => Start::Menu,
        "menu" => Start::NoTerminal,
        "open" => Start::Run(Item::Open),
        "status" => Start::Run(Item::Status),
        "pair" => Start::Run(Item::Pair),
        "web" => Start::Run(Item::Web),
        "update" => Start::Run(Item::Update),
        "diagnostics" => Start::Run(Item::Diagnostics),
        // Arguments were never read before: any other still opens the app.
        _ => Start::Window,
    }
}

/// A key, as one read of a terminal in raw mode hands it over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    Up,
    Down,
    Enter,
    Number(usize),
    Quit,
    Other,
}

pub(crate) fn key(bytes: &[u8]) -> Key {
    match bytes {
        [0x1b, b'[' | b'O', b'A'] | [b'k'] => Key::Up,
        [0x1b, b'[' | b'O', b'B'] | [b'j'] => Key::Down,
        [b'\r'] | [b'\n'] => Key::Enter,
        [digit @ b'1'..=b'9'] => Key::Number(usize::from(digit - b'0')),
        // Escape alone, q, Ctrl-C and Ctrl-D.
        [0x1b] | [b'q'] | [0x03] | [0x04] => Key::Quit,
        _ => Key::Other,
    }
}

/// The line selected after `key`, and the item picked by it, if any.
pub(crate) fn pressed(selected: usize, key: Key) -> (usize, Option<Item>) {
    let last = ITEMS.len() - 1;
    match key {
        Key::Up => (selected.checked_sub(1).unwrap_or(last), None),
        Key::Down => (if selected >= last { 0 } else { selected + 1 }, None),
        Key::Enter => (selected, Some(ITEMS[selected.min(last)].0)),
        Key::Number(n) if (1..=ITEMS.len()).contains(&n) => (n - 1, Some(ITEMS[n - 1].0)),
        Key::Quit => (selected, Some(Item::Quit)),
        Key::Number(_) | Key::Other => (selected, None),
    }
}

/// How a terminal can be drawn on, decided the way the installer decides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Look {
    /// The accent's escape, or `None` for no colour at all.
    pub accent: Option<&'static str>,
    pub utf: bool,
    pub cols: usize,
}

impl Look {
    /// No colour, no box characters: what a pipe or a log gets.
    pub(crate) const PLAIN: Look = Look {
        accent: None,
        utf: false,
        cols: 80,
    };

    /// From the environment, for a terminal `cols` wide.
    pub(crate) fn of(var: impl Fn(&str) -> Option<String>, cols: usize) -> Look {
        let set = |name: &str| var(name).filter(|value| !value.is_empty());
        let term = set("TERM").unwrap_or_else(|| "dumb".to_owned());
        if set("CI").is_some() || term == "dumb" {
            return Look::PLAIN;
        }
        let accent = set("NO_COLOR")
            .is_none()
            .then(|| match set("COLORTERM").as_deref() {
                Some("truecolor" | "24bit") => "\x1b[38;2;226;121;91m",
                _ if term.contains("256") => "\x1b[38;5;173m",
                _ => "\x1b[33m",
            });
        let locale = set("LC_ALL")
            .or_else(|| set("LC_CTYPE"))
            .or_else(|| set("LANG"))
            .unwrap_or_default()
            .to_lowercase();
        Look {
            accent,
            utf: locale.contains("utf-8") || locale.contains("utf8"),
            cols: cols.min(100),
        }
    }

    fn paint(&self, style: &str, text: &str) -> String {
        match self.accent {
            Some(_) => format!("{style}{text}\x1b[0m"),
            None => text.to_owned(),
        }
    }

    fn accent(&self, text: &str) -> String {
        self.paint(self.accent.unwrap_or_default(), text)
    }

    fn bold(&self, text: &str) -> String {
        self.paint("\x1b[1m", text)
    }

    pub(crate) fn dim(&self, text: &str) -> String {
        self.paint("\x1b[2m", text)
    }
}

/// The installer's banner, with the version where it says what it installs.
pub(crate) fn banner(look: &Look) -> String {
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    if !look.utf || look.cols < 66 {
        return format!(
            "\n  {}  {}\n\n",
            look.accent(&look.bold("devpit")),
            look.dim(&version)
        );
    }
    let art = [
        "    ██                          ▄▄    ██",
        "▄▄▄▄██  ▄▄▄▄▄▄  ▄▄  ▄▄  ▄▄▄▄▄▄  ▄▄  ▄▄██▄▄",
        "██  ██  ██▄▄██  ██  ██  ██  ██  ██    ██  ",
        "██▄▄██  ██▄▄▄▄   ▀██▀   ██▄▄██  ██    ██▄▄",
        "                        ██",
    ];
    let mut drawn = String::from("\n");
    for (at, line) in art.iter().enumerate() {
        let beside = match at {
            1 => format!("    {}", look.bold("devpit")),
            2 => format!("    {}", look.dim(&version)),
            _ => String::new(),
        };
        drawn.push_str(&format!("  {}{beside}\n", look.accent(line)));
    }
    drawn.push('\n');
    drawn
}

/// The menu's lines, `selected` marked, and the keys under them.
pub(crate) fn menu(look: &Look, selected: usize) -> String {
    let mut drawn = String::new();
    for (at, (item, label)) in ITEMS.iter().enumerate() {
        let number = at + 1;
        let later = if *item == Item::Web {
            format!("  {}", look.dim(WEB_LATER))
        } else {
            String::new()
        };
        if at == selected {
            let mark = if look.utf { "›" } else { ">" };
            drawn.push_str(&format!(
                "  {} {number}  {}{later}\n",
                look.accent(mark),
                look.bold(label)
            ));
        } else {
            drawn.push_str(&format!("    {number}  {label}{later}\n"));
        }
    }
    let keys = if look.utf {
        format!("↑↓ choose · Enter pick · 1–{} · q quit", ITEMS.len())
    } else {
        format!("up/down choose, enter pick, 1-{}, q quit", ITEMS.len())
    };
    drawn.push_str(&format!("\n  {}\n", look.dim(&keys)));
    drawn
}

/// `devpit status`: one `name: value` a line.
pub(crate) fn status_lines(facts: &Facts) -> String {
    let (running, waiting) = match facts.sessions {
        Some(sessions) => (sessions.running.to_string(), sessions.waiting.to_string()),
        None => ("devpit is not open".to_owned(), "-".to_owned()),
    };
    let disk = facts.disk.map_or("unknown".to_owned(), |disk| {
        format!(
            "{} free of {}",
            crate::checkup::size(disk.free),
            crate::checkup::size(disk.total)
        )
    });
    let load = facts.load.map_or("unknown".to_owned(), |load| {
        format!("{load:.2} on {} CPUs", facts.cpus)
    });
    [
        ("version", env!("CARGO_PKG_VERSION").to_owned()),
        ("sessions running", running),
        ("waiting on you", waiting),
        ("disk", disk),
        ("load", load),
    ]
    .iter()
    .map(|(name, value)| format!("{name}: {value}\n"))
    .collect()
}

/// `devpit diagnostics`: each check, and its fix under one that failed.
pub(crate) fn check_lines(look: &Look, checks: &[Check]) -> String {
    let mut drawn = String::new();
    for check in checks {
        let mark = match (check.ok, look.utf) {
            (true, true) => look.paint("\x1b[32m", "✓"),
            (false, true) => look.paint("\x1b[31m", "✗"),
            (true, false) => "ok  ".to_owned(),
            (false, false) => "FAIL".to_owned(),
        };
        drawn.push_str(&format!("{mark} {}: {}\n", check.name, check.detail));
        if let Some(fix) = &check.fix {
            let pad = if look.utf { "  " } else { "     " };
            drawn.push_str(&format!("{pad}fix: {fix}\n"));
        }
    }
    drawn
}

/// `text` moved right by `pad`, line by line: the menu's margin.
pub(crate) fn indented(text: &str, pad: &str) -> String {
    text.lines().map(|line| format!("{pad}{line}\n")).collect()
}

/// What the app's door said, in words for the person at the terminal.
pub(crate) fn told(why: &str) -> String {
    if why == devpit_agentapi::client::closed() {
        "devpit is not open: run `devpit open` first".to_owned()
    } else if why == "turn Remote on first" {
        "turn Remote on first, in Settings → Remote".to_owned()
    } else {
        why.to_owned()
    }
}

/// A pairing, as the terminal shows it: the code, the QR code to scan when
/// there is a screen to draw it on, and the link.
pub(crate) fn pairing_lines(code: &str, url: &str, minutes: u64, qr: bool) -> String {
    let mut drawn = format!("code: {code}  (good for {minutes} minutes)\n");
    if qr {
        if let Ok(symbol) = qrcode::QrCode::new(url.as_bytes()) {
            // Light on dark, which is what most terminals are.
            drawn.push_str(
                &symbol
                    .render::<qrcode::render::unicode::Dense1x2>()
                    .dark_color(qrcode::render::unicode::Dense1x2::Light)
                    .light_color(qrcode::render::unicode::Dense1x2::Dark)
                    .quiet_zone(true)
                    .build(),
            );
            drawn.push('\n');
        }
    }
    drawn.push_str(&format!("link: {url}\n"));
    drawn.push_str("The device then waits for you to allow it on devpit's screen.\n");
    drawn
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
