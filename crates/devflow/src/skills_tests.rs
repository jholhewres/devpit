use super::*;
use crate::config::STAGES;

const OURS: &str = "---\nname: devflow-plan\ndescription: Our own plan.\n---\n\nPlan our way.\n";

fn write(base: &Path, relative: &str, text: &str) {
    let path = base.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(path, text).expect("write");
}

#[test]
fn every_stage_has_a_built_in_skill_that_names_itself() {
    let empty = tempfile::tempdir().expect("tempdir");
    let all = offered(&STAGES);
    assert_eq!(all, names().collect::<Vec<_>>());
    for stage in STAGES {
        assert!(
            all.contains(&format!("devflow-{stage}").as_str()),
            "{stage}"
        );
    }
    for name in all {
        let skill = resolve(empty.path(), empty.path(), name).expect(name);
        assert_eq!(skill.layer, Layer::BuiltIn);
        assert_eq!(skill.frontmatter["name"], name);
        assert_eq!(
            skill.entry()["uri"],
            format!("skill://devflow/{name}/SKILL.md")
        );
    }
}

#[test]
fn only_the_stages_switched_on_offer_their_skills() {
    assert_eq!(offered(&["plan"]), ["devflow-plan", "devflow-setup"]);
    assert_eq!(offered(&[]), ["devflow-setup"]);
}

#[test]
fn a_projects_skill_replaces_the_persons_which_replaces_the_built_in() {
    let project = tempfile::tempdir().expect("project");
    let personal = tempfile::tempdir().expect("personal");
    let resolved = || resolve(project.path(), personal.path(), "devflow-plan").expect("resolved");

    let mine = OURS.replace("Our own", "My own");
    write(personal.path(), "devflow-plan/SKILL.md", &mine);
    assert_eq!(resolved().layer, Layer::Personal);
    assert_eq!(resolved().text, mine);

    write(
        project.path(),
        ".devpit/devflow/skills/devflow-plan/SKILL.md",
        OURS,
    );
    assert_eq!(resolved().layer, Layer::Project);
    assert_eq!(resolved().text, OURS);
    assert_eq!(resolved().frontmatter["description"], "Our own plan.");

    // Another skill is untouched by either.
    let other = resolve(project.path(), personal.path(), "devflow-review").expect("review");
    assert_eq!(other.layer, Layer::BuiltIn);
}

#[test]
fn a_skill_is_listed_with_the_digest_and_size_of_its_file() {
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        ".devpit/devflow/skills/devflow-plan/SKILL.md",
        OURS,
    );
    let skill = resolve(project.path(), project.path(), "devflow-plan").expect("plan");
    assert_eq!(
        skill.entry(),
        json!({
            "uri": "skill://devflow/devflow-plan/SKILL.md",
            "frontmatter": { "name": "devflow-plan", "description": "Our own plan." },
            "resources": [{
                "uri": "skill://devflow/devflow-plan/SKILL.md",
                "digest": "sha256:529b1390994099b75e7e22760ab3ce3664cf2a5709d000dd1f831147b367f54b",
                "size": 69,
            }],
        })
    );
    assert_eq!(skill.to_json()["layer"], "project");
}

#[test]
fn a_skill_that_cannot_be_used_is_refused_rather_than_passed_over() {
    let project = tempfile::tempdir().expect("project");
    let at = ".devpit/devflow/skills/devflow-plan/SKILL.md";
    let refused = |project: &Path| resolve(project, project, "devflow-plan").expect_err("refused");

    write(
        project.path(),
        at,
        &OURS.replace("name: devflow-plan", "name: plan"),
    );
    assert!(refused(project.path()).contains("names it `plan`"));

    write(project.path(), at, "Plan our way.\n");
    assert!(refused(project.path()).contains("frontmatter"));

    let large = format!("{OURS}{}", "x".repeat(MOST_BYTES as usize));
    write(project.path(), at, &large);
    assert!(refused(project.path()).contains("larger than"));

    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().expect("outside");
        write(outside.path(), "SKILL.md", OURS);
        std::fs::remove_file(project.path().join(at)).expect("remove");
        std::os::unix::fs::symlink(outside.path().join("SKILL.md"), project.path().join(at))
            .expect("link");
        assert!(refused(project.path()).contains("outside"));
    }
}

#[test]
fn a_skill_is_named_by_its_name_or_its_uri_and_nothing_else() {
    assert_eq!(named("devflow-plan"), Some("devflow-plan"));
    assert_eq!(
        named("skill://devflow/devflow-plan/SKILL.md"),
        Some("devflow-plan")
    );
    assert_eq!(named("../devflow-plan"), None);
    assert_eq!(named("skill://devflow/../SKILL.md"), None);
    let empty = tempfile::tempdir().expect("tempdir");
    assert!(resolve(empty.path(), empty.path(), "../../etc/passwd")
        .expect_err("not a skill")
        .contains("not a Devflow skill"));
}
