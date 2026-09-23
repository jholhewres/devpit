//! Blocks cut from what real shells wrote, through tmux's pipe-pane.
//!
//! Recorded from bash and zsh started with devpit's own wrappers inside tmux,
//! each running the same four lines typed by `pane.submit`: coloured output,
//! a progress line redrawn with `\r`, a failing command, and a `cd`.

use devpit_pty::blocks::{Block, Segmenter};

fn cut(raw: &[u8]) -> Vec<Block> {
    let mut segmenter = Segmenter::new();
    let mut ended = Vec::new();
    // Fed in small reads, the way a fifo hands them over.
    for chunk in raw.chunks(7) {
        segmenter.feed(chunk, 0, |_| {}, |_, block| ended.extend(block));
    }
    ended
}

fn check(raw: &[u8]) {
    let blocks = cut(raw);
    let lines: Vec<_> = blocks
        .iter()
        .map(|one| one.head.command.clone().unwrap_or_default())
        .collect();
    assert_eq!(
        lines,
        [
            r#"printf "\033[32mgreen\033[0m line\n""#,
            r#"for i in 1 2 3; do printf "\rstep $i"; done; echo"#,
            "ls /nonexistent",
            "cd /usr && pwd",
        ]
    );
    let text = |at: usize| String::from_utf8_lossy(&blocks[at].output).into_owned();
    assert!(
        text(0).contains("\x1b[32mgreen\x1b[0m line"),
        "{:?}",
        text(0)
    );
    assert!(text(1).contains("step 3"), "{:?}", text(1));
    assert_eq!(blocks[2].head.code, Some(2));
    assert!(text(2).contains("No such file"), "{:?}", text(2));
    // zsh ends every output with its PROMPT_SP mark, a `%` it rubs out itself.
    assert!(text(3).starts_with("/usr"), "{:?}", text(3));
    assert_eq!(blocks[0].head.code, Some(0));
    // Nothing of a mark, an envelope or a prompt leaked into any output.
    for at in 0..blocks.len() {
        assert!(!text(at).contains("133;"), "{:?}", text(at));
        assert!(!text(at).contains("tmux;"), "{:?}", text(at));
        assert!(!text(at).contains("devpit-cmd"), "{:?}", text(at));
    }
}

#[test]
fn bash_through_tmux() {
    check(include_bytes!("fixtures/blocks-bash-tmux.raw"));
}

#[test]
fn zsh_through_tmux() {
    check(include_bytes!("fixtures/blocks-zsh-tmux.raw"));
}
