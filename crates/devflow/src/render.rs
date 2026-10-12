//! A configuration written out as devflow.yaml, with the comments a person
//! needs to edit it by hand. The one layout devpit writes; reading it back
//! gives the same configuration.

use std::fmt::Write;

use crate::config::{Approved, Config, Stage};

/// The text of `config`, commented.
pub fn render(config: &Config) -> String {
    let mut out = String::new();
    let stages = &config.stages;
    out.push_str(
        "# Devflow: briefing → context → plan → execution → review → completion,\n\
         # served by devpit's MCP. This file is the source of truth: devpit edits\n\
         # it and never commits it.\n",
    );
    let _ = writeln!(out, "version: {}", config.version);
    out.push_str("\n# Switched on by the person's click in the devpit app, never by an agent.\n");
    let _ = writeln!(out, "enabled: {}", config.enabled);

    out.push_str("\nstages:\n");
    out.push_str("  # approve: person stops for the person's yes; none goes on.\n");
    approved(&mut out, "briefing", &stages.briefing);
    plain(&mut out, "context", &stages.context);
    approved(&mut out, "plan", &stages.plan);
    let execution = &stages.execution;
    let _ = writeln!(out, "  execution:\n    enabled: {}", execution.enabled);
    if execution.checks.is_empty() {
        out.push_str("    checks: []\n");
    } else {
        out.push_str(
            "    # Run by devpit from the project's root for the evidence on the card.\n\
             \x20   # report: junit | vitest | jest | cargo | none\n\
             \x20   checks:\n",
        );
        for check in &execution.checks {
            let _ = writeln!(out, "      - name: {}", scalar(&check.name));
            let _ = writeln!(out, "        run: {}", scalar(&check.run));
            let _ = writeln!(out, "        report: {}", check.report.word());
            if !check.evidence.is_empty() {
                let kept: Vec<&str> = check.evidence.iter().map(|one| one.word()).collect();
                let _ = writeln!(out, "        evidence: [{}]", kept.join(", "));
            }
        }
    }
    let _ = writeln!(out, "    e2e_dir: {}", scalar(&execution.e2e_dir));
    plain(&mut out, "review", &stages.review);
    let _ = writeln!(
        out,
        "  completion:\n    enabled: {}\n    gate: {}",
        stages.completion.enabled,
        scalar(&stages.completion.gate)
    );

    out.push_str(
        "\nreview:\n  # The agent CLI each persona runs in; a persona not listed is\n  \
         # reviewed by the session itself.\n",
    );
    if config.review.reviewers.is_empty() {
        out.push_str("  reviewers: []\n");
    } else {
        out.push_str("  reviewers:\n");
        for reviewer in &config.review.reviewers {
            let _ = writeln!(out, "    - agent: {}", reviewer.agent.word());
            let _ = writeln!(out, "      persona: {}", scalar(&reviewer.persona));
            if let Some(model) = &reviewer.model {
                let _ = writeln!(out, "      model: {}", scalar(model));
            }
        }
    }
    let severities: Vec<&str> = config
        .review
        .block_on
        .iter()
        .map(|one| one.word())
        .collect();
    let _ = writeln!(out, "  block_on: [{}]", severities.join(", "));
    out.push_str(
        "  # How several reviewers' findings become one: jev (Decisions, else a\n  \
         # model pass), agent (one model pass) or none (merged as they are).\n",
    );
    let _ = writeln!(out, "  consolidate: {}", config.review.consolidate.word());

    out.push_str(
        "\n# Decisions (Jev) answers typed questions about the gates. shadow logs\n\
         # them; enforce blocks, and waits for the person's click in the app.\n\
         decisions:\n",
    );
    let decisions = &config.decisions;
    let _ = writeln!(
        out,
        "  use: {}\n  mode: {}",
        decisions.use_,
        decisions.mode.word()
    );
    if !decisions.rubrics.is_empty() {
        out.push_str("  rubrics:\n");
        for (stage, rubric) in &decisions.rubrics {
            let _ = writeln!(out, "    {stage}: {}", scalar(rubric));
        }
    }
    out
}

fn plain(out: &mut String, name: &str, stage: &Stage) {
    let _ = writeln!(out, "  {name}:\n    enabled: {}", stage.enabled);
}

fn approved(out: &mut String, name: &str, stage: &Approved) {
    let _ = writeln!(
        out,
        "  {name}:\n    enabled: {}\n    approve: {}",
        stage.enabled,
        stage.approve.word()
    );
}

/// A string as YAML: plain when it reads back as the same string, double
/// quoted otherwise.
pub(crate) fn scalar(text: &str) -> String {
    if plain_enough(text) {
        return text.to_owned();
    }
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for one in text.chars() {
        match one {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            one if one.is_control() => {
                let _ = write!(quoted, "\\u{:04x}", one as u32);
            }
            one => quoted.push(one),
        }
    }
    quoted.push('"');
    quoted
}

/// Whether `text` written bare reads back as itself and as a string.
fn plain_enough(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    let typed = matches!(
        text.to_ascii_lowercase().as_str(),
        "true" | "false" | "null" | "~" | "yes" | "no" | "on" | "off"
    ) || text.parse::<f64>().is_ok();
    !typed
        && (first.is_ascii_alphanumeric() || "./_$".contains(first))
        && !text.ends_with([' ', ':'])
        && !text.contains(": ")
        && !text.contains(" #")
        && text
            .chars()
            .all(|one| one.is_ascii_alphanumeric() || " ./_-=+@:$".contains(one))
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
