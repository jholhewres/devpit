# Any agent on the island and the card

devpit hears Claude Code through its hooks. Any other agent — aider, opencode,
a CI job, a script of your own — can tell devpit what it is doing the same
way, with one command, and shows up on the island and on its card like
Claude Code does.

```sh
devpit-agent hook --agent aider SessionStart
devpit-agent hook --agent aider UserPromptSubmit
devpit-agent hook --agent aider PreToolUse --tool Edit --target src/app.rs
devpit-agent hook --agent aider PostToolUse --tool Edit
devpit-agent hook --agent aider Stop --said "Tests pass"
devpit-agent hook --agent aider SessionEnd
```

`devpit-agent` is on the `PATH` of every terminal devpit opens. Run inside
one of them, the event belongs to that terminal and its card.

## The command

`devpit-agent hook --agent NAME EVENT [options]`

- `--agent NAME` — up to 24 of `a-z`, `0-9` and `-`. `claude` is taken:
  Claude Code speaks through its own hooks.
- `EVENT` — one of `SessionStart`, `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse`, `PostToolUseFailure`, `Notification` (the agent waits on
  the person), `Stop` (a turn ended), `StopFailure`, `SessionEnd`.
- `--session ID` — which session the event belongs to. Without it, every
  event from the same agent process is one session.
- `--tool NAME`, `--target WHAT` — the step: `Edit src/app.rs`,
  `Run npm test`. A target that looks like a path is read as one.
- `--said TEXT` — what the turn ended with (`Stop`), or the error
  (`StopFailure`).

JSON piped in is taken as the payload to build on, in Claude Code's hook
shape (`tool_name`, `tool_input`, `session_id`, `cwd`…); the options win over
it:

```sh
echo '{"tool_name":"Bash","tool_input":{"command":"npm test"}}' \
  | devpit-agent hook --agent aider PreToolUse
```

The command always exits 0 and waits at most a second and a half: a report
that fails must not fail the agent sending it.

## What it never does

There is no `PermissionRequest`. An approval asked this way could pass for
one of Claude Code's, and approving is the person's, in the agent itself.

## Gemini CLI, with nothing to set up

In a devpit terminal Gemini CLI reports on its own. The terminal sets
`GEMINI_CLI_SYSTEM_DEFAULTS_PATH` to `<home>/gemini-hooks.json`: the
system's own defaults file, if there is one, with devpit's hooks added.
Gemini merges that file under the person's settings, and it concatenates hook
lists, so their own hooks still run and `~/.gemini/settings.json` is never
written. If the system's file exists but cannot be read as JSON, devpit leaves
the variable alone rather than hide it.

The hooks post with `?from=gemini`. The listener puts each report into Claude
Code's words: `BeforeTool` becomes `PreToolUse`, `AfterTool` becomes
`PostToolUse`, or `PostToolUseFailure` when the tool returned an error.
`BeforeAgent` becomes `UserPromptSubmit`, and `AfterAgent` becomes `Stop`
with the reply. A `Notification` is a permission prompt, so the session shows
as waiting. Gemini's tool names are mapped to devpit's too: `run_shell_command`
to Bash, `replace` to Edit, and so on. Nothing is answered from devpit, and
the reply is thrown away, as Gemini would show any text a hook prints.

Antigravity is not covered. It is an editor whose agent settings are global,
with no per-terminal file to point at, and devpit does not write a global file.
