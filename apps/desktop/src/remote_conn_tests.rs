use super::*;

/// A viewer built before ptyd reads the list it always did: a ptyd pane is
/// one more terminal in the same shape, after the tmux ones.
#[test]
fn a_ptyd_pane_is_listed_beside_tmux_ones_in_the_shape_an_old_viewer_reads() {
    let tmux = vec![devpit_tmux::Running {
        leaf_id: "leaf_t".into(),
        command: "zsh".into(),
        tty: "/dev/pts/3".into(),
        profile: String::new(),
    }];
    let ptyd = vec![devpit_rpc::PaneRunning {
        pane_id: "leaf_p".into(),
        command: "claude".into(),
        busy: true,
        agent: Some("claude".into()),
        label: "Claude Code".into(),
    }];
    let listed = RemoteOut::Projects {
        projects: vec![RemoteProject {
            id: "p".into(),
            name: "P".into(),
            group: None,
            terminals: terminals(tmux, ptyd),
        }],
    };
    assert_eq!(
        serde_json::to_string(&listed).expect("json"),
        r#"{"t":"projects","projects":[{"id":"p","name":"P","group":null,"terminals":[{"pane":"leaf_t","command":"zsh"},{"pane":"leaf_p","command":"claude"}]}]}"#
    );
}

/// What an old viewer sends about a pane is read as it always was, whoever
/// holds the pane: nothing about the wire changed for ptyd.
#[test]
fn an_old_viewers_pane_messages_are_read_unchanged() {
    let read = |text: &str| serde_json::from_str::<RemoteIn>(text).expect(text);
    assert_eq!(
        read(r#"{"t":"paneOpen","project":"p","pane":"leaf_p","cols":40,"rows":20}"#),
        RemoteIn::PaneOpen {
            project: "p".into(),
            pane: "leaf_p".into(),
            cols: 40,
            rows: 20
        }
    );
    assert_eq!(
        read(r#"{"t":"paneInput","pane":"leaf_p","b64":"bHMK"}"#),
        RemoteIn::PaneInput {
            pane: "leaf_p".into(),
            b64: "bHMK".into()
        }
    );
    assert_eq!(
        read(r#"{"t":"panePaste","pane":"leaf_p","text":"ls"}"#),
        RemoteIn::PanePaste {
            pane: "leaf_p".into(),
            text: "ls".into()
        }
    );
    assert_eq!(
        read(r#"{"t":"paneClose","pane":"leaf_p"}"#),
        RemoteIn::PaneClose {
            pane: "leaf_p".into()
        }
    );
}
