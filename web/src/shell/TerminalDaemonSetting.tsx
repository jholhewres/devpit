import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/*
 * Terminals held by devpit's own daemon instead of tmux, while it is a
 * preview: off unless turned on, per machine, and only for new plain
 * terminals. The ones already open stay where they were made.
 */
export function TerminalDaemonSetting(): React.JSX.Element {
  const [on, setOn] = useState(false)
  useEffect(() => {
    void ask(() => commands.terminalDaemonRead()).then((answer) => setOn(answer.data ?? false))
  }, [])
  const flip = (): void => {
    void ask(() => commands.terminalDaemonSet(!on)).then((answer) => {
      if (answer.data !== undefined) setOn(answer.data)
    })
  }
  return (
    <PrefSwitch
      on={on}
      onFlip={flip}
      title="Terminals that outlive devpit (preview)"
      said="New plain terminals run in devpit's own terminal daemon, so a crash, restart or update of devpit finds them still running."
      more="Agents and the terminals already open stay in tmux. If the daemon cannot start, a new terminal opens in tmux as before."
    />
  )
}
