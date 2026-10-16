use std::cell::RefCell;
use std::rc::Rc;

use devpit_devflow::config::Agent;

use super::*;
use crate::devflow_adhoc::tests::checkout_with_a_feature;
use crate::devflow_adhoc::Pull;
use crate::devflow_review::tests::{fake_claude, project_at};

/// Devflow set up with one reviewer reading every axis, so nothing is pending.
fn one_reading() -> State {
    let yaml = "version: 1\nenabled: false\nreview:\n  reviewers:\n    - agent: claude\n      persona: devflow-all\n";
    let config = devpit_devflow::config::parse(yaml).expect("yaml");
    State::Valid(yaml.to_owned(), Box::new(config))
}

/// What a fake pull request was asked, in order.
#[derive(Clone, Default)]
struct Seen(Rc<RefCell<Vec<String>>>);

impl Seen {
    fn said(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

impl Host for Seen {
    fn review(
        &self,
        commit: &str,
        event: Event,
        _body: &str,
        comments: &[Inline],
    ) -> Result<Vec<u64>, String> {
        let at: Vec<String> = comments
            .iter()
            .map(|one| format!("{}:{}", one.file, one.line))
            .collect();
        self.0
            .borrow_mut()
            .push(format!("review {commit} {} {}", event.api(), at.join(",")));
        Ok((1..=comments.len() as u64).collect())
    }

    fn edit_inline(&self, id: u64, _body: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("edit {id}"));
        Ok(())
    }

    fn comment(&self, _body: &str) -> Result<u64, String> {
        self.0.borrow_mut().push("comment".to_owned());
        Ok(100)
    }

    fn edit_comment(&self, id: u64, _body: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("edit comment {id}"));
        Ok(())
    }
}

#[cfg(unix)]
#[test]
fn a_pull_requests_review_is_published_on_it_only_after_the_yes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scripts = tempfile::tempdir().expect("scripts");
    let root = dir.path().canonicalize().expect("root");
    let id = checkout_with_a_feature(&root);
    let project = project_at(&id, &root);
    let head = devpit_git::commit_of(&root, "feature").expect("feature");
    let pull_head = head.clone();
    let github = move |_: &Path, _: u64| {
        Ok(Pull {
            base_ref_name: "main".into(),
            head_ref_oid: pull_head.clone(),
            title: "Open links".into(),
            body: String::new(),
        })
    };
    let claude = fake_claude(scripts.path());
    let reach = move |agent: Agent| match agent {
        Agent::Claude => Ok(claude.clone()),
        _ => Err("not installed".to_owned()),
    };
    let asked = adhoc::standalone(&json!({ "pr": 7 }))
        .expect("asked")
        .expect("no card");
    let reviewed =
        crate::devflow_adhoc::respond_with(&project, &one_reading(), asked, &reach, &github)
            .expect("reviewed");
    let review = format!("pr-7/{}", reviewed["run"].as_str().expect("run"));
    assert!(
        reviewed["next"]
            .as_str()
            .is_some_and(|next| next.contains(&review)),
        "{reviewed}"
    );
    let seen = Seen::default();
    let pull = seen.clone();
    let hosting = move |_: &Path, pr: u64| -> Box<dyn Host> {
        assert_eq!(pr, 7);
        Box::new(pull.clone())
    };

    let drafted = respond_with(
        &project,
        &one_reading(),
        &json!({ "review": review }),
        &hosting,
    )
    .expect("drafted");
    assert_eq!(drafted["applied"], false, "{drafted}");
    assert_eq!(drafted["inline"][0]["at"], "app.py:2", "{drafted}");
    assert!(seen.said().is_empty());

    let published = respond_with(
        &project,
        &one_reading(),
        &json!({ "review": review, "apply": true }),
        &hosting,
    )
    .expect("published");
    assert_eq!(published["applied"], true, "{published}");
    assert_eq!(
        seen.said(),
        vec![
            format!("review {head} COMMENT app.py:2"),
            "comment".to_owned()
        ]
    );

    // Not a review of this project: nothing is read outside its reviews.
    let elsewhere = respond_with(
        &project,
        &one_reading(),
        &json!({ "review": "pr-8/20261014T120000Z" }),
        &hosting,
    );
    assert!(elsewhere.is_err());
}
