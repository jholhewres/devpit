import { useEffect, useState } from 'react'

import type { ClaudePluginInstallation } from '../gen/bindings'
import { dismiss, dismissed, offerFor } from './claudePlugin'
import { ask, commands } from './live'

/*
 * "Install devpit plugin", in the strip under a terminal running Claude Code.
 *
 * The plugin is what gives a session started by the person's own command —
 * `claude2`, a function, an alias — devpit's hooks and board tools, which the
 * flags devpit puts on its own launch lines never reach. Installing is theirs
 * to choose, so it is offered here and done on a click, the way Warp offers
 * its own.
 */
type Step = { kind: 'offered' } | { kind: 'working' } | { kind: 'done' } | { kind: 'failed'; why: string }

export function PluginChip(): React.JSX.Element | null {
  const [installations, setInstallations] = useState<ClaudePluginInstallation[] | null>(null)
  const [step, setStep] = useState<Step>({ kind: 'offered' })
  const [hidden, setHidden] = useState(false)

  useEffect(() => {
    let live = true
    void ask(() => commands.claudePluginState()).then((answer) => live && setInstallations(answer.data ?? null))
    return () => {
      live = false
    }
  }, [])

  if (!installations || hidden) return null
  const offer = offerFor(installations)
  if (step.kind === 'offered' && (offer === null || dismissed(offer, installations))) return null

  const install = (): void => {
    setStep({ kind: 'working' })
    void ask(() => commands.claudePluginInstall()).then((answer) => {
      if (answer.data) {
        setInstallations(answer.data)
        setStep({ kind: 'done' })
      } else setStep({ kind: 'failed', why: answer.error ?? 'the plugin could not be installed' })
    })
  }
  const close = (): void => {
    if (step.kind === 'offered') dismiss(installations)
    setHidden(true)
  }

  const verb = offer === 'update' ? 'Update' : 'Install'
  return (
    <span className="pchip" data-step={step.kind} title={step.kind === 'failed' ? step.why : undefined}>
      {step.kind === 'offered' && (
        <button className="pchip__go" onClick={install} title="Gives every Claude Code session in a devpit terminal the board's tools and devpit's hooks">
          ↓ {verb} devpit plugin
        </button>
      )}
      {step.kind === 'working' && <span className="pchip__say">{verb === 'Update' ? 'Updating' : 'Installing'} devpit plugin…</span>}
      {step.kind === 'done' && <span className="pchip__say">devpit plugin ready — run /reload-plugins in Claude Code</span>}
      {step.kind === 'failed' && <span className="pchip__say">The plugin could not be installed</span>}
      {step.kind !== 'working' && (
        <button className="pchip__x" aria-label="Dismiss" onClick={close}>
          ×
        </button>
      )}
    </span>
  )
}
