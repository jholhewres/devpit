//! One orchestrator on two machines: its folder and its projects' ids differ
//! on each, and its links still settle on the same repositories.

mod support;

use std::path::PathBuf;

use devpit_core::origin::origin_hash;
use devpit_sync::clock::Clock;
use devpit_sync::{files, State, SyncKey};
use serde_json::{json, Value};
use support::{opened, Home, Stand};

const P: &str = "git@github.com:jo/p.git";
const Q: &str = "https://github.com/jo/q";
const R: &str = "git@github.com:jo/r.git";

struct Machine {
    home: Home,
    orchestrator: PathBuf,
}

impl Machine {
    fn new(id: &str, made: &SyncKey) -> Machine {
        let home = Home::new(id);
        files::keep_key(&home.root(), made).expect("keep");
        files::keep_state(
            &home.root(),
            &State {
                machine: id.to_owned(),
                clock: Clock::new(id),
                spaces: Default::default(),
                synced_at: None,
            },
        )
        .expect("state");
        // Under each machine's own home, so its path differs on each.
        let orchestrator = home.root().join("orchestrator").join("work");
        std::fs::create_dir_all(&orchestrator).expect("folder");
        home.store
            .add_project(&orchestrator, None)
            .expect("orchestrator");
        Machine { home, orchestrator }
    }

    fn project(&self, name: &str, origin: Option<&str>) -> String {
        let root = self.home.root().join(name);
        std::fs::create_dir_all(&root).expect("checkout");
        self.home.store.add_project(&root, origin).expect("add")
    }

    fn link(&self, ids: &[&String]) {
        self.home
            .store
            .set_orchestrator_setting(&self.orchestrator, "projects", json!(ids))
            .expect("links");
    }

    fn settings(&self) -> Value {
        self.home
            .store
            .orchestrator_settings(&self.orchestrator)
            .expect("settings")
    }

    fn linked(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.settings()["projects"]
            .as_array()
            .map(|all| {
                all.iter()
                    .filter_map(|one| one.as_str())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        ids.sort();
        ids
    }
}

async fn rounds(machines: &mut [Machine], stand: &Stand, times: usize) {
    for _ in 0..times {
        for one in machines.iter_mut() {
            one.home.sync(stand).await.expect("round");
        }
    }
}

fn sorted(mut ids: Vec<&String>) -> Vec<String> {
    ids.sort();
    ids.into_iter().cloned().collect()
}

#[tokio::test]
async fn links_settle_on_the_same_repositories_whatever_each_machine_calls_them() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut machines = [Machine::new("m_a", &made), Machine::new("m_b", &made)];
    let (pa, qa, ra) = (
        machines[0].project("p", Some(P)),
        machines[0].project("q", Some(Q)),
        machines[0].project("r", Some(R)),
    );
    let (pb, qb) = (
        machines[1].project("p-clone", Some("https://github.com/jo/p")),
        machines[1].project("q-clone", Some("git@github.com:jo/q.git")),
    );
    let notes = machines[1].project("notes", None);
    machines[0].link(&[&pa, &ra]);
    machines[1].link(&[&notes]);

    rounds(&mut machines, &stand, 3).await;

    assert_eq!(machines[0].linked(), sorted(vec![&pa, &ra]));
    assert_eq!(
        machines[1].linked(),
        sorted(vec![&pb, &notes]),
        "B takes A's link to p, and keeps its own link to a folder with no origin"
    );
    assert_eq!(
        machines[1].settings()["elsewhere"],
        json!([origin_hash(R)]),
        "r is linked, and not on this computer"
    );

    // B moves the links: p off, q on.
    machines[1].link(&[&notes, &qb]);
    rounds(&mut machines, &stand, 3).await;

    assert_eq!(machines[0].linked(), sorted(vec![&qa, &ra]));
    assert_eq!(machines[1].linked(), sorted(vec![&notes, &qb]));

    let sealed = opened(&made, &stand.kept());
    for local in [&pa, &qa, &ra, &pb, &qb, &notes] {
        assert!(
            !sealed.contains(local.as_str()),
            "a local id synced: {local}"
        );
    }
    for machine in &machines {
        let home = machine.home.root();
        assert!(
            !sealed.contains(home.to_string_lossy().as_ref()),
            "a path synced"
        );
    }
}

#[tokio::test]
async fn an_orchestrator_settles_and_stops_sending() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut machines = [Machine::new("m_a", &made), Machine::new("m_b", &made)];
    let pa = machines[0].project("p", Some(P));
    machines[1].project("p", Some(P));
    machines[0].link(&[&pa]);
    rounds(&mut machines, &stand, 3).await;

    for one in machines.iter_mut() {
        let outcome = one.home.sync(&stand).await.expect("round");
        let mentions = |list: &[String]| list.iter().any(|space| space == "orchestrators");
        assert!(
            !mentions(&outcome.sent) && !mentions(&outcome.written),
            "{outcome:?}"
        );
    }
}
