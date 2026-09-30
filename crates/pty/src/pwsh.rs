//! PowerShell, the shell a Windows terminal starts, and the prompt that makes
//! it speak.
//!
//! The same marks the bash and zsh startup files emit (see [`crate::shell`]),
//! from a `prompt` that wraps the person's own and a `PSConsoleHostReadLine`
//! that says when a line starts running. Handed over as `-EncodedCommand`,
//! which runs after their profile, needs no file on disk, is not held to the
//! execution policy a script file is, and is plain base64 on the command line —
//! nothing for tmux, psmux or ConPTY to quote wrongly.

use crate::shell::{Feature, Launch};

/// The prompt wrapper, run once as the shell starts.
pub(crate) const PROMPT: &str = include_str!("shell/prompt.ps1");

/// The shell a Windows terminal starts: PowerShell 7 where it is on PATH,
/// the Windows PowerShell every Windows has otherwise.
#[cfg(windows)]
pub(crate) fn preferred() -> String {
    let on_path = |name: &str| {
        std::env::var_os("PATH")
            .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
    };
    chosen(on_path).to_owned()
}

pub(crate) fn chosen(on_path: impl Fn(&str) -> bool) -> &'static str {
    if on_path("pwsh.exe") {
        "pwsh.exe"
    } else {
        "powershell.exe"
    }
}

/// `pwsh`, `powershell.exe`, `C:\…\pwsh.exe`: either PowerShell, however
/// it is spelled.
pub(crate) fn is_powershell(shell: &str) -> bool {
    let leaf = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase();
    matches!(
        leaf.strip_suffix(".exe").unwrap_or(&leaf),
        "pwsh" | "powershell"
    )
}

/// The launch that installs the prompt, or `None` for a shell that is not
/// PowerShell or a request without the marks.
pub(crate) fn launch(shell: &str, features: &[Feature]) -> Option<Launch> {
    if !is_powershell(shell) || !features.contains(&Feature::Marks) {
        return None;
    }
    Some(Launch {
        program: shell.to_owned(),
        args: vec![
            "-NoLogo".to_owned(),
            "-NoExit".to_owned(),
            "-EncodedCommand".to_owned(),
            encoded(PROMPT),
        ],
        env: Vec::new(),
    })
}

/// What `-EncodedCommand` takes: the script's UTF-16LE bytes, in base64.
pub(crate) fn encoded(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64(&bytes)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let at = |i: usize| u32::from(chunk.get(i).copied().unwrap_or(0));
        let triple = (at(0) << 16) | (at(1) << 8) | at(2);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(triple >> (18 - 6 * i)) as usize & 63]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "pwsh_tests.rs"]
mod tests;
