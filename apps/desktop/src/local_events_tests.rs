use super::*;

#[test]
fn an_event_is_one_frame_with_its_name_and_payload() {
    assert_eq!(
        framed("board:changed", r#""prj_1""#),
        r#"{"p":"prj_1","t":"board:changed"}"#
    );
    assert_eq!(
        framed("notice:rang", "null"),
        r#"{"p":null,"t":"notice:rang"}"#
    );
}

#[test]
fn a_browser_hears_only_the_topics_a_page_may_subscribe_to() {
    assert_eq!(
        asked(r#"{"listen":["board:changed","island:cursor","nope","run:changed"]}"#),
        ["board:changed", "run:changed"]
    );
    assert!(asked("not json").is_empty());
    assert!(asked(r#"{"listen":"board:changed"}"#).is_empty());
}

#[test]
fn an_event_reaches_the_sockets_that_asked_for_it_and_drops_the_gone() {
    let (send, got) = mpsc::channel();
    let hears = std::sync::Arc::new(Socket {
        topics: Mutex::new(vec!["run:changed"]),
        send,
    });
    let (gone_send, gone) = mpsc::channel();
    drop(gone);
    let left = std::sync::Arc::new(Socket {
        topics: Mutex::new(vec!["run:changed"]),
        send: gone_send,
    });
    sockets()
        .lock()
        .expect("sockets")
        .extend([hears.clone(), left.clone()]);
    carried("board:changed", "1");
    carried("run:changed", "2");
    assert_eq!(
        got.try_recv().expect("heard"),
        r#"{"p":2,"t":"run:changed"}"#
    );
    assert!(got.try_recv().is_err(), "a topic it did not ask for");
    let open = sockets().lock().expect("sockets");
    assert!(open.iter().any(|one| std::sync::Arc::ptr_eq(one, &hears)));
    assert!(!open.iter().any(|one| std::sync::Arc::ptr_eq(one, &left)));
}

/// Every event the web shell subscribes to is one a browser can hear: a
/// topic missing here would leave that part of the page stale in Chrome.
#[test]
fn every_topic_the_web_subscribes_to_is_carried() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src");
    let mut named = std::collections::BTreeSet::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("web/src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path.to_string_lossy();
            if !(name.ends_with(".ts") || name.ends_with(".tsx")) || name.contains(".test.") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("source");
            for call in [
                "shared<",
                "shared(",
                "onEvent(",
                "onCarried<",
                "onCarried(",
                "listen<",
                "listen(",
            ] {
                for piece in text.split(call).skip(1) {
                    let piece = piece.split_once('(').map_or(piece, |(_, rest)| rest);
                    let piece = piece.trim_start_matches(['(', ' ']);
                    if let Some(rest) = piece.strip_prefix('\'') {
                        if let Some((topic, _)) = rest.split_once('\'') {
                            if topic.contains(':') {
                                named.insert(topic.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    let desktop_only = [
        "browser:driven",
        "browser:menu-did",
        "browser:menu-for",
        "browser:showing",
        "island:asked",
        "island:cursor",
        "island:open-chat",
        "island:open-pane",
        "island:session",
        "island:settled",
        "tray:new-terminal",
        "tray:open-card",
        "tray:open-project",
        "tray:settings",
    ];
    let missing: Vec<&String> = named
        .iter()
        .filter(|topic| {
            !TOPICS.contains(&topic.as_str()) && !desktop_only.contains(&topic.as_str())
        })
        .collect();
    assert!(missing.is_empty(), "web topics not carried: {missing:?}");
}
