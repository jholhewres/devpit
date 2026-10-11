import { useEffect } from 'react'

/*
 * Asking the app things on a timer, and only while somebody is looking.
 *
 * Two hooks ask the backend on a clock: what each pane has in the foreground
 * (every 2 s, through tmux) and what the terminals cost (every 3 s, which
 * walks the page tables of every process under every pane). Neither stopped
 * when the window was hidden, so a minimised devpit went on asking the kernel
 * both questions for nobody.
 */

/** Whether a timer has any reason to fire. */
export const shouldPoll = (hidden: boolean, projectId: string | null, watching = true): boolean =>
  !hidden && watching && projectId !== null

/**
 * Runs `look` now and every `every` ms, but only while the window is visible.
 *
 * On coming back it asks at once rather than waiting out the interval: a
 * window restored to a screen showing what was true a minute ago is the same
 * staleness the timer exists to avoid.
 *
 * A tick is skipped while the last answer is still awaited. Both questions go
 * through tmux, and a tmux server buried under flooding panes answers nobody
 * for minutes: asked again every two seconds regardless, the questions piled
 * up one waiting thread each until no command in the app had a thread left.
 */
export function whileWatched(
  look: () => void | Promise<unknown>,
  every: number,
  projectId: string | null,
  watching = true,
): void {
  useEffect(() => {
    const hidden = (): boolean => typeof document !== 'undefined' && document.hidden
    let timer: number | null = null
    let asking = false
    const free = (): void => {
      asking = false
    }
    const ask = (): void => {
      if (asking) return
      const answer = look()
      if (!answer) return
      asking = true
      void answer.then(free, free)
    }

    const stop = (): void => {
      if (timer !== null) window.clearInterval(timer)
      timer = null
    }

    const start = (): void => {
      stop()
      if (!shouldPoll(hidden(), projectId, watching)) return
      ask()
      timer = window.setInterval(ask, every)
    }

    start()
    document.addEventListener('visibilitychange', start)
    return () => {
      stop()
      document.removeEventListener('visibilitychange', start)
    }
  }, [look, every, projectId, watching])
}
