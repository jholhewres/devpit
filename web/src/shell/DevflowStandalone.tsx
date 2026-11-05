import { useEffect, useState } from 'react'

import type { DevflowBinary, DevflowMcp, DevflowPlugin, DevflowPluginAction } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The standalone devflow, beside devpit's own: its Claude Code plugin in the
 * profile the project's sessions run as, and its binary for other tools.
 * Both write outside devpit, so an install is done only on a yes to what it
 * runs. The plugin and devpit's Devflow on together give agents the same
 * tools twice; that is said here, with a way to switch one off.
 */

const ACTING: Record<DevflowPluginAction, string> = {
  install: 'Installing…',
  update: 'Updating…',
  remove: 'Removing…',
  enable: 'Enabling…',
  disable: 'Disabling…',
}

function where(plugin: DevflowPlugin): string {
  return `${plugin.profile ?? 'the default profile'} (${plugin.directory})`
}

export function DevflowPluginRow({ projectId, devpitOn, onDeactivate }: { projectId: string; devpitOn: boolean; onDeactivate: () => void }): React.JSX.Element | null {
  const [plugin, setPlugin] = useState<DevflowPlugin | null>(null)
  const [sure, setSure] = useState(false)
  const [busy, setBusy] = useState<DevflowPluginAction | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    void ask(() => commands.devflowPlugin(projectId)).then((answer) => {
      if (!live) return
      setPlugin(answer.data)
      setProblem(answer.error)
    })
    return () => {
      live = false
    }
  }, [projectId, devpitOn])

  if (!plugin) return problem ? <p className="pref__d voice__bad">{problem}</p> : null
  const act = (action: DevflowPluginAction): void => {
    setBusy(action)
    void ask(() => commands.devflowPluginDo(projectId, action)).then((answer) => {
      if (answer.data) setPlugin(answer.data)
      setProblem(answer.error)
      setSure(false)
      setBusy(null)
    })
  }
  const installed = plugin.version !== null
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">devflow plugin for Claude Code</span>
        <span className="pref__d">
          {installed
            ? `devflow@devflow ${plugin.version}, ${plugin.enabled ? 'on' : 'off'} in ${where(plugin)}.`
            : `Not installed in ${where(plugin)}. devpit's Devflow does not need it; it is the standalone devflow, for sessions outside devpit's.`}
        </span>
        {plugin.twice && (
          <div className="pref" role="alert">
            <span className="pref__body">
              <span className="pref__t">The devflow tools twice</span>
              <span className="pref__d">The plugin is on and so is devpit&rsquo;s Devflow in this project: its agents get both sets of tools. Switch one off.</span>
            </span>
            <button className="btn" onClick={() => act('disable')} disabled={busy !== null}>
              Disable the plugin
            </button>
            <button className="btn" onClick={onDeactivate} disabled={busy !== null}>
              Deactivate devpit&rsquo;s Devflow
            </button>
          </div>
        )}
        {!installed && sure && (
          <span className="pref__d">
            This runs <code>claude plugin marketplace add jholhewres/devflow</code>, <code>claude plugin marketplace update devflow</code> and <code>claude plugin install devflow@devflow</code> with{' '}
            <code>CLAUDE_CONFIG_DIR</code> at {plugin.directory}.
          </span>
        )}
        <div className="voice__row">
          {busy && <span className="pref__d">{ACTING[busy]}</span>}
          {!installed && !sure && (
            <button className="btn" onClick={() => setSure(true)}>
              Install plugin…
            </button>
          )}
          {!installed && sure && (
            <>
              <button className="btn" onClick={() => setSure(false)} disabled={busy !== null}>
                Cancel
              </button>
              <button className="btn btn--go" onClick={() => act('install')} disabled={busy !== null}>
                Install
              </button>
            </>
          )}
          {installed && (
            <>
              {!plugin.enabled && (
                <button className="btn" onClick={() => act('enable')} disabled={busy !== null}>
                  Enable
                </button>
              )}
              <button className="btn" onClick={() => act('update')} disabled={busy !== null}>
                Update
              </button>
              <button className="btn" onClick={() => act('remove')} disabled={busy !== null}>
                Remove
              </button>
            </>
          )}
        </div>
        {problem && <span className="pref__d voice__bad">{problem}</span>}
      </div>
    </div>
  )
}

/* Whether `version` is older than `known`, by its numbers: a newer devflow
   is never offered the download that would replace it with an older one. */
function behind(version: string, known: string): boolean {
  const parts = (one: string): number[] => one.split(/[.-]/).map((part) => Number.parseInt(part, 10) || 0)
  const [have, want] = [parts(version), parts(known)]
  for (let at = 0; at < Math.max(have.length, want.length); at += 1) {
    const diff = (have[at] ?? 0) - (want[at] ?? 0)
    if (diff !== 0) return diff < 0
  }
  return false
}

export function DevflowBinaryRow(): React.JSX.Element | null {
  const [binary, setBinary] = useState<DevflowBinary | null>(null)
  const [sure, setSure] = useState(false)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    void ask(() => commands.devflowBinary()).then((answer) => live && setBinary(answer.data))
    return () => {
      live = false
    }
  }, [])

  if (!binary) return null
  const install = (): void => {
    setBusy(true)
    void ask(() => commands.devflowBinaryInstall()).then((answer) => {
      if (answer.data) setBinary(answer.data)
      setProblem(answer.error)
      setSure(false)
      setBusy(false)
    })
  }
  const installed = binary.version !== null
  const offered = binary.version === null || behind(binary.version, binary.known)
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">devflow outside devpit</span>
        <span className="pref__d">{installed ? `devflow ${binary.version} at ${binary.path}.` : `Not installed at ${binary.path}.`} The binary is for other tools; devpit&rsquo;s own Devflow does not use it.</span>
        {sure && (
          <span className="pref__d">
            This downloads devflow {binary.known} for this machine from github.com/jholhewres/devflow, refuses it unless it matches the release&rsquo;s checksums.txt, and puts it at {binary.path}.
          </span>
        )}
        <div className="voice__row">
          {busy && <span className="pref__d">Downloading…</span>}
          {sure ? (
            <>
              <button className="btn" onClick={() => setSure(false)} disabled={busy}>
                Cancel
              </button>
              <button className="btn btn--go" onClick={install} disabled={busy}>
                {installed ? 'Update' : 'Install'}
              </button>
            </>
          ) : (
            offered && (
              <button className="btn" onClick={() => setSure(true)}>
                {installed ? `Update to ${binary.known}…` : 'Install devflow…'}
              </button>
            )
          )}
        </div>
        {binary.doctor.length > 0 && (
          <pre className="errprev__body" aria-label="devflow doctor">
            {binary.doctor.join('\n')}
          </pre>
        )}
        {problem && <span className="pref__d voice__bad">{problem}</span>}
      </div>
    </div>
  )
}

/* devflow as an MCP server in each tool's own configuration, for its sessions
   outside devpit. A write to another tool's file waits for a yes to the file. */
export function DevflowMcpRow(): React.JSX.Element | null {
  const [mcp, setMcp] = useState<DevflowMcp | null>(null)
  const [sure, setSure] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    void ask(() => commands.devflowMcp()).then((answer) => {
      if (!live) return
      setMcp(answer.data)
      setProblem(answer.error)
    })
    return () => {
      live = false
    }
  }, [])

  if (!mcp) return problem ? <p className="pref__d voice__bad">{problem}</p> : null
  const set = (tool: string, added: boolean): void => {
    setBusy(true)
    void ask(() => commands.devflowMcpSet(tool, added)).then((answer) => {
      if (answer.data) setMcp(answer.data)
      setProblem(answer.error)
      setSure(null)
      setBusy(false)
    })
  }
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">devflow as an MCP server in other tools</span>
        <span className="pref__d">
          Each tool runs <code>{mcp.command} mcp</code> for its own sessions. devpit&rsquo;s Devflow does not need it.
        </span>
        {mcp.tools.map((one) => (
          <div className="pref" key={one.tool}>
            <span className="pref__body">
              <span className="pref__t">{one.label}</span>
              <span className="pref__d">
                {one.added ? `Added in ${one.file}.` : sure === one.tool ? `Not added. This writes devflow into ${one.file}, leaving the rest of it as it is.` : 'Not added.'}
              </span>
            </span>
            {one.added ? (
              <button className="btn" onClick={() => set(one.tool, false)} disabled={busy}>
                Remove
              </button>
            ) : sure === one.tool ? (
              <>
                <button className="btn" onClick={() => setSure(null)} disabled={busy}>
                  Cancel
                </button>
                <button className="btn btn--go" onClick={() => set(one.tool, true)} disabled={busy}>
                  Add
                </button>
              </>
            ) : (
              <button className="btn" onClick={() => setSure(one.tool)} disabled={busy}>
                Add…
              </button>
            )}
          </div>
        ))}
        {problem && <span className="pref__d voice__bad">{problem}</span>}
      </div>
    </div>
  )
}
