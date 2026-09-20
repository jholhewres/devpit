import { useEffect, useState } from 'react'

import type { MenuFor } from '../gen/bindings'
import { BrowserMenu } from './BrowserMenu'
import { VIEWPORTS } from './browsing'
import { ask, commands } from './live'
import { abandoned } from './typing'
import { onCarried } from './window'

/*
 * The whole of the menu window.
 *
 * It exists because of one platform fact: a browser pane's page is a second
 * **native** webview inside the main window, and two native webviews have no
 * z-order between them. Whatever is added last is on top and that has to be
 * the page — so a dropdown drawn by devpit came out behind the site. A window
 * is the one thing the operating system will reliably stack above another.
 *
 * What it costs is this file: the panel is in a different document from the
 * pane it belongs to, so nothing it changes can be a `setState` away. What the
 * pane needs to know goes back through `browser_menu_did`, and what the panel
 * needs to draw itself arrives on `browser:menu-for` before it is shown.
 *
 * **It renders nothing until it has been told what it is for.** The window is
 * made once and reused, so without that it would come back showing the last
 * pane's session with a tick on it — which is the one thing somebody opens
 * this menu to check.
 */

export function BrowserMenuWindow(): React.JSX.Element | null {
  const [showing, setShowing] = useState<MenuFor | null>(null)

  /* Asked once, then listened for.
   *
   * The asking is what makes the *first* opening work. That opening is the one
   * that builds this window, and Rust emits `browser:menu-for` in the same
   * breath — while this document is still loading and has nobody listening.
   * The event went nowhere and the panel stayed empty: the dropdown did not
   * work once, and worked every time after, which is exactly the shape of a
   * listener registered too late.
   *
   * The listening is what makes every opening after it work, when this is
   * already mounted and there is no mount to ask on. */
  useEffect(() => {
    void ask(() => commands.browserMenuShowing()).then((answer) => {
      if (answer.data) setShowing(answer.data)
    })
  }, [])

  useEffect(() => onCarried<MenuFor>('browser:menu-for', setShowing), [])

  /* Escape closes it, the way it closes every other menu in this app. There is
     no click-away handler: the window loses focus when you click elsewhere and
     Rust hides it on that, which also covers clicking another application.
  
     Through the composition guard, because this panel has two text fields in
     it — the session's name and the site to narrow an import to — and an IME
     uses Escape to abandon a half-typed word. Closing the window on that would
     take the word and the menu with it. */
  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) void ask(() => commands.browserMenuHide())
    }
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  }, [])

  if (!showing) return null

  const pane = showing.pane
  const did = (what: Parameters<typeof commands.browserMenuDid>[1]): void => {
    void ask(() => commands.browserMenuDid(pane, what))
  }

  return (
    <div className="bmenuwin">
      <BrowserMenu
        pane={pane}
        at={showing.at}
        session={showing.session}
        viewport={VIEWPORTS.find((one) => one.id === showing.viewport) ?? null}
        granted={showing.granted}
        onSession={(session) => did({ did: 'session', session })}
        /* The id and not the preset: the pane has the same table and looks it
           up there. Sending the numbers would be two copies of them. */
        onViewport={(viewport) => did({ did: 'viewport', viewport: viewport?.id ?? null })}
        onGrant={(granted) => did({ did: 'grant', granted })}
        onSaid={(said) => did({ did: 'said', said })}
        onDone={() => void ask(() => commands.browserMenuHide())}
      />
    </div>
  )
}
