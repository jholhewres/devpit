import { useEffect, useState } from 'react'
import { commands } from '../gen/bindings'
import type { AppInfo, Capabilities } from '../gen/bindings'

/**
 * Proof that one channel crosses the whole stack.
 *
 * Reads what the Rust side actually reports rather than anything hardcoded: a
 * screen that renders fixture data proves nothing about the wiring under it.
 * It lives on the diagnostics surface because it answers the first question of
 * anyone taking a backup or filing a bug, and none of the questions of someone
 * doing the work.
 */
export function BuildReadout(): React.JSX.Element {
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [capabilities, setCapabilities] = useState<Capabilities | null>(null)
  const [failure, setFailure] = useState<string | null>(null)

  useEffect(() => {
    void (async () => {
      try {
        const read = await commands.appInfo()
        if (read.status === 'error') {
          // The contract's message is written to be read, so it is shown as-is.
          setFailure(read.error.message)
          return
        }
        setInfo(read.data)
        setCapabilities(await commands.appCapabilities())
      } catch (thrown) {
        // The generated wrapper rethrows real Errors — a broken bridge, a
        // missing permission — rather than folding them into the contract's
        // error shape. Without this catch the promise rejects unhandled and
        // the panel stays silent forever, which reads as a hung app.
        setFailure(thrown instanceof Error ? thrown.message : String(thrown))
      }
    })()
  }, [])

  return (
    <>
      <h3>This build</h3>
      {failure ? <p className="failure">{failure}</p> : null}

      {info ? (
        <div className="rows">
          <div className="row">
            <span className="row__main">version</span>
            <span className="row__meta">{info.version}</span>
          </div>
          <div className="row">
            <span className="row__main">platform</span>
            <span className="row__meta">{info.platform}</span>
          </div>
          <div className="row">
            <span className="row__main">state</span>
            <span className="row__meta">{info.statePath}</span>
          </div>
        </div>
      ) : failure ? null : (
        <p className="empty">No answer from the process — is the window running outside Tauri?</p>
      )}

      {capabilities ? (
        <>
          <h3>Capabilities</h3>
          <ul className="capabilities">
            {Object.entries(capabilities).map(([name, on]) => (
              // Off is the honest default in this phase, and it is drawn as
              // such rather than hidden: a capability the build does not have
              // should read as absent.
              <li key={name} data-on={on}>
                {name}
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </>
  )
}
