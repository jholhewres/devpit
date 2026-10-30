//! devpit's words for the devflow engine, and its own `flow/`: a card where
//! the engine says item, the `devpit_devflow_*` tools, `DEVPIT_*` variables,
//! and `.devpit/devflow/` as every project's folder.

use devflow_engine::dialect::{Assets, Tools};
use devflow_engine::Dialect;

macro_rules! flow {
    ($path:literal) => {
        include_str!(concat!("../../../flow/", $path))
    };
}

static FLOW: Assets = Assets {
    skills: [
        (
            "devflow-briefing",
            Some("briefing"),
            flow!("skills/devflow-briefing/SKILL.md"),
        ),
        (
            "devflow-context",
            Some("context"),
            flow!("skills/devflow-context/SKILL.md"),
        ),
        (
            "devflow-plan",
            Some("plan"),
            flow!("skills/devflow-plan/SKILL.md"),
        ),
        (
            "devflow-execution",
            Some("execution"),
            flow!("skills/devflow-execution/SKILL.md"),
        ),
        (
            "devflow-review",
            Some("review"),
            flow!("skills/devflow-review/SKILL.md"),
        ),
        (
            "devflow-completion",
            Some("completion"),
            flow!("skills/devflow-completion/SKILL.md"),
        ),
        (
            "devflow-setup",
            None,
            flow!("skills/devflow-setup/SKILL.md"),
        ),
    ],
    personas: [
        ("devflow-spec", flow!("personas/devflow-spec.md")),
        ("devflow-standards", flow!("personas/devflow-standards.md")),
        ("devflow-security", flow!("personas/devflow-security.md")),
        ("devflow-all", flow!("personas/devflow-all.md")),
    ],
    rubrics: [
        ("devflow-finding", flow!("rubrics/devflow-finding.json")),
        ("devflow-brief", flow!("rubrics/devflow-brief.json")),
        ("devflow-plan", flow!("rubrics/devflow-plan.json")),
    ],
    schema: flow!("schemas/devflow-findings.schema.json"),
    publish: [
        flow!("publish/github.md"),
        flow!("publish/gitlab.md"),
        flow!("publish/bitbucket.md"),
    ],
};

/// devpit's, as the engine's golden test pins it.
pub(crate) static DEVPIT: Dialect = Dialect {
    product: "devpit",
    version: env!("CARGO_PKG_VERSION"),
    env: "DEVPIT",
    folder: devflow_engine::folder::LEGACY,
    item: "card",
    items: "cards",
    an_item: "a card",
    work_item: "card",
    a_work_item: "a card",
    item_field: "card",
    tools: Tools {
        evidence: "devpit_devflow_evidence",
        gate: "devpit_devflow_gate",
        set: "devpit_devflow_set",
        stage: "devpit_devflow_stage",
        review: "devpit_devflow_review",
        publish: "devpit_devflow_publish",
        handoff: "devpit_handoff",
        finish: "devpit_finish_card",
    },
    files_label: "Attached",
    files_more: "among the artifacts",
    packet_heading: "Card",
    plan_board: "cards are made on the devpit board.",
    plan_outside: "is not another card of this plan, nor a card on the board (`card_…` or `#42`)",
    yaml_header: "# Devflow: briefing → context → plan → execution → review → completion,\n\
                  # served by devpit's MCP. This file is the source of truth: devpit edits\n\
                  # it and never commits it.\n",
    yaml_enabled: "# Switched on by the person's click in the devpit app, never by an agent.\n",
    yaml_enforce: "waits for the person's click in the app",
    yaml_provider: "the provider in Settings → Decisions",
    decisions_opted_out: "this project's state is not sent to Decisions — see Settings → Decisions",
    decisions_no_key: "no key — set one in Settings → Decisions",
    assets: &FLOW,
};

/// Pins `.devpit/devflow/` and devpit's way of starting programs, once,
/// before the engine reads anything. Later calls do nothing.
pub(crate) fn pinned() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = devflow_engine::folder::pin(devflow_engine::folder::LEGACY);
        let _ = devflow_engine::env::spawn_with(|program| devpit_pty::host_env::command(program));
    });
}

/// `.devpit/devflow/devflow.yaml`, relative to a project's root.
pub(crate) fn file() -> String {
    pinned();
    devflow_engine::folder::default_file()
}

/// The layout every project's folder is written in.
pub(crate) fn layout() -> devflow_engine::folder::Layout {
    pinned();
    devflow_engine::folder::Layout {
        dialect: &DEVPIT,
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "devflow_dialect_tests.rs"]
mod tests;
