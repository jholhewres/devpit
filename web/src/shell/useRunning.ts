import { useCallback, useState } from 'react'

import type { PaneRunning } from '../gen/bindings'
import { ask, commands } from './live'
import { whileWatched } from './whileWatched'

/*
 * What each of a project's panes has in the foreground.
 *
 * Asked of the operating system through tmux rather than reported by the
 * programs themselves: an agent CLI opened in a terminal has no reason to tell
 * this app it exists. The other route is writing hooks into every CLI's own
 * settings file, which reaches further — a status, a session id, a prompt —
 * and edits files this app does not own, in the person's home directory,
 * without being asked. This answers the question the sidebar actually has.
 *
 * Polled, because a foreground process changes with no event to hang on. Two
 * seconds is slow enough to be free and fast enough that starting an agent
 * shows up before you have looked away — and only while the window is on
 * screen, because a minimised devpit asking tmux every two seconds is asking
 * for nobody.
 */

const EVERY_MS = 2000

export function useRunning(projectId: string | null): readonly PaneRunning[] {
  const [running, setRunning] = useState<readonly PaneRunning[]>([])

  const look = useCallback(() => {
    if (!projectId) {
      setRunning([])
      return
    }
    void ask(() => commands.sessionRunning(projectId)).then((answer) => {
      /* A project with no session yet answers with an empty list rather
         than an error, so there is nothing here to report. */
      if (answer.data) setRunning(answer.data)
    })
  }, [projectId])

  whileWatched(look, EVERY_MS, projectId)

  return running
}
