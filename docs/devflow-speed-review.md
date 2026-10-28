# Devflow speed review (0.1.45)

Measured on 2026-10-28 by running five devpit cards through the whole flow
(execution → evidence → review → gate → handoff) on this repository, plus the
Decisions log since 2026-10-01. An analysis of 14 October measured cost
(the main session about 70 % of it); this one measures time.

## Where the time goes

| Card | Wall clock | Fix loops | Evidence runs | Reviews |
|---|---|---|---|---|
| Remove the autopilot | ~30 min | 0 | 3 (one killed at 931 s on `make e2e`, one only to re-record a proof file) | 1 × 3 personas |
| #249 reviewers' account | ~30 min | 2 | 3 | 2 × 3 personas, 1 × `devflow-all` |
| #248 publish | ~20 min | 3 | 4 | 1 × 3 personas, 2 × `devflow-all` |
| #219 project switch | ~25 min | 2 | 4 (one failed on the file-size guard) | 3 × `devflow-all` |
| #215 board runs | ~14 min | 1 | 3 (one failed on a flaky web test) | 2 × `devflow-all` |

Per step:

| Step | Time | Notes |
|---|---|---|
| Evidence (`make test`) | ~4 min a run, 2–4 runs a card | the largest share by far; every loop pays it again |
| Evidence with `make e2e` | > 15 min, never finished | the agent's wait for `devflow_evidence` is ~20 min, so a gate with e2e could not pass at all |
| Review, 3 personas in parallel | 80–150 s, ~100–130k tokens each | wall clock is the slowest persona |
| Review, one `devflow-all` (Opus) | 50–80 s, ~85–95k tokens | same findings quality on deltas |
| Review, one `devflow-all` (Sonnet), tiny delta | 10 s, 67k tokens | one sample; model tiers are card 7a |
| Jev consolidation per review | 2.6 s average, 26 s max | 4.7 findings a review, ~0.7 s each |
| Gate | < 1 s | |
| Skills | tokens, not time | review skill 14.7k chars; personas 5.6–8.5k; the review packet 46 KB, read once per reviewer |

Two things cost time without being steps:

- **`devflow_evidence` blocks the app's agent API.** While it ran, a
  `devpit_devflow_skill` call waited more than 120 s.
- **A proof file's existence is frozen at evidence time.** A gate failed for a
  proof written a minute after the run, and only a whole new `make test` run
  could record it.

## What made the loops

Every fix loop came from a real medium finding (a comment that could take
another's id and edit the wrong text on GitHub; a deleted profile pinning the
reviewers; a read-modify-write without a lock). None were critical or high,
so none blocked; they were fixed because they were right. The loops were
worth it; their cost is the 4-minute evidence each time.

Noise that cost reading time:

- A re-review with `previous` carries every non-blocking finding of the
  earlier run as still standing, fixed or not, so a card's comment grows with
  findings already fixed.
- The first review of the autopilot card took its base from before an
  unrelated docs commit, and two findings were about that commit.

Friction found on the way:

- Criteria under `## Critérios de aceite` are not read (`on_card: false`);
  only `## Acceptance criteria` is. Older Portuguese cards fail silently.
- Evidence maps cargo test names but not vitest names, so a web criterion
  needs a proof file.
- `devpit_devflow_set` re-lays devflow.yaml and drops fields at their default
  (`e2e_dir`).
- `tileRedraw.test.tsx` fails under load (`findByText` over 1 s), and with it
  the unit check.
- The installed 0.1.44 runs headless reviewers on the default account (#249),
  so this run used `review.reviewers: []` and session subagents.

## Jev (Decisions)

| Rubric | Calls since 1 Oct | Cost | Avg latency | pass / grey / fail |
|---|---|---|---|---|
| devflow-finding | 3,687 | $0.125 | 665 ms | 1,487 / 1,159 / 944 |
| devflow-done | 196 | $0.011 | 680 ms | 163 / 27 / 6 |
| devflow-brief | 103 | $0.005 | 863 ms | 102 / 0 / 0 |
| devflow-plan | 90 | $0.006 | 704 ms | 5 / 27 / 57 |

All in shadow: nothing was ever held by Jev. Against the person's own verdict
(1,046 findings and cards the person called real or done), Jev agreed 42 %
of the time (437 pass, 384 grey, 225 fail). Today's consolidations merged
nothing ("13 findings into 13").

What it does today: answers typed questions about each finding (is it real,
is the severity right), each card at the gate, each brief and each plan, and
logs them for calibration. Cost and latency are negligible: $0.15 in four
weeks, under 3 s a review. Value is low: on findings it disagrees with the
person more often than it agrees, the brief rubric has never said anything
but pass, and the plan rubric fails most plans.

Recommendation: **keep it in shadow, cheap as it is, for `devflow-done` and
`devflow-finding` calibration; drop the brief rubric from the default; make
the plan rubric advisory text, not a pass/fail; stop calling the
consolidation "jev" by default** until it merges something.

## Proposed cuts

| # | Cut | Saving | Risk |
|---|---|---|---|
| 1 | Per-card checks are the fast suite; e2e runs at release (preset and detection stop proposing `make e2e` per card) | makes the gate possible here; ~15 min a card where e2e was set | e2e regressions found later, at release |
| 2 | The gate re-checks proof files at gate time, and a passing run at the same HEAD is reused | ~4 min per proof-only redo | a stale run must still be refused after any code change |
| 3 | `devflow_evidence` runs off the agent-API thread | other tools answer during the 4 min | none |
| 4 | `review.scale` on by default: one `devflow-all` for deltas (`previous`) and diffs under the threshold; three personas only on a first review of a big or sensitive diff | 30–70 s and ~200k tokens a review | one reading for three axes on small diffs |
| 5 | Model tier per stage and reviewer (card 7a): Sonnet for `devflow-all` on deltas and for spec/standards | 5–8× faster reviewer, fewer tokens | weaker findings: measure on the next cards |
| 6 | A re-review re-checks the carried findings whose lines the delta touched, and drops those it no longer sees | shorter comments, no rereading fixed findings | a carried finding missed by mistake |
| 7 | Evidence reads vitest names (JSON reporter) | no proof-file workaround for web criteria | none |
| 8 | Criteria under a localized heading are read, or the card says they are not | no silent `on_card: false` | none |
| 9 | Split the review skill: retro, publish and review-without-card load on demand | ~40 % of the review skill's tokens per card session | an agent that needs them must ask |
| 10 | Fix the `tileRedraw` flake | no false red unit check under load | none |
| 11 | Jev: drop the brief rubric, plan rubric advisory, consolidation off by default | a few calls per card | none measurable |

Cuts 1 and 4 can be done in this repository's devflow.yaml today (1 is done,
on Jhol's yes); the rest are code, and land in devpit now and in stagegate
after the extraction.
