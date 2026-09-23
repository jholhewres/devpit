import { useEffect, useState } from 'react'

import type { Settings as Stored } from '../gen/bindings'
import { ask, commands } from './live'
import { inOrder } from './inOrder'
import { PrefsSide } from './PrefsSide'
import { OpenApps } from './OpenApps'
import { ProjectRows } from './ProjectRows'
import { useShell, type PrefsPane } from './useShell'
import { ProviderRows } from './ProviderRows'
import { SkillsPane } from './SkillsPane'
import { TerminalContrast } from './TerminalContrast'
import { UpdateSettings } from './UpdateSettings'
import { Usage } from './Usage'
import { Worktrees } from './Worktrees'

/* Settings takes the window. Back and Escape leave; the gear and every
   account-menu row land here on the pane they name. */

/* The yes/no settings, named once. */
type Flag = 'automaticUpdates' | 'confirmStop' | 'focusMode'

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
  const set = (field: Flag, next: boolean): void => {
    void inOrder('settings', () =>
      ask(() =>
        commands.settingsWrite(
          null,
          field === 'automaticUpdates' ? next : null,
          field === 'confirmStop' ? next : null,
          null,
          field === 'focusMode' ? next : null,
        ),
      ),
    ).then((answer) => setFlags(answer.data ?? flags))
  }

  /* Null is "never asked", and two of these default to on: updates, and the
     prompt that stands between somebody and losing what a terminal was doing.
     The focus mode is the exception — it is unfinished, and an unfinished
     thing does not become the default by nobody having an opinion yet. */
  const on = (field: Flag): boolean => flags?.[field] ?? field !== 'focusMode'

  return (
    <div className="prefs" data-open="true">
      <PrefsSide pane={pane} onBack={closePrefs} onSelect={openPrefs} />
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

            {/* What it does today, and nothing about what it might do. A pane
                describing a feature that is not built is a pane people plan
                around. */}
            <p className="acc__note">
              Today the account signs you in on this computer and tells devpit your name and
              address. That is all it does: <b>nothing is uploaded and nothing is synced</b>, and
              sync between machines is not built yet.
            </p>
            <p className="acc__note">
              Projects, boards, conversations, terminal history and files are on this computer.
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
            <button className="pref" role="switch" aria-checked={on('automaticUpdates')} onClick={() => set('automaticUpdates', !on('automaticUpdates'))}>
              <span className="pref__body"><span className="pref__t">Automatic updates</span><span className="pref__d">Check in the background and offer to install.</span></span>
              <span className="sw"></span>
            </button>
            <button className="pref" role="switch" aria-checked={on('focusMode')} onClick={() => set('focusMode', !on('focusMode'))}>
              <span className="pref__body"><span className="pref__t">Focus mode</span><span className="pref__d">Unfinished. A door for one project: what arrives from another waits until you come out.</span></span>
              <span className="sw"></span>
            </button>
            <OpenApps />
            <UpdateSettings />
          </section>

          <section className="prefs__in" hidden={pane !== 'providers'}>
            <h1 className="prefs__h">Providers</h1>
            <ProviderRows />
          </section>

          <section className="prefs__in" hidden={pane !== 'storage'}>
            <h1 className="prefs__h">Storage</h1>
            <div className="pref">
              <span className="pref__body"><span className="pref__t">Workspace directory</span><span className="pref__d">Each project gets one, holding its board, skills, capabilities, transcripts and worktrees. None of it is in the repository.<br /><code>~/.devpit/projects/&lt;name&gt;-&lt;suffix&gt;</code></span></span>
            </div>
            <p className="acc__note">Transcripts are always written: a session that a restart
              loses is a session that was never yours to come back to. Nothing is uploaded, and
              removing a project leaves its folder where it is.</p>
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
            <TerminalContrast
              value={flags?.terminalContrast ?? null}
              onPick={(value) =>
                void inOrder('settings', () => ask(() => commands.settingsWrite(null, null, null, value, null))).then((answer) =>
                  setFlags(answer.data ?? flags),
                )
              }
            />
          </section>

          <section className="prefs__in prefs__in--wide" hidden={pane !== 'skills'}>
            <SkillsPane />
          </section>
          <section className="prefs__in" hidden={pane !== 'usage'}>
            <Usage />
          </section>
      </div>
    </div>
  )
}
