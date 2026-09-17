import { useCallback, useRef, useState } from 'react'

import { useAway } from './away'
import { CapabilityRows } from './CapabilityRows'
import type { PaneName } from './paneList'
import { SessionRows } from './SessionRowsView'
import { Threads } from './Threads'
import { useKit } from './useKit'
import { useShell, type PrefsPane } from './useShell'

/*
 * The left column: what you start, what is running, and who you are.
 *
 * The sidebar opens things; it never closes them. Clicking a row you are
 * already in should land you there, not toggle it away under you.
 */

export function Sidebar({
  onSearch,
  onSignIn,
}: {
  onSearch: () => void
  onSignIn: () => void
}): React.JSX.Element {
  const shell = useShell()
  const account = shell.account
  const kit = useKit()
  const [menu, setMenu] = useState<'new' | 'kit' | 'acct' | null>(null)
  const newBox = useRef<HTMLDivElement>(null)
  const kitBox = useRef<HTMLDivElement>(null)
  const acctBox = useRef<HTMLDivElement>(null)

  const open = (kind: PaneName): void => {
    shell.show(kind)
    setMenu(null)
  }
  /* What you are looking at, not what exists. Marking every open pane lit
     Board and Files at once, which reads as two places being current — and
     the tab strip already says what is open. */
  const showing = (kind: PaneName): boolean => shell.active?.kind === kind
  const openPrefs = (pane: PrefsPane): void => {
    shell.openPrefs(pane)
    setMenu(null)
  }
  const signOut = (): void => {
    shell.signOut()
    setMenu(null)
  }
  /* A pick is a decision, so the menu goes away with it — and so does a click
     anywhere else, or Escape. One hook per menu, because each closes on a
     click outside its own box and the trigger lives inside it. */
  const shut = useCallback(() => setMenu(null), [])
  useAway(newBox, shut, menu === 'new')
  useAway(kitBox, shut, menu === 'kit')
  useAway(acctBox, shut, menu === 'acct')

  return (
    <aside className="side">
        <div className="gutter">
          <div className="newmenu" ref={newBox}>
            <button className="act" aria-haspopup="true" aria-expanded={menu === 'new'} onClick={() => setMenu(menu === 'new' ? null : 'new')}>
              <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14" /></svg></span>
              <span className="act__label">New Task</span>
            </button>
            <div className="newmenu__pop" role="menu" hidden={menu !== 'new'}>
              <button className="newmenu__item" role="menuitem" onClick={() => open('chat')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg></span>
                <span className="newmenu__label">Chat</span>
                <span className="newmenu__key">&#8984;N</span>
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('term')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg></span>
                <span className="newmenu__label">Terminal</span>
                <span className="newmenu__key">&#8984;T</span>
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('board')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
                <span className="newmenu__label">Card on the board</span>
                <span className="newmenu__key">&#8984;&#8679;N</span>
              </button>
            </div>
          </div>
        </div>

        <div className="side__scroll gutter">
          <div className="searchslot">
            <button className="act" onClick={onSearch}>
              <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg></span>
              <span className="act__label">Search</span>
            </button>
          </div>

          <button className="act" onClick={() => open('board')} aria-pressed={showing('board')}>
            <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
            <span className="act__label">Board</span>
          </button>

          <button className="act" onClick={() => open('files')} aria-pressed={showing('files')}>
            <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
            <span className="act__label">Files</span>
          </button>

          {/* Resources stays last: it is a popover, and everything above it is a place. */}
          <CapabilityRows />

          {/* What the project has to work with: which skills, which servers.
               A row each would cost the list half its height for a fifth of
               its use, and the count on each item means the popover answers
               the common question without being opened. */}
          <div className="newmenu" ref={kitBox}>
            <button className="act" aria-haspopup="true" aria-expanded={menu === 'kit'} onClick={() => setMenu(menu === 'kit' ? null : 'kit')}>
              <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m12 2 9 5-9 5-9-5Z" /><path d="m3 17 9 5 9-5" /><path d="m3 12 9 5 9-5" /></svg></span>
              <span className="act__label">Resources</span>
              <span className="act__chev"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m7 9 5 5 5-5" /></svg></span>
            </button>
            <div className="newmenu__pop" role="menu" hidden={menu !== 'kit'}>
              <button className="newmenu__item" role="menuitem" onClick={() => open('skills')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                <span className="newmenu__label">Skills</span>
                {kit.skills !== null && <span className="newmenu__n">{kit.skills}</span>}
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('mcps')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                <span className="newmenu__label">MCPs</span>
                {kit.servers !== null && <span className="newmenu__n">{kit.servers}</span>}
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('workspace')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
                <span className="newmenu__label">Workspace folder</span>
              </button>
            </div>
          </div>

          <div className="navsep"></div>

          <SessionRows />

          <Threads />
        </div>

        <div className="side__foot">
          <button className="signin" onClick={onSignIn}>
            <span className="signin__ico"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg></span>
            <span className="signin__body">
              <span className="signin__t">Sign in</span>
              <span className="signin__d">Optional — nothing is synced yet</span>
            </span>
          </button>

          <div className="acct" ref={acctBox}>
            <button className="acct__row" aria-haspopup="true" aria-expanded={menu === 'acct'} onClick={() => setMenu(menu === 'acct' ? null : 'acct')}>
              <span className="acct__av">{account.initials}</span>
              <span className="acct__body">
                <span className="acct__n">{account.name}</span>
                <span className="acct__e">{account.email ?? 'no address yet'}</span>
              </span>
              <span className="acct__chev"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m7 15 5-5 5 5" /></svg></span>
            </button>
            <div className="acct__pop" role="menu" hidden={menu !== 'acct'}>
              <div className="acct__head">
                <span className="acct__av acct__av--lg">{account.initials}</span>
                <span className="acct__body">
                  <span className="acct__n">{account.name}</span>
                  <span className="acct__e">{account.email ?? 'no address yet'}</span>
                </span>
              </div>
              {/* One item, because Profile, What's saved and Devices were
                   three names for the same preferences pane. */}
              <button className="newmenu__item" role="menuitem" onClick={() => openPrefs('account')}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg></span><span className="newmenu__label">Account</span></button>
              <div className="acct__sep"></div>
            <button className="newmenu__item" role="menuitem" data-danger onClick={signOut}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><path d="m16 17 5-5-5-5M21 12H9" /></svg></span><span className="newmenu__label">Sign out</span></button>
            </div>
          </div>

          <button className="gear" onClick={() => openPrefs('general')} title="Settings" aria-label="Settings"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" /></svg></button>
        </div>
      </aside>
  )
}
