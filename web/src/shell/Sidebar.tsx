import { useEffect, useState } from 'react'

import { ACCOUNT } from '../mock/data'
import type { PaneName } from './paneList'
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
  const [menu, setMenu] = useState<'new' | 'kit' | 'acct' | null>(null)
  const [synced, setSynced] = useState('Synced 2 minutes ago')

  const open = (name: PaneName): void => {
    shell.show(name)
    setMenu(null)
  }
  const isOpen = (name: PaneName): boolean => shell.open.includes(name)
  const openPrefs = (pane: PrefsPane): void => {
    shell.openPrefs(pane)
    setMenu(null)
  }
  const signOut = (): void => {
    shell.signOut()
    setMenu(null)
  }
  const sync = (): void => {
    setSynced('Syncing…')
    window.setTimeout(() => setSynced('Synced just now'), 900)
  }

  /* A pick is a decision, so the menu goes away with it — and so does a click
     anywhere else, or Escape. */
  useEffect(() => {
    if (!menu) return
    const shut = (): void => setMenu(null)
    const key = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') setMenu(null)
    }
    document.addEventListener('click', shut)
    document.addEventListener('keydown', key)
    return () => {
      document.removeEventListener('click', shut)
      document.removeEventListener('keydown', key)
    }
  }, [menu])

  return (
    <aside className="side">
        <div className="pad10">
          <div className="newmenu">
            <button className="act" aria-haspopup="true" aria-expanded={menu === 'new'} onClick={(event) => { event.stopPropagation(); setMenu(menu === 'new' ? null : 'new') }}>
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

        <div className="side__scroll pad10">
          <div className="searchslot">
            <button className="act" onClick={onSearch}>
              <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg></span>
              <span className="act__label">Search</span>
            </button>
          </div>

          <button className="act" onClick={() => open('board')} aria-pressed={isOpen('board')}>
            <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
            <span className="act__label">Board</span>
          </button>

          <button className="act" onClick={() => open('files')} aria-pressed={isOpen('files')}>
            <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
            <span className="act__label">Files</span>
          </button>

          {/* What the project has to work with: which agents, which servers,
               which plugins. A row each would cost the list half its height for
               a fifth of its use, and the count on each item means the popover
               answers the common question without being opened. */}
          <div className="newmenu">
            <button className="act" aria-haspopup="true" aria-expanded={menu === 'kit'} onClick={(event) => { event.stopPropagation(); setMenu(menu === 'kit' ? null : 'kit') }}>
              <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m12 2 9 5-9 5-9-5Z" /><path d="m3 17 9 5 9-5" /><path d="m3 12 9 5 9-5" /></svg></span>
              <span className="act__label">Resources</span>
              <span className="act__chev"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m7 9 5 5 5-5" /></svg></span>
            </button>
            <div className="newmenu__pop" role="menu" hidden={menu !== 'kit'}>
              <button className="newmenu__item" role="menuitem" onClick={() => open('skills')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                <span className="newmenu__label">Skills</span>
                <span className="newmenu__key">5</span>
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('mcps')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                <span className="newmenu__label">MCPs</span>
                <span className="newmenu__key">2/4</span>
              </button>
              <button className="newmenu__item" role="menuitem" onClick={() => open('caps')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2 4 6v6c0 5 3.4 9.1 8 10 4.6-.9 8-5 8-10V6Z" /><path d="m9 12 2 2 4-4" /></svg></span>
                <span className="newmenu__label">Capabilities</span>
                <span className="newmenu__key">2 on</span>
              </button>
              <div className="newmenu__rule"></div>
              <button className="newmenu__item newmenu__item--live" role="menuitem" onClick={() => open('diagram')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 4h6v6H4zM14 14h6v6h-6z" /><path d="M10 7h4a3 3 0 0 1 3 3v4" /></svg></span>
                <span className="newmenu__label">Diagram</span>
              </button>
              <button className="newmenu__item newmenu__item--live" role="menuitem" onClick={() => open('excalidraw')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3Z" /><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18Z" /><path d="M2 2l7.6 7.6" /><circle cx="11" cy="11" r="2" /></svg></span>
                <span className="newmenu__label">Excalidraw</span>
              </button>
              <div className="newmenu__rule"></div>
              <button className="newmenu__item" role="menuitem" onClick={() => open('workspace')}>
                <span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
                <span className="newmenu__label">Workspace folder</span>
              </button>
            </div>
          </div>

          <div className="navsep"></div>

          <div className="heading">Sessions <span className="heading__n">6</span></div>
          <button className="card" data-ctx="session" onClick={() => open('chat')} aria-pressed={isOpen('chat')}>
            <span className="card__l1"><span className="card__t">Rebuild the shell on GPUI</span><span className="st-work spin"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round"><path d="M21 12a9 9 0 1 1-6.2-8.6" /></svg></span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg></span><span className="card__link"><span className="card__lane">Doing</span></span><span className="card__state card__state--work">Working 3m</span></span>
          </button>
          <button className="card" data-ctx="session" onClick={() => open('term')} aria-pressed={isOpen('term')}>
            <span className="card__l1"><span className="card__t">make test</span><span className="st-work spin"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round"><path d="M21 12a9 9 0 1 1-6.2-8.6" /></svg></span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg></span><span className="card__link"><span className="card__lane">Doing</span><span className="card__task">Rebuild the shell on GPUI</span></span><span className="card__state card__state--work">running</span></span>
          </button>
          <button className="card" data-ctx="session" onClick={() => open('chat')} aria-pressed={isOpen('chat')}>
            <span className="card__l1"><span className="card__t">Board opens in the content</span><span className="st-wait"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 9v4M12 17h.01" /><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" /></svg></span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg></span><span className="card__link"><span className="card__lane">Refine</span></span><span className="card__state card__state--wait">Waiting</span></span>
          </button>
          <button className="card" data-ctx="session" onClick={() => open('chat')} aria-pressed={isOpen('chat')}>
            <span className="card__l1"><span className="card__t">Two read ceilings were guarding nothing</span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg></span><span className="card__link"><span className="card__lane">Check</span></span><span className="card__state card__state--idle">28m</span></span>
          </button>
          <button className="card" data-ctx="session" onClick={() => open('term')} aria-pressed={isOpen('term')}>
            <span className="card__l1"><span className="card__t">cargo watch</span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg></span><span className="card__loose">~/devpit</span><span className="card__state card__state--idle">12m</span></span>
          </button>
          <button className="card" data-ctx="session" onClick={() => open('term')} aria-pressed={isOpen('term')}>
            <span className="card__l1"><span className="card__t">git rebase -i main</span><span className="st-fail"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></span></span>
            <span className="card__l2"><span className="card__kind"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg></span><span className="card__loose">~/devpit</span><span className="card__state card__state--fail">Failed</span></span>
          </button>
        </div>

        <div className="side__foot">
          <button className="signin" onClick={onSignIn}>
            <span className="signin__ico"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg></span>
            <span className="signin__body">
              <span className="signin__t">Sign in</span>
              <span className="signin__d">Save your workspace setup</span>
            </span>
          </button>

          <div className="acct">
            <button className="acct__row" aria-haspopup="true" aria-expanded={menu === 'acct'} onClick={(event) => { event.stopPropagation(); setMenu(menu === 'acct' ? null : 'acct') }}>
              <span className="acct__av">{ACCOUNT.initials}</span>
              <span className="acct__body">
                <span className="acct__n">{ACCOUNT.name}</span>
                <span className="acct__e">{ACCOUNT.email}</span>
              </span>
              <span className="acct__chev"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m7 15 5-5 5 5" /></svg></span>
            </button>
            <div className="acct__pop" role="menu" hidden={menu !== 'acct'}>
              <div className="acct__head">
                <span className="acct__av acct__av--lg">{ACCOUNT.initials}</span>
                <span className="acct__body">
                  <span className="acct__n">{ACCOUNT.name}</span>
                  <span className="acct__e">{ACCOUNT.email}</span>
                </span>
              </div>
              <div className="acct__sync"><span className="acct__dot"></span><span>{synced}</span></div>
            <button className="newmenu__item" role="menuitem" onClick={() => openPrefs('account')}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg></span><span className="newmenu__label">Profile</span></button>
            <button className="newmenu__item" role="menuitem" onClick={() => openPrefs('account')}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M17.5 19a4.5 4.5 0 0 0 .3-9 6 6 0 0 0-11.6 1.6A3.7 3.7 0 0 0 7 19Z" /><path d="M12 12v5M9.5 14.5 12 12l2.5 2.5" /></svg></span><span className="newmenu__label">What's saved</span></button>
            <button className="newmenu__item" role="menuitem" onClick={() => openPrefs('account')}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M2 20h20" /></svg></span><span className="newmenu__label">Devices</span></button>
            <button className="newmenu__item" role="menuitem" onClick={(event) => { event.stopPropagation(); sync() }}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2" /><path d="M3 20v-5h5M21 4v5h-5" /></svg></span><span className="newmenu__label">Sync now</span></button>
              <div className="acct__sep"></div>
            <button className="newmenu__item" role="menuitem" data-danger onClick={signOut}><span className="newmenu__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><path d="m16 17 5-5-5-5M21 12H9" /></svg></span><span className="newmenu__label">Sign out</span></button>
            </div>
          </div>

          <button className="gear" onClick={() => openPrefs('general')} title="Settings" aria-label="Settings"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" /></svg></button>
        </div>
      </aside>
  )
}
