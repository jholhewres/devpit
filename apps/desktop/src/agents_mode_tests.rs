use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::{answer, brief_in, on, IMPORT, KEY};

fn en() -> crate::session_locale::SessionLocale {
    crate::session_locale::SessionLocale::of(devpit_core::locale::Locale::En)
}

fn pt_br() -> crate::session_locale::SessionLocale {
    crate::session_locale::SessionLocale::of(devpit_core::locale::Locale::PtBr)
}

const SEEDED: &str = include_str!("orchestrator_claude.md");

/// A folder seeded the way an orchestrator's is: the person's `CLAUDE.md`
/// and devpit's brief beside it.
fn seeded() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tmp");
    std::fs::write(dir.path().join("CLAUDE.md"), SEEDED).expect("seeded");
    std::fs::create_dir_all(dir.path().join(".devpit")).expect("dir");
    std::fs::write(dir.path().join(".devpit/orchestrator.md"), "devpit").expect("brief");
    dir
}

/// The orchestrator's context: what `CLAUDE.md` imports, and whether each
/// imported file is there to be read.
fn context(folder: &Path) -> Vec<(String, bool)> {
    std::fs::read_to_string(folder.join("CLAUDE.md"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().strip_prefix('@'))
        .map(|path| (path.to_owned(), folder.join(path).is_file()))
        .collect()
}

/// Every file under `folder`, byte for byte.
fn files(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let bytes = std::fs::read(&path).expect("file");
                out.insert(path.strip_prefix(root).expect("under").to_path_buf(), bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(folder, folder, &mut out);
    out
}

fn orchestrator(root: &Path) -> devpit_rpc::Project {
    serde_json::from_value(json!({
        "id": "prj_orch", "name": "orch", "rootPath": root.display().to_string(),
        "group": null, "accent": "#000000", "worktrees": [], "unreadable": null,
        "orchestrator": "claude", "origin": null, "lastOpenedAt": null,
        "icon": null, "color": null,
    }))
    .expect("a project")
}

#[test]
fn turning_agents_mode_on_writes_its_brief_and_import_and_off_removes_both() {
    let dir = seeded();
    let folder = dir.path();

    brief_in(folder, true, en()).expect("on");
    assert_eq!(
        context(folder),
        [
            (".devpit/orchestrator.md".to_owned(), true),
            (".devpit/agents.md".to_owned(), true),
        ]
    );
    let brief = std::fs::read_to_string(folder.join(".devpit/agents.md")).expect("brief");
    assert!(brief.contains("You are the person's right hand"), "{brief}");
    let agents = std::fs::read_to_string(folder.join("AGENTS.md")).expect("AGENTS.md");
    assert!(
        agents.contains("You are the person's right hand"),
        "{agents}"
    );

    brief_in(folder, false, en()).expect("off");
    assert_eq!(
        context(folder),
        [(".devpit/orchestrator.md".to_owned(), true)]
    );
    assert!(!folder.join(".devpit/agents.md").exists());
    assert_eq!(
        std::fs::read_to_string(folder.join("CLAUDE.md")).expect("read"),
        SEEDED,
        "the person's file is as it was"
    );
}

/// With both modes on, devpit's imports keep one order, whichever mode the
/// person turned on first, and writing the folder again moves nothing.
#[test]
fn with_both_modes_on_the_imports_keep_one_order_whichever_came_first() {
    let order = [
        ".devpit/orchestrator.md".to_owned(),
        ".devpit/assistant.md".to_owned(),
        ".devpit/agents.md".to_owned(),
    ];
    for agents_first in [true, false] {
        let dir = seeded();
        let folder = dir.path();
        if agents_first {
            brief_in(folder, true, en()).expect("agents");
        }
        crate::assistant::brief_in(folder, true, en()).expect("assistant");
        brief_in(folder, true, en()).expect("agents");
        let names: Vec<String> = context(folder).into_iter().map(|(path, _)| path).collect();
        assert_eq!(names, order, "agents first: {agents_first}");
        let before = files(folder);
        crate::assistant::brief_in(folder, true, en()).expect("again");
        brief_in(folder, true, en()).expect("again");
        assert_eq!(files(folder), before, "agents first: {agents_first}");
    }
    // A file whose brief line was removed keeps the same order at its end.
    let mine = "Notes\n";
    let assistant = crate::assistant::imported(mine, true);
    let both = crate::orchestrator_brief::imported(&assistant, IMPORT, true);
    assert_eq!(both, format!("Notes\n@.devpit/assistant.md\n{IMPORT}\n"));
    assert_eq!(crate::assistant::imported(&both, true), both);
}

/// With the mode off, every file devpit writes in an orchestrator's folder is
/// the one it always was: the same bytes whether the mode was never touched,
/// set off, or turned on and off again.
#[test]
fn with_agents_mode_off_the_orchestrators_files_are_byte_identical() {
    let dir = tempfile::tempdir().expect("tmp");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    let folder = dir.path().join("orch");
    std::fs::create_dir_all(&folder).expect("folder");

    crate::orchestrator::written_in(&store, &folder, en()).expect("written");
    let never = files(&folder);
    assert_eq!(
        never.get(Path::new("CLAUDE.md")).map(Vec::as_slice),
        Some(SEEDED.as_bytes())
    );
    assert_eq!(
        never
            .get(Path::new(".devpit/orchestrator.md"))
            .map(Vec::as_slice),
        Some(crate::orchestrator::brief(en()).as_bytes())
    );
    assert!(!never.contains_key(Path::new(".devpit/agents.md")));
    for text in never.values() {
        assert!(!String::from_utf8_lossy(text).contains(IMPORT));
    }

    store
        .set_orchestrator_setting(&folder, KEY, json!(false))
        .expect("off");
    crate::orchestrator::written_in(&store, &folder, en()).expect("written");
    assert_eq!(files(&folder), never, "set off");

    store
        .set_orchestrator_setting(&folder, KEY, json!(true))
        .expect("on");
    crate::orchestrator::written_in(&store, &folder, en()).expect("written");
    assert_ne!(files(&folder), never, "on writes its brief");

    store
        .set_orchestrator_setting(&folder, KEY, json!(false))
        .expect("off");
    crate::orchestrator::written_in(&store, &folder, en()).expect("written");
    assert_eq!(files(&folder), never, "turned on and off again");
}

/// An orchestrator whose mode nobody ever touched is left as it is, even
/// with a file of its own at the brief's path or a line naming it.
#[test]
fn a_never_touched_orchestrator_keeps_a_file_and_line_of_that_name() {
    let dir = tempfile::tempdir().expect("tmp");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    let folder = dir.path().join("orch");
    std::fs::create_dir_all(folder.join(".devpit")).expect("folder");
    let mine = format!("{SEEDED}{IMPORT}\n");
    std::fs::write(folder.join("CLAUDE.md"), &mine).expect("mine");
    std::fs::write(folder.join(".devpit/agents.md"), "my own notes").expect("mine");

    crate::orchestrator::written_in(&store, &folder, en()).expect("written");
    assert_eq!(
        std::fs::read_to_string(folder.join("CLAUDE.md")).expect("read"),
        mine
    );
    assert_eq!(
        std::fs::read_to_string(folder.join(".devpit/agents.md")).expect("read"),
        "my own notes"
    );
}

/// Turning the mode off removes nothing through a `.devpit` that is a link to
/// somewhere else.
#[cfg(unix)]
#[test]
fn off_removes_nothing_through_a_linked_devpit_folder() {
    let dir = tempfile::tempdir().expect("tmp");
    let folder = dir.path().join("orch");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    std::fs::write(folder.join("CLAUDE.md"), SEEDED).expect("seeded");
    std::fs::write(elsewhere.join("agents.md"), "theirs").expect("theirs");
    std::os::unix::fs::symlink(&elsewhere, folder.join(".devpit")).expect("link");

    let _ = brief_in(&folder, false, en());
    assert_eq!(
        std::fs::read_to_string(elsewhere.join("agents.md")).expect("still there"),
        "theirs"
    );
}

/// Off for a new orchestrator, on once the person turns it on, and kept.
#[test]
fn agents_mode_is_off_by_default_and_kept_as_the_person_set_it() {
    let dir = tempfile::tempdir().expect("tmp");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    let folder = dir.path().join("orch");
    std::fs::create_dir_all(&folder).expect("folder");
    assert!(!on(&store, &folder), "off by default");
    store
        .set_orchestrator_setting(&folder, KEY, json!(true))
        .expect("on");
    let reopened = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    assert!(on(&reopened, &folder), "kept");
    assert!(
        !crate::assistant::on(&reopened, &folder),
        "the other mode is its own"
    );
}

/// The agents' tools answer an orchestrator whose Agents mode is on, and
/// refuse it otherwise: called by name with the mode off, they are refused.
#[test]
fn the_agents_tools_answer_only_an_orchestrator_in_agents_mode() {
    let dir = tempfile::tempdir().expect("tmp");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    let folder = dir.path().join("orch");
    std::fs::create_dir_all(&folder).expect("folder");
    let here = orchestrator(&folder);

    let refused = answer(&store, "agents", &here).expect_err("off");
    assert!(refused.contains("Agents mode is off"), "{refused}");
    let mut session = here.clone();
    session.orchestrator = None;
    store
        .set_orchestrator_setting(&folder, KEY, json!(true))
        .expect("on");
    assert!(
        answer(&store, "agents", &session).is_err(),
        "a project's session"
    );
    assert_eq!(
        answer(&store, "agents", &here).expect("on"),
        json!({ "agents": [] })
    );
}

/// Portuguese writes the pt-BR agents brief under the same line naming the
/// build, a translation that stays within a quarter more characters.
#[test]
fn portuguese_writes_the_pt_br_agents_brief() {
    let dir = seeded();
    brief_in(dir.path(), true, pt_br()).expect("on");
    let pt = include_str!("agents_mode_devpit_pt_br.md");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".devpit/agents.md")).expect("agents"),
        format!(
            "<!-- written by devpit {} -->\n{pt}",
            env!("CARGO_PKG_VERSION")
        )
    );
    let en = include_str!("agents_mode_devpit.md");
    assert!(pt.chars().count() <= en.chars().count() * 5 / 4);
}
