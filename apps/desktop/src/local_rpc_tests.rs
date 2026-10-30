use super::*;

#[test]
fn only_a_plain_command_name_reaches_the_handler() {
    for command in ["project_list", "chat_history", "board_get2"] {
        assert!(plain_command(command), "{command}");
    }
    for command in [
        "",
        "plugin:dialog|open",
        "../x",
        "Project_list",
        "a b",
        "__TAURI_INVOKE_KEY__",
        &"x".repeat(129),
    ] {
        assert!(!plain_command(command), "{command}");
    }
}

#[test]
fn the_browser_gets_the_commands_answer_or_its_error_and_nothing_else() {
    let ok = InvokeResponse::Ok(InvokeResponseBody::Json("[1,2]".to_owned()));
    assert_eq!(answered(ok), ("200 OK", b"[1,2]".to_vec()));
    let failed = InvokeResponse::Err(tauri::ipc::InvokeError(
        serde_json::json!({ "message": "no" }),
    ));
    let (status, body) = answered(failed);
    assert_eq!(status, "500 Internal Server Error");
    assert_eq!(
        String::from_utf8(body).expect("utf8"),
        r#"{"message":"no"}"#
    );
}

/// The call is made from the app's own origin, so the handler treats it as
/// the window's and its access rules are the window's.
#[test]
fn the_call_is_made_from_the_apps_own_url() {
    let url = app_url();
    if cfg!(any(windows, target_os = "android")) {
        assert_eq!(url.as_str(), "http://tauri.localhost/");
    } else {
        assert_eq!(url.scheme(), "tauri");
        assert_eq!(url.host_str(), Some("localhost"));
    }
}

/// The invoke key is the app's own and never travels: nothing the browser
/// sends is read as a key, and no reply carries one.
#[test]
fn the_invoke_key_never_leaves_rust() {
    let source = include_str!("local_rpc.rs");
    let set_from = source
        .lines()
        .filter(|line| line.contains("invoke_key:"))
        .collect::<Vec<_>>();
    assert_eq!(
        set_from,
        ["            invoke_key: app.invoke_key().to_owned(),"]
    );
    for file in [
        include_str!("local_server.rs"),
        include_str!("local_events.rs"),
        include_str!("local_files.rs"),
    ] {
        assert!(!file.contains("invoke_key"), "a key outside local_rpc");
    }
}

/// The API this rests on is Tauri's `unstable` one: if `on_message`, the
/// request or the responder change shape, this stops compiling.
#[allow(dead_code)]
fn the_handler_api_keeps_its_shape(webview: tauri::Webview, app: &tauri::AppHandle) {
    let request = InvokeRequest {
        cmd: String::new(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: app_url(),
        body: InvokeBody::Json(serde_json::Value::Null),
        headers: Default::default(),
        invoke_key: app.invoke_key().to_owned(),
    };
    let responder: Box<tauri::ipc::OwnedInvokeResponder<tauri::Wry>> = Box::new(
        |_webview,
         _command: String,
         _response: InvokeResponse,
         _ok: CallbackFn,
         _err: CallbackFn| {},
    );
    webview.on_message(request, responder);
}

#[test]
fn tauri_is_held_to_the_version_the_handler_was_written_against() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest
            .lines()
            .any(|line| line.starts_with("tauri = { version = \"=2.11.5\"")),
        "tauri is pinned in apps/desktop/Cargo.toml: re-check local_rpc before moving it"
    );
}
