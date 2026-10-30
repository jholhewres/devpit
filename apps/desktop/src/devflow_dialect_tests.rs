use super::*;

/// Sabotage: pin `.devflow` in `pinned`, and a project already on
/// `.devpit/devflow/` would be set up again from nothing.
#[test]
fn every_project_keeps_devflow_where_devpit_always_did() {
    assert_eq!(file(), ".devpit/devflow/devflow.yaml");
    assert_eq!(layout().dir, ".devpit/devflow");
    assert_eq!(layout().dialect, &DEVPIT);
}

/// Sabotage: point a skill at the engine's own text, and the agent is told
/// devflow's tools instead of `devpit_devflow_*`.
#[test]
fn the_skills_served_are_devpits_own() {
    let plan = DEVPIT
        .assets
        .skills
        .iter()
        .find(|(name, ..)| *name == "devflow-plan")
        .map(|(.., text)| *text)
        .expect("plan");
    assert_eq!(
        plan,
        include_str!("../../../flow/skills/devflow-plan/SKILL.md")
    );
    assert!(plan.contains("devpit_devflow_plan"));
}
