import { useEffect, useState } from 'react'

import type { Settings as Stored } from '../gen/bindings'
import { ask, commands } from './live'
import { ProjectRows } from './ProjectRows'
import { useShell, type PrefsPane } from './useShell'
import { ProviderRows } from './ProviderRows'
import { SkillsPane } from './SkillsPane'
import { Usage } from './Usage'
import { Worktrees } from './Worktrees'

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
  const { closePrefs, openPrefs, theme, setTheme, signOut, account } = useShell()
  const [flags, setFlags] = useState<Stored | null>(null)

  useEffect(() => {
    void ask(() => commands.settingsRead()).then((answer) => setFlags(answer.data))
  }, [])

  /* Each toggle writes only its own field: the command takes null for
     "leave this one alone", so one switch cannot overwrite another. */
  const set = (field: 'automaticUpdates' | 'keepTranscripts' | 'telemetry', next: boolean): void => {
    void ask(() =>
      commands.settingsWrite(
        field === 'telemetry' ? next : null,
        null,
        field === 'automaticUpdates' ? next : null,
        field === 'keepTranscripts' ? next : null,
      ),
    ).then((answer) => setFlags(answer.data ?? flags))
  }

  /* Null is "never asked". Updates and transcripts default to on; sharing
     data defaults to off, because nobody opted into it. */
  const on = (field: 'automaticUpdates' | 'keepTranscripts' | 'telemetry'): boolean =>
    flags?.[field] ?? field !== 'telemetry'

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
          <button className="prefs__i" aria-selected={pane === 'worktrees'} onClick={() => openPrefs('worktrees')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></svg>Worktrees</button>
          <button className="prefs__i" aria-selected={pane === 'usage'} onClick={() => openPrefs('usage')}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20V10M10 20V4M16 20v-7M22 20H2" /></svg>Usage</button>
        </nav>
      </aside>
      <div className="prefs__main">
          <section className="prefs__in" hidden={pane !== 'account'}>
            <h1 className="prefs__h">Account</h1>

            <div className="acc__grid">
              <span className="acc__big">{account.initials}</span>
              <div className="acc__fields">
                <div className="acc__t">{account.name}</div>
                <div className="acc__d">{account.email ?? 'no address yet'}</div>
              </div>
            </div>

            {/* Everything an account does needs a server, and there is not one
                yet. The fields, the connected providers and the device list
                were drawn from nothing; a profile showing a handle nobody
                signed in with is worse than a panel that says "not yet". */}
            <p className="acc__note">
              Accounts are not live yet. When they are, signing in will save the workspace around
              your work &mdash; board columns and cards, what you turned on, appearance and
              shortcuts.
            </p>
            <p className="acc__note">
              Projects, conversations, terminal history and files stay on this computer. The
              account saves the workspace around them, not the work itself.
            </p>

            <div className="acc__sub">Leaving</div>
            <div className="acc__row">
              <span className="acc__body">
                <span className="acc__t">Sign out</span>
                <span className="acc__d">Clears the signed-in state on this computer</span>
              </span>
              <button className="acc__act acc__act--danger" onClick={signOut}>Sign out</button>
            </div>
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
            <button className="pref" role="switch" aria-checked={on('telemetry')} onClick={() => set('telemetry', !on('telemetry'))}>
              <span className="pref__body"><span className="pref__t">Share anonymous usage data</span><span className="pref__d">Feature use and reliability only. Prompts, replies, project names and file paths are never sent.</span></span>
              <span className="sw"></span>
            </button>
            <button className="pref" role="switch" aria-checked={on('automaticUpdates')} onClick={() => set('automaticUpdates', !on('automaticUpdates'))}>
              <span className="pref__body"><span className="pref__t">Automatic updates</span><span className="pref__d">Check in the background and offer to install.</span></span>
              <span className="sw"></span>
            </button>
          </section>

          <section className="prefs__in" hidden={pane !== 'providers'}>
            <h1 className="prefs__h">Providers</h1>
            <ProviderRows />
          </section>

          <section className="prefs__in" hidden={pane !== 'storage'}>
            <h1 className="prefs__h">Storage</h1>
            <div className="pref">
              <span className="pref__body"><span className="pref__t">Workspace directory</span><span className="pref__d">Each project gets one, holding its board, skills, capabilities, transcripts and worktrees. None of it is in the repository.<br /><code>~/.devpit/workspaces/devpit</code></span></span>
            </div>
            <button className="pref" role="switch" aria-checked={on('keepTranscripts')} onClick={() => set('keepTranscripts', !on('keepTranscripts'))}>
              <span className="pref__body"><span className="pref__t">Keep transcripts</span><span className="pref__d">Every turn is written to disk so a session survives a restart. Turning this off leaves the ones already written where they are &mdash; nothing is deleted.</span></span>
              <span className="sw"></span>
            </button>
          </section>

          <section className="prefs__in" hidden={pane !== 'worktrees'}>
            <Worktrees />
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
            <SkillsPane />
          </section>
          <section className="prefs__in prefs__in--wide" hidden={pane !== 'usage'}>
            <Usage />
          </section>
      </div>
    </div>
  )
}
