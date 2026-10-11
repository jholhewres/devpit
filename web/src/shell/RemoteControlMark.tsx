import type { RemoteState } from '../gen/bindings'
import { ask, commands } from './live'
import { went } from './problems'

/** Remote Control's state, in words. */
export function remoteSays(remote: RemoteState): string {
  if (!remote.on) return 'Remote Control off'
  return remote.url ? 'Remote Control on' : 'Remote Control on — no page on claude.ai to open'
}

/*
 * Whether a session can be reached by Claude Code's Remote Control: a button
 * to its page while on, a mark otherwise. Turned on and off by /remote-control.
 */
export function RemoteControlMark({ remote, name }: { remote: RemoteState; name: string }): React.JSX.Element {
  const said = remoteSays(remote)
  const icon = (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="7" y="2" width="10" height="20" rx="2" /><path d="M11 18h2" /></svg>
  )
  if (remote.on && remote.url) {
    const url = remote.url
    return (
      <button className="sess__icon rctl" data-on="true" title={`${said} — open ${url}`} aria-label={`Open ${name} on claude.ai`} onClick={() => void ask(() => commands.urlOpen(url)).then(went)}>
        {icon}
      </button>
    )
  }
  return (
    <span className="rcmark rctl" data-on={remote.on ? 'true' : undefined} role="img" title={said} aria-label={`${said} for ${name}`}>
      {icon}
    </span>
  )
}
