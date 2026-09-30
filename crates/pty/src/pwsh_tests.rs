use super::*;

/// The standard decoding, written out, so the test does not trust the
/// encoder to check itself.
fn decoded(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        other => panic!("not base64: {other}"),
    };
    let mut out = Vec::new();
    for quad in text.as_bytes().chunks(4) {
        let kept: Vec<u8> = quad.iter().copied().filter(|c| *c != b'=').collect();
        let bits = kept
            .iter()
            .fold(0u32, |acc, c| (acc << 6) | u32::from(value(*c)))
            << (6 * (4 - kept.len()));
        out.extend(&bits.to_be_bytes()[1..kept.len()]);
    }
    out
}

#[test]
fn base64_pads_like_everyone_else() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn the_encoded_command_is_the_script_in_utf16() {
    let script = "Write-Host 'olá — devpit'";
    let bytes = decoded(&encoded(script));
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    assert_eq!(String::from_utf16(&units).expect("utf-16"), script);
}

#[test]
fn powershell_is_known_however_it_is_spelled() {
    for shell in [
        "pwsh",
        "pwsh.exe",
        "PowerShell.exe",
        r"C:\Program Files\PowerShell\7\pwsh.exe",
        "/usr/bin/pwsh",
    ] {
        assert!(is_powershell(shell), "{shell}");
    }
    for shell in ["bash", "cmd.exe", "pwsh-preview", "zsh"] {
        assert!(!is_powershell(shell), "{shell}");
    }
}

#[test]
fn powershell_7_is_preferred_where_it_is_installed() {
    assert_eq!(chosen(|name| name == "pwsh.exe"), "pwsh.exe");
    assert_eq!(chosen(|_| false), "powershell.exe");
}

#[test]
fn powershell_is_wrapped_only_when_the_marks_are_asked_for() {
    assert!(launch("pwsh.exe", &[]).is_none());
    assert!(launch("pwsh.exe", &[Feature::Ready]).is_none());
    assert!(launch("bash", &[Feature::Marks]).is_none());

    let wrapped = launch("pwsh.exe", &[Feature::Marks]).expect("wrapped");
    assert_eq!(wrapped.program, "pwsh.exe");
    assert_eq!(wrapped.args[..3], ["-NoLogo", "-NoExit", "-EncodedCommand"]);
    assert_eq!(wrapped.args[3], encoded(PROMPT));
    // Nothing on the line a shell or psmux could read as quoting.
    assert!(wrapped.args[3]
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')));
}

/// That the marks come out of a real PowerShell was checked by running this
/// script under a pty and reading them; this pins the pieces it has to carry.
#[test]
fn the_prompt_speaks_the_marks_the_scanner_reads() {
    for mark in [
        "'133;A'",
        "'133;B'",
        "'133;C'",
        "\"133;D;$status\"",
        "\"7;file://",
        "\"777;devpit-cmd;",
    ] {
        assert!(PROMPT.contains(mark), "{mark}");
    }
    // Chained, never replaced: theirs still draws the prompt.
    assert!(PROMPT.contains("$global:__devpit_their_prompt = $function:prompt"));
    assert!(PROMPT.contains("$global:__devpit_their_readline = $function:PSConsoleHostReadLine"));
    // The PATH entry and the requests are consumed, not inherited.
    assert!(PROMPT.contains("'DEVPIT_BIN'"));
    // Windows PowerShell 5.1 has no `e escape.
    assert!(!PROMPT.contains("`e"));
}
