use super::*;

#[test]
fn a_closure_is_named_for_the_command_it_was_written_in() {
    assert_eq!(
        command_of("devpit_desktop::board::board_get::{{closure}}"),
        "board_get"
    );
    assert_eq!(
        command_of("devpit_desktop::git::status_of<str>::{{closure}}::{{closure}}"),
        "status_of"
    );
    let work = || 1;
    assert_eq!(
        command_of(std::any::type_name_of_val(&work)),
        "a_closure_is_named_for_the_command_it_was_written_in"
    );
}

#[test]
fn a_stall_report_names_the_command_working_and_then_answered() {
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let (started, running) = std::sync::mpsc::channel::<()>();
    let call = tauri::async_runtime::spawn(blocking(move || {
        let _ = started.send(());
        let _ = wait.recv();
        Ok(())
    }));
    running.recv().expect("started");
    let name = "a_stall_report_names_the_command_working_and_then_answered";

    let before = said();
    let working = before.split("; answered ").next().unwrap_or_default();
    assert!(working.contains(name), "{before}");

    go.send(()).expect("go");
    tauri::async_runtime::block_on(call)
        .expect("joined")
        .expect("answered");
    let after = said();
    let answered = after.split("; answered ").nth(1).unwrap_or_default();
    assert!(answered.contains(name), "{after}");
}
