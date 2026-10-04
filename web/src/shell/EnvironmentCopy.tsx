import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useState } from 'react'

import { ask, commands } from './live'

/* Copies what devpit sees of this machine, for a bug report. */
export function EnvironmentCopy(): React.JSX.Element {
  const [said, setSaid] = useState<string | null>(null)

  const copy = async (): Promise<void> => {
    const answer = await ask(() => commands.diagnosticsEnvironment())
    if (!answer.data) {
      setSaid(answer.error)
      return
    }
    await writeText(answer.data.text).then(
      () => setSaid('Copied.'),
      () => setSaid(answer.data?.text ?? null),
    )
  }

  return (
    <p className="acc__note">
      <button className="btn" onClick={() => void copy()}>
        Copy environment info
      </button>
      {said && <span className="envcopy__said">{said}</span>}
    </p>
  )
}
