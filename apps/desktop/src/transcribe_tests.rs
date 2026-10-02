use super::{multipart, recording, spoken};

#[test]
fn only_a_recording_the_composer_kept_is_transcribed() {
    assert!(recording("voice-01k2.webm"));
    assert!(recording("voice-01k2.m4a"));
    assert!(!recording("pasted-01k2.png"));
    assert!(!recording("voice-../../etc/passwd.wav"));
    assert!(!recording("voice-a/b.webm"));
    assert!(!recording("voice-01k2.exe"));
}

#[test]
fn a_language_is_a_short_code() {
    assert!(spoken("pt") && spoken("en") && spoken("yue"));
    assert!(!spoken("pt-BR") && !spoken("") && !spoken("PT") && !spoken("english"));
}

#[test]
fn the_form_carries_the_file_the_model_and_the_language() {
    let (boundary, body) = multipart(
        "voice-1.webm",
        b"OPUS",
        "gpt-4o-mini-transcribe",
        Some("pt"),
    );
    let body = String::from_utf8(body).expect("utf8");
    assert!(body.contains("name=\"model\"\r\n\r\ngpt-4o-mini-transcribe\r\n"));
    assert!(body.contains("name=\"language\"\r\n\r\npt\r\n"));
    assert!(body.contains("filename=\"voice-1.webm\"\r\ncontent-type: audio/webm\r\n\r\nOPUS\r\n"));
    assert!(body.ends_with(&format!("--{boundary}--\r\n")));
    let (_, plain) = multipart("voice-1.ogg", b"x", "whisper-1", None);
    assert!(!String::from_utf8(plain)
        .expect("utf8")
        .contains("name=\"language\""));
}
