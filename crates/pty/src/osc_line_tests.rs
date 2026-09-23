//! devpit's own OSC 777 lines, apart from the standard sequences.

use super::*;

#[test]
fn devpits_own_hook_says_which_line_ran() {
    let mut scanner = Scanner::new();
    let mut heard = Vec::new();
    scanner.scan(b"\x1b]777;devpit-cmd;git status --short\x07", |told| {
        heard.push(told)
    });
    assert_eq!(heard, [Told::Command("git status --short".to_owned())]);
    // Another program's 777 is not ours.
    let mut other = Vec::new();
    scanner.scan(b"\x1b]777;notify;title;body\x07", |told| other.push(told));
    assert!(other.is_empty());
}
