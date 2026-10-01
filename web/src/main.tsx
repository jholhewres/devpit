import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import { answerBeforeRestart } from './shell/beforeRestart'
import { IslandWindow } from './island/IslandWindow'
import { BrowserMenuWindow } from './shell/BrowserMenuWindow'
import { watchCsp } from './shell/csp'
import { reportPresence } from './shell/presence'
import { reportUncaught } from './shell/uncaught'
import './index.css'

/*
 * Two things can be rendered here, and the window says which.
 *
 * A browser pane's menu is a window of its own — it has to be, because the
 * pane's page is a second native webview and two native webviews have no
 * z-order between them, so a menu drawn by the app comes out underneath the
 * site. `browser_menu.rs` builds that window with an initialization script
 * that sets the flag below, before any of this runs.
 *
 * It is the same bundle on purpose. The menu is the app's own component with
 * the app's own styles, and a second entry point would be a second copy of
 * both, drifting.
 */
const isMenu = (window as unknown as { __DEVPIT_MENU__?: boolean }).__DEVPIT_MENU__ === true
// The island is the same kind of window: its own label, the same bundle.
const isIsland = (window as unknown as { __DEVPIT_ISLAND__?: boolean }).__DEVPIT_ISLAND__ === true
const isAside = isMenu || isIsland

// Before anything renders: a policy that blocks something during startup is
// exactly the case nobody can debug from a blank window.
watchCsp()
// Errors nothing else caught, for the report the person may have switched on.
reportUncaught()
// And whether somebody is at it, so those reports go only when nobody is.
if (!isAside) reportPresence()
// And the window answers when an update is about to restart it. Not the menu:
// it holds nothing anybody would lose, and it is closed by the restart anyway.
if (!isAside) answerBeforeRestart()

createRoot(document.getElementById('root')!).render(
  <StrictMode>{isIsland ? <IslandWindow /> : isMenu ? <BrowserMenuWindow /> : <App />}</StrictMode>
)
