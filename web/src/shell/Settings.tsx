import { useState } from 'react'

import { ACCOUNT } from '../mock/data'
import { ProjectRows } from './ProjectRows'
import { useShell, type PrefsPane } from './useShell'

/* Settings takes the window. Back and Escape leave; the gear and every
   account-menu row land here on the pane they name. */

export function Settings({
  pane,
  onAddProject,
  onRemove,
}: {
  pane: PrefsPane
  onAddProject: () => void
  onRemove: (project: string) => void
}): React.JSX.Element {
  const { closePrefs, openPrefs, theme, setTheme, signOut } = useShell()
  const [synced, setSynced] = useState('Synced 2 minutes ago')

  const sync = (): void => {
    setSynced('Syncing…')
    window.setTimeout(() => setSynced('Synced just now'), 900)
  }

  return (
    <div className="prefs" data-open="true">
      <aside className="prefs__side">
        <button className="prefs__back" onClick={closePrefs}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M19 12H5M12 19l-7-7 7-7" /></svg>Back</button>
        <div className="prefs__find"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>Search settings</div>
        <nav className="prefs__nav">
          <button className="prefs__i" aria-selected={pane === 'account'} onClick={() => openPrefs('account')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg>Account</button>
          <button className="prefs__i" aria-selected={pane === 'projects'} onClick={() => openPrefs('projects')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg>Projects</button>
          <button className="prefs__i" aria-selected={pane === 'general'} onClick={() => openPrefs('general')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.18V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 7.26 19.4l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 3.09 13H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 7.26l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 10 3.09V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 2.74 1.51l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 20.91 11H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" /></svg>General</button>
          <button className="prefs__i" aria-selected={pane === 'appearance'} onClick={() => openPrefs('appearance')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 3v18" /></svg>Appearance</button>
          <button className="prefs__i" aria-selected={pane === 'providers'} onClick={() => openPrefs('providers')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="3" /><path d="m8 10 3 3-3 3M14 16h3" /></svg>Providers</button>
          <button className="prefs__i" aria-selected={pane === 'skills'} onClick={() => openPrefs('skills')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>Skills</button>
          <button className="prefs__i" aria-selected={pane === 'storage'} onClick={() => openPrefs('storage')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><ellipse cx="12" cy="6" rx="8" ry="3" /><path d="M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6" /><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" /></svg>Storage</button>
          <button className="prefs__i" aria-selected={pane === 'usage'} onClick={() => openPrefs('usage')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20V10M10 20V4M16 20v-7M22 20H2" /></svg>Usage</button>
        </nav>
      </aside>
      <div className="prefs__main">
          <section className="prefs__in" hidden={pane !== 'account'}>
            <h1 className="prefs__h">Account</h1>

            <div className="acc__grid">
              <span className="acc__big">{ACCOUNT.initials}</span>
              <div className="acc__fields">
                <label className="fld"><span className="fld__l">Display name</span>
                  <span className="fld__b" contentEditable="true" role="textbox">{ACCOUNT.name}</span></label>
                <label className="fld"><span className="fld__l">Handle</span>
                  <span className="fld__b" contentEditable="true" role="textbox">jholhewres</span></label>
              </div>
              <button className="acc__act">Change photo</button>
            </div>

            <div className="acc__sub">Sign-in</div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.53 2.36 1.09 2.94.83.09-.65.35-1.09.63-1.34-2.22-.25-4.56-1.11-4.56-4.95 0-1.09.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.03a9.5 9.5 0 0 1 5 0c1.91-1.3 2.75-1.03 2.75-1.03.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.85-2.34 4.7-4.57 4.95.36.31.68.92.68 1.85v2.74c0 .26.18.58.69.48A10 10 0 0 0 12 2Z" /></svg><span className="acc__body">
              <span className="acc__t">GitHub</span><span className="acc__d">jholhewres &middot; connected</span></span><span className="acc__tag">Primary</span></div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 8h5a5 5 0 1 1-1.5-3" /></svg><span className="acc__body">
              <span className="acc__t">Google</span><span className="acc__d">Not connected</span></span><button className="acc__act">Connect</button></div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="5" width="18" height="14" rx="2" /><path d="m3 7 9 6 9-6" /></svg><span className="acc__body">
              <span className="acc__t">Email</span><span className="acc__d">{ACCOUNT.email}</span></span><button className="acc__act">Change</button></div>

            <div className="acc__sub">Saved to your account</div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16M15 4v16" /></svg><span className="acc__body">
              <span className="acc__t">Board columns and cards</span><span className="acc__d">Every project's board, as you left it</span></span><span className="acc__tag">On</span></div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 3v6M15 3v6" /><path d="M6 9h12v3a6 6 0 0 1-12 0Z" /><path d="M12 18v3" /></svg><span className="acc__body">
              <span className="acc__t">Capabilities you turned on</span><span className="acc__d">So a new machine starts with the same set</span></span><span className="acc__tag">On</span></div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 3v18" /></svg><span className="acc__body">
              <span className="acc__t">Appearance and shortcuts</span><span className="acc__d">Theme, sidebar widths, key bindings</span></span><span className="acc__tag">On</span></div>
            <p className="acc__note">Projects, conversations, terminal history and files stay on
              this computer. The account saves the workspace around them, not the work itself.</p>
            <div className="acc__row" style={{marginTop: '8px'}}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2" /><path d="M3 20v-5h5M21 4v5h-5" /></svg><span className="acc__body">
              <span className="acc__t">Sync</span><span className="acc__d">{synced}</span></span>
              <button className="acc__act" onClick={sync}>Sync now</button></div>

            <div className="acc__sub">Devices</div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M2 20h20" /></svg><span className="acc__body">
              <span className="acc__t">This computer</span><span className="acc__d">Ubuntu &middot; active now</span></span><span className="acc__tag">Current</span></div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="6" y="3" width="12" height="18" rx="2" /><path d="M10 7h4M12 16v.01" /></svg><span className="acc__body">
              <span className="acc__t">Work desktop</span><span className="acc__d">macOS &middot; 3 days ago</span></span><button className="acc__act">Sign out</button></div>

            <div className="acc__sub">Leaving</div>
            <div className="acc__row"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><path d="m16 17 5-5-5-5M21 12H9" /></svg><span className="acc__body">
              <span className="acc__t">Sign out everywhere</span><span className="acc__d">Ends every session, including this one</span></span><button className="acc__act acc__act--danger" onClick={signOut}>Sign out</button></div>
          </section>

          <section className="prefs__in" hidden={pane !== 'projects'}>
            <div className="prefs__hrow">
              <h1 className="prefs__h">Projects</h1>
              <button className="btn btn--go" onClick={onAddProject}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14" /></svg>Add project</button>
            </div>
        <ProjectRows onRemove={onRemove} />
            <p className="acc__note">Removing a project takes it out of devpit. The folder, the git
              repository and your code are never touched &mdash; devpit only forgets where it was.</p>
          </section>

          <section className="prefs__in" hidden={pane !== 'general'}>
            <h1 className="prefs__h">General</h1>
            <div className="pref">
              <span className="pref__body"><span className="pref__t">Local by default</span><span className="pref__d">Projects, conversations and settings are kept on this computer.</span></span>
            </div>
            <button className="pref" role="switch" aria-checked="false">
              <span className="pref__body"><span className="pref__t">Share anonymous usage data</span><span className="pref__d">Feature use and reliability only. Prompts, replies, project names and file paths are never sent.</span></span>
              <span className="sw"></span>
            </button>
            <button className="pref" role="switch" aria-checked="true">
              <span className="pref__body"><span className="pref__t">Automatic updates</span><span className="pref__d">Check in the background and offer to install.</span></span>
              <span className="sw"></span>
            </button>
          </section>

          <section className="prefs__in" hidden={pane !== 'providers'}>
            <h1 className="prefs__h">Providers</h1>
            <div className="card2">
              <div className="card2__top">
                <div style={{flex: '1', minWidth: '0'}}>
                  <div className="card2__t">Coding agents</div>
                  <div className="card2__d">devpit drives agent CLIs installed on this computer. Install or sign in with each agent&rsquo;s own CLI, then refresh.</div>
                </div>
                <div>
                  <button className="card2__go"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-2.6-6.4" /><path d="M21 3v6h-6" /></svg>Refresh</button>
                  <div className="card2__when">Checked 3h ago</div>
                </div>
              </div>
              <button className="prov" role="switch" aria-checked="true">
                <span className="prov__ico">CC<span className="prov__dot" style={{background: 'var(--success)'}}></span></span>
                <span className="prov__body">
                  <span className="prov__top"><span className="prov__n">Claude Code</span><span className="prov__v">v2.1.263</span></span>
                  <span className="prov__sub">~/.local/bin/claude · 5 models</span>
                </span>
                <span className="sw"></span>
              </button>
              <button className="prov" role="switch" aria-checked="true">
                <span className="prov__ico">CX<span className="prov__dot" style={{background: 'var(--success)'}}></span></span>
                <span className="prov__body">
                  <span className="prov__top"><span className="prov__n">Codex CLI</span><span className="prov__v">v0.149.1</span></span>
                  <span className="prov__sub">~/.local/bin/codex · 4 models</span>
                </span>
                <span className="sw"></span>
              </button>
              <div className="prov prov--off">
                <span className="prov__ico">CU<span className="prov__dot" style={{background: 'var(--ghost)'}}></span></span>
                <span className="prov__body">
                  <span className="prov__top"><span className="prov__n">Cursor CLI</span></span>
                  <span className="prov__sub">Not detected on PATH as cursor-agent</span>
                </span>
                <span className="prov__chev"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span>
              </div>
              <div className="prov prov--off">
                <span className="prov__ico">OC<span className="prov__dot" style={{background: 'var(--ghost)'}}></span></span>
                <span className="prov__body">
                  <span className="prov__top"><span className="prov__n">OpenCode</span></span>
                  <span className="prov__sub">Not detected on PATH as opencode</span>
                </span>
                <span className="prov__chev"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span>
              </div>
              <div className="prov prov--off">
                <span className="prov__ico">AM<span className="prov__dot" style={{background: 'var(--ghost)'}}></span></span>
                <span className="prov__body">
                  <span className="prov__top"><span className="prov__n">Amp</span></span>
                  <span className="prov__sub">Not detected on PATH as amp</span>
                </span>
                <span className="prov__chev"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span>
              </div>
            </div>
          </section>

          <section className="prefs__in" hidden={pane !== 'storage'}>
            <h1 className="prefs__h">Storage</h1>
            <div className="pref">
              <span className="pref__body"><span className="pref__t">Workspace directory</span><span className="pref__d">Each project gets one, holding its board, skills, capabilities, transcripts and worktrees. None of it is in the repository.<br /><code>~/.devpit/workspaces/devpit</code></span></span>
            </div>
            <button className="pref" role="switch" aria-checked="true">
              <span className="pref__body"><span className="pref__t">Keep transcripts</span><span className="pref__d">Every turn is written to disk so a session survives a restart.</span></span>
              <span className="sw"></span>
            </button>
          </section>

          <section className="prefs__in" hidden={pane !== 'appearance'}>
            <h1 className="prefs__h">Appearance</h1>
            <div className="thm" role="radiogroup" aria-label="Theme">
              <button className="thm__c" role="radio" aria-checked={theme === 'system'} onClick={() => setTheme('system')}>
                <span className="thm__p thm__p--sys">
                  <span className="thm__lay thm__p--light"><span className="thm__side"></span><span className="thm__main"><span className="thm__bar"></span><span className="thm__bar thm__bar--s"></span><span className="thm__bar"></span></span></span>
                  <span className="thm__lay thm__p--dark"><span className="thm__side"></span><span className="thm__main"><span className="thm__bar"></span><span className="thm__bar thm__bar--s"></span><span className="thm__bar"></span></span></span>
                </span>
                <span className="thm__l">System</span>
              </button>
              <button className="thm__c" role="radio" aria-checked={theme === 'light'} onClick={() => setTheme('light')}>
                <span className="thm__p thm__p--light"><span className="thm__side"></span><span className="thm__main">
                  <span className="thm__bar"></span><span className="thm__bar thm__bar--s"></span><span className="thm__bar"></span>
                </span></span>
                <span className="thm__l">Light</span>
              </button>
              <button className="thm__c" role="radio" aria-checked={theme === 'dark'} onClick={() => setTheme('dark')}>
                <span className="thm__p thm__p--dark"><span className="thm__side"></span><span className="thm__main">
                  <span className="thm__bar"></span><span className="thm__bar thm__bar--s"></span><span className="thm__bar"></span>
                </span></span>
                <span className="thm__l">Dark</span>
              </button>
            </div>
            <p className="acc__note">The terminal keeps its dark ground in every theme. A shell paints
              its own ANSI colours for a dark background; on a white one they stop being readable.</p>
          </section>

          <section className="prefs__in prefs__in--wide" hidden={pane !== 'skills'}>
            <div className="sk">
              <div className="sk__list">
                <div className="sk__find"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>Search skills…</div>
                <div className="sk__pick"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>All providers<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></div>
                <div className="sk__h">Workspace <span>8</span></div>
                <div className="sk__rows">
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">planner</span><span className="skrow__d">Reads the card, asks what is missing, writes the plan back</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">executor</span><span className="skrow__d">Takes the plan and writes the code in the card&rsquo;s worktree</span></span>
                </button>
                <button className="skrow" aria-current="true">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">reviewer</span><span className="skrow__d">Reads the diff against the plan; reports, never edits</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">test-runner</span><span className="skrow__d">Runs the project&rsquo;s own test command, attaches the output</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">summariser</span><span className="skrow__d">Condenses a long session into what the next one needs</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">brancher</span><span className="skrow__d">Names and opens the worktree a card runs in</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">committer</span><span className="skrow__d">Writes the message from the diff and the card</span></span>
                </button>
                <button className="skrow">
                  <span className="skrow__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <span className="skrow__b"><span className="skrow__n">triager</span><span className="skrow__d">Sorts an inbox card into a lane, or asks why it cannot</span></span>
                </button>
                </div>
                <div className="sk__foot">8 of 8 shown</div>
              </div>

              <div className="sk__doc">
                <div className="sk__top">
                  <span className="sk__mark"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div style={{flex: '1', minWidth: '0'}}>
                    <div className="sk__name">reviewer</div>
                    <div className="sk__from">Claude Code · runs when a card lands in Check</div>
                  </div>
                  <button className="sksw" role="switch" aria-checked="true" aria-label="Enabled">
                    <span className="sw"></span>
                  </button>
                </div>

                <p className="sk__what">Reads the diff a card produced against the plan that card carries, and reports what it cannot verify. It never edits: a reviewer that fixes what it finds has no independent reading left to give.</p>

                <dl className="facts2">
                  <dt>Invoke</dt><dd>/reviewer</dd>
                  <dt>Lane</dt><dd>Check</dd>
                  <dt>Model</dt><dd>claude-opus-5</dd>
                  <dt>Tools</dt><dd>Read, Grep, Bash</dd>
                  <dt>Contents</dt><dd>4.2 kB</dd>
                  <dt>Updated</dt><dd>2 days ago</dd>
                </dl>

                <div className="sk__acts">
              <button className="btn"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></svg>Open SKILL.md</button>
              <button className="btn"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg>Show in the finder</button>
              <button className="btn"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>Copy path</button>
                </div>

                <div className="sk__file">SKILL.md</div>
                <h2>Reviewer</h2>
                <p>Reads a finished card and answers one question: <em>does the diff do what the plan said, and what in it cannot be checked from here?</em></p>

                <h3>When it runs</h3>
                <ul>
                  <li>A card is dropped into <b>Check</b>.</li>
                  <li>Someone asks for it by name in a chat.</li>
                </ul>

                <h3>What it may not do</h3>
                <ul>
                  <li>Edit a file. If the fix is obvious, it says so and stops.</li>
                  <li>Move the card. Passing review is not the same as shipping.</li>
                  <li>Run anything that writes — tests are the test-runner&rsquo;s job.</li>
                </ul>

                <h3>What a report has to contain</h3>
                <ul>
                  <li>Every claim in the plan, marked verified, unverified, or contradicted.</li>
                  <li>For each unverified one, what would settle it.</li>
                  <li>Nothing about style unless the project&rsquo;s own guard would have caught it.</li>
                </ul>

                <p>A review that only says &ldquo;looks good&rdquo; is a review that was not read. If there is genuinely nothing to raise, say which claims were checked and how.</p>
              </div>
            </div>
          </section>
          <section className="prefs__in prefs__in--wide" hidden={pane !== 'usage'}>
            <div className="use">
              <div className="use__top">
                <div>
                  <h1 className="prefs__h" style={{margin: '0'}}>Usage</h1>
                  <div className="use__when">Aug 10 to Sep 8</div>
                </div>
                <div className="seg2">
                  <button aria-selected="true">Daily</button>
                  <button aria-selected="false">Monthly</button>
                  <button aria-selected="false">Projects</button>
                </div>
                <button className="card2__go">Last 30 days <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></button>
              </div>

              <div className="use__grid">
                <div>
                  <div className="use__label">Raw token cost</div>
                  <div className="use__big">$7,211.18<span style={{color: 'var(--text-3)'}}>*</span></div>
                  <div className="use__note">* if billed at the full API rate</div>

                  <div className="bar">
                    <div className="bar__top"><span className="bar__n">Claude Code</span><span className="bar__v">$7,183.70</span></div>
                    <div className="bar__track"><div className="bar__fill" style={{width: '99.6%'}}></div></div>
                    <div className="bar__sub">99.6% of cost · 11.9B tokens</div>
                  </div>
                  <div className="bar">
                    <div className="bar__top"><span className="bar__n">Codex</span><span className="bar__v">$27.48</span></div>
                    <div className="bar__track"><div className="bar__fill" style={{width: '2%', background: 'var(--text-3)'}}></div></div>
                    <div className="bar__sub">0.4% of cost · 44.8M tokens</div>
                  </div>
                </div>

                <div>
                  <div className="use__hrow"><span className="use__h">Daily cost</span></div>
                  <svg width="100%" viewBox="0 0 640 190" role="img" aria-label="Daily cost, Aug 10 to Sep 8" preserveAspectRatio="none" style={{height: '190px'}}>
                    <defs>
                      <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stopColor="var(--accent)" stop-opacity="0.30" />
                        <stop offset="100%" stopColor="var(--accent)" stop-opacity="0.02" />
                      </linearGradient>
                    </defs>
                    <line x1="0" y1="12.0" x2="640" y2="12.0" stroke="var(--border)" strokeWidth="1" />
                    <line x1="0" y1="88.0" x2="640" y2="88.0" stroke="var(--border)" strokeWidth="1" />
                    <line x1="0" y1="164" x2="640" y2="164" stroke="var(--border)" strokeWidth="1" />
                    <path d="M0.0 130.6 L22.1 45.4 L44.1 94.1 L66.2 118.4 L88.3 75.8 L110.3 27.2 L132.4 66.7 L154.5 103.2 L176.6 124.5 L198.6 85.0 L220.7 30.2 L242.8 57.6 L264.8 97.1 L286.9 112.3 L309.0 72.8 L331.0 18.1 L353.1 54.6 L375.2 91.0 L397.2 118.4 L419.3 100.2 L441.4 60.6 L463.4 81.9 L485.5 106.2 L507.6 121.4 L529.7 88.0 L551.7 39.4 L573.8 69.8 L595.9 103.2 L617.9 115.4 L640.0 94.1 L640 164 L0 164 Z" fill="url(#fade)" />
                    <path d="M0.0 130.6 L22.1 45.4 L44.1 94.1 L66.2 118.4 L88.3 75.8 L110.3 27.2 L132.4 66.7 L154.5 103.2 L176.6 124.5 L198.6 85.0 L220.7 30.2 L242.8 57.6 L264.8 97.1 L286.9 112.3 L309.0 72.8 L331.0 18.1 L353.1 54.6 L375.2 91.0 L397.2 118.4 L419.3 100.2 L441.4 60.6 L463.4 81.9 L485.5 106.2 L507.6 121.4 L529.7 88.0 L551.7 39.4 L573.8 69.8 L595.9 103.2 L617.9 115.4 L640.0 94.1" fill="none" stroke="var(--accent)" strokeWidth="1.6" strokeLinejoin="round" />
                    <path d="M0.0 162.5 L22.1 156.4 L44.1 159.4 L66.2 161.0 L88.3 157.9 L110.3 154.9 L132.4 157.9 L154.5 161.0 L176.6 162.5 L198.6 159.4 L220.7 154.9 L242.8 156.4 L264.8 159.4 L286.9 161.0 L309.0 157.9 L331.0 154.9 L353.1 156.4 L375.2 159.4 L397.2 161.0 L419.3 159.4 L441.4 157.9 L463.4 159.4 L485.5 161.0 L507.6 161.0 L529.7 159.4 L551.7 156.4 L573.8 157.9 L595.9 161.0 L617.9 161.0 L640.0 159.4" fill="none" stroke="var(--text-3)" strokeWidth="1.2" strokeLinejoin="round" />
                    <text x="0" y="182" fontSize="11" fill="var(--ghost)" fontFamily="ui-monospace, monospace">Aug 10</text>
                    <text x="320" y="182" fontSize="11" fill="var(--ghost)" textAnchor="middle" fontFamily="ui-monospace, monospace">Aug 25</text>
                    <text x="640" y="182" fontSize="11" fill="var(--ghost)" textAnchor="end" fontFamily="ui-monospace, monospace">Sep 8</text>
                  </svg>
                </div>
              </div>

              <div className="tiles">
              <div className="tile2"><div className="tile2__l">Processed tokens</div><div className="tile2__v">11.9B</div><div className="tile2__s">442M per active day</div></div>
              <div className="tile2"><div className="tile2__l">Cached input</div><div className="tile2__v">11.7B</div><div className="tile2__s">100% of observed input</div></div>
              <div className="tile2"><div className="tile2__l">Uncached input</div><div className="tile2__v">2.08M</div><div className="tile2__s">183M cache writes</div></div>
              <div className="tile2"><div className="tile2__l">Output</div><div className="tile2__v">20.6M</div><div className="tile2__s">includes 62.1K reasoning</div></div>
              <div className="tile2"><div className="tile2__l">Cache savings</div><div className="tile2__v">$52,380</div><div className="tile2__s">7.3&times; the raw token cost</div></div>
              </div>

              <div className="use__cols">
                <div>
                  <div className="use__hrow"><span className="use__h">Breakdown</span>
                    <span className="seg2" style={{marginLeft: 'auto'}}><button aria-selected="true">Model</button><button aria-selected="false">Day</button></span>
                  </div>
                <div className="mrow mrow--h"><span className="mrow__n" style={{color: 'inherit'}}><b style={{background: 'none'}}></b><span style={{fontFamily: 'inherit'}}>Model</span></span><span>Cost</span><span>Share</span><span>Tokens</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-opus-5</span></span><span>$6,906.33</span><span>95.8%</span><span>11.4B</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-fable-5-1</span></span><span>$126.02</span><span>1.7%</span><span>94.8M</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-sonnet-5</span></span><span>$90.55</span><span>1.3%</span><span>297M</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--text-3)'}}></b><span>gpt-5.6-sol</span></span><span>$20.86</span><span>0.3%</span><span>28.6M</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-haiku-4-5</span></span><span>$9.60</span><span>0.1%</span><span>23.6M</span></div>
                <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--text-3)'}}></b><span>gpt-5.6-terra</span></span><span>$6.39</span><span>0.1%</span><span>16M</span></div>
                  <div className="use__scan">Scanned 279 transcripts · 30,327 usage records · 0.9s</div>
                </div>

                <div>
                  <div className="use__hrow"><span className="use__h">Cost quality</span></div>
                  <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Provider reported</span></span><span>0.0%</span></div>
                  <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Model priced</span></span><span>99.9%</span></div>
                  <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Unpriced</span></span><span>0.1%</span></div>
                  <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Cache savings</span></span><span>$52,380</span></div>
                </div>
              </div>
            </div>
          </section>
      </div>
    </div>
  )
}
