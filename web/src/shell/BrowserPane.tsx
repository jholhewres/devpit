import { useCallback, useEffect, useRef, useState } from 'react'

import { BrowserSignIn } from './BrowserSignIn'
import { aim, boxOf, moved, shown, type Where } from './browsing'
import type { Drove, Showing } from '../gen/bindings'
import { ask, commands } from './live'
import type { Tab } from './strip'
import { useShell } from './useShell'
import { onCarried } from './window'

/*
 * A page, in a webview that is not ours.
 *
 * What this component draws is a **hole**. The page itself is a second native
 * webview inside the same window, placed over this element in window
 * coordinates — it is not an iframe and it is not inside this document, which
 * is the whole point: the page cannot reach what devpit draws, and devpit's
 * commands are scoped to the `main` webview so the page cannot call them
 * either.
 *
 * Two consequences the rest of the file is about:
 *
 * 1. **Nothing moves it but us.** A native webview knows nothing about the
 *    layout that decided where it goes, so every resize and every scroll has
 *    to be measured here and sent over.
 * 2. **Nothing hides it but us.** It floats above the window. A pane that
 *    scrolls out of view, or a tab that stops being the visible one, leaves a
 *    page sitting over whatever is there now unless this closes it.
 */

export function BrowserPane({ tab }: { tab: Tab }): React.JSX.Element {
  const { rename } = useShell()
  const box = useRef<HTMLDivElement | null>(null)
  const placed = useRef<Where | null>(null)
  const open = useRef(false)

  const [typed, setTyped] = useState('')
  const [at, setAt] = useState('')
  const [refused, setRefused] = useState<string | null>(null)
  const [signingIn, setSigningIn] = useState(false)
  /* Which session this pane is in, visible rather than buried in settings:
     the failure it prevents is somebody signed into the wrong account and
     having no way to see why. */
  const [session] = useState('default')
  /* Off, and only a person turns it on. An agent that asks about a pane it
     was not given is told no, so the refusal reaches its transcript. */
  const [granted, setGranted] = useState(false)
  const [drove, setDrove] = useState<string | null>(null)

  /* Measured, not derived: see the note above.
   *
   * And it does one more thing than it looks like. A pane that is not the
   * active tab is hidden with `display: none` (`Panes.tsx`, `data-show`), so
   * this component **never unmounts** and the cleanup below never runs. The
   * page would go on living as a 1×1 webview at the window's corner — still
   * loaded, still running script, still drivable by an agent. A hidden pane
   * measures 0×0, and that is the signal to close it; coming back measures
   * real again, and the url is reopened. */
  const place = useCallback((): void => {
    const node = box.current
    if (!node) return
    const now = boxOf(node.getBoundingClientRect())
    const shown = now.width > 0 && now.height > 0

    if (!shown) {
      if (open.current) {
        open.current = false
        placed.current = null
        void ask(() => commands.browserClose(tab.id))
      }
      return
    }
    if (!open.current) {
      /* Back on screen: the page comes back where it was. */
      if (!at) return
      placed.current = now
      void ask(() => commands.browserOpen(tab.id, at, now, session)).then((answer) => {
        if (answer.error === null) open.current = true
      })
      return
    }
    if (!moved(placed.current, now)) return
    placed.current = now
    void ask(() => commands.browserPlace(tab.id, now))
  }, [at, session, tab.id])

  const go = useCallback(
    (where: string): void => {
      const aimed = aim(where)
      if ('refused' in aimed) {
        setRefused(aimed.refused)
        return
      }
      const node = box.current
      if (!node) return
      setRefused(null)
      const now = boxOf(node.getBoundingClientRect())
      placed.current = now
      void ask(() => commands.browserOpen(tab.id, aimed.at, now, session)).then((answer) => {
        if (answer.error !== null) {
          /* A page that will not load says so here rather than leaving a blank
             rectangle, which is what a person reads as the app being broken. */
          setRefused(answer.error)
          return
        }
        open.current = true
        setAt(aimed.at)
        setTyped(shown(aimed.at))
      })
    },
    [session, tab.id],
  )

  /* Where the page actually is. A link, a redirect or a form leaves the last
     url devpit asked for saying where the page was *sent*, not where it *is* —
     so the bar follows the webview rather than the request. */
  useEffect(
    () =>
      onCarried<Showing>('browser:showing', (said) => {
        if (said.pane !== tab.id) return
        setAt(said.url)
        setTyped(shown(said.url))
        setRefused(null)
        /* The page names its own tab once it has a name. A page that calls
           itself nothing leaves the tab on the address, which is what a
           browser has always done. */
        rename(tab.id, said.title.trim() || shown(said.url))
      }),
    [rename, tab.id],
  )

  /* What an agent is doing to this page, drawn as it happens rather than
     reported once the page has already changed. */
  useEffect(
    () =>
      onCarried<Drove>('browser:driven', (said) => {
        if (said.pane !== tab.id) return
        setDrove(said.said)
      }),
    [tab.id],
  )

  /* The webview follows the pane, and goes away with it. */
  useEffect(() => {
    const node = box.current
    if (!node) return undefined

    const watchers: Array<{ disconnect: () => void }> = []
    if (typeof ResizeObserver !== 'undefined') {
      const size = new ResizeObserver(() => place())
      size.observe(node)
      watchers.push(size)
    }
    /* `display: none` is not a resize on every engine, so the attribute that
       causes it is watched directly. Without this a tab switch leaves the
       page running behind whatever is on screen. */
    if (typeof MutationObserver !== 'undefined' && node.parentElement) {
      const shown = new MutationObserver(() => place())
      shown.observe(node.parentElement, { attributes: true, attributeFilter: ['data-show'] })
      watchers.push(shown)
    }
    /* Scrolling moves the hole without resizing it, and a window resize moves
       every hole at once. Neither reaches a ResizeObserver on this element. */
    window.addEventListener('resize', place)
    window.addEventListener('scroll', place, true)

    return () => {
      window.removeEventListener('resize', place)
      window.removeEventListener('scroll', place, true)
      for (const watcher of watchers) watcher.disconnect()
      /* Unconditionally: a webview left behind floats over whatever the window
         shows next, and nothing else in the app knows it is there. */
      open.current = false
      placed.current = null
      void ask(() => commands.browserClose(tab.id))
    }
  }, [place, tab.id])

  return (
    <div className="browser">
      <div className="browser__bar">
        <button
          type="button"
          className="browser__act"
          aria-label="Back"
          disabled={!at}
          onClick={() => void ask(() => commands.browserBack(tab.id))}
        >
          ‹
        </button>
        <button
          type="button"
          className="browser__act"
          aria-label="Forward"
          disabled={!at}
          onClick={() => void ask(() => commands.browserForward(tab.id))}
        >
          ›
        </button>
        <button
          type="button"
          className="browser__act"
          aria-label="Reload"
          disabled={!at}
          onClick={() => at && go(at)}
        >
          ⟳
        </button>
        <button
          type="button"
          className="browser__act"
          aria-label="Stop"
          disabled={!at}
          onClick={() => void ask(() => commands.browserStop(tab.id))}
        >
          ✕
        </button>
        <form
          className="browser__where"
          onSubmit={(event) => {
            event.preventDefault()
            go(typed)
          }}
        >
          <input
            className="browser__address"
            aria-label="Address"
            placeholder="localhost:3000"
            spellCheck={false}
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
        </form>
        {/* A page at its login screen is the one thing that makes a browser
            pane useless. This is the way out of it. */}
        <button
          type="button"
          className="browser__act"
          aria-label="Bring a signed-in session"
          aria-pressed={signingIn}
          onClick={() => setSigningIn((was) => !was)}
        >
          {/* A key, drawn rather than typed: U+26BF renders as an empty box
              in the fonts this app ships with, which the e2e screenshot showed
              and no assertion would have. */}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="8" cy="12" r="4" />
            <path d="M12 12h9M18 12v4M15 12v3" />
          </svg>
        </button>
        <span className="browser__session" title="The session this pane uses">
          {session}
        </span>
        <label className="browser__grant" title="Let an agent drive this page">
          <input
            type="checkbox"
            aria-label="Let an agent drive this page"
            checked={granted}
            onChange={(event) => {
              const may = event.target.checked
              setGranted(may)
              void ask(() => commands.browserGrant(tab.id, may))
              if (!may) setDrove(null)
            }}
          />
          Agent
        </label>
      </div>
      {/* The hole. The page is drawn over this by the window, not by React. */}
      <div className="browser__page" ref={box} />
      {signingIn && <BrowserSignIn pane={tab.id} at={at} session={session} />}
      {granted && drove && (
        <div className="browser__drove" role="status">
          The agent {drove}
        </div>
      )}
      {refused && (
        <div className="browser__said" role="status">
          {refused}
        </div>
      )}
    </div>
  )
}
