import { useEffect, useState } from 'react'

import type { Settings as Stored } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefsSide } from './PrefsSide'
import { OpenApps } from './OpenApps'
import { ProjectRows } from './ProjectRows'
import { useShell, type PrefsPane } from './useShell'
import { ProviderRows } from './ProviderRows'
import { SkillsPane } from './SkillsPane'
import { Usage } from './Usage'
import { Worktrees } from './Worktrees'

/* Settings takes the window. Back and Escape leave; the gear and every
   account-menu row land here on the pane they name. */

/* The yes/no settings, named once. */
type Flag = 'automaticUpdates' | 'keepTranscripts' | 'telemetry' | 'confirmStop'

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
    void ask(() =>
      commands.settingsWrite(
        field === 'telemetry' ? next : null,
        null,
        field === 'automaticUpdates' ? next : null,
        field === 'keepTranscripts' ? next : null,
        field === 'confirmStop' ? next : null,
      ),
    ).then((answer) => setFlags(answer.data ?? flags))
  }

  /* Null is "never asked". Updates, transcripts and the close prompt default
     to on; sharing data defaults to off, because nobody opted into it. */
  const on = (field: Flag): boolean => flags?.[field] ?? field !== 'telemetry'

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

            {/* The name and the address are read from the accounts server on
                launch. Sync is not live yet, and this says so rather than
                showing a switch that does nothing. */}
            <p className="acc__note">
              Sync is not live yet. When it is, the account will save the workspace around your
              work &mdash; board columns and cards, what you turned on, appearance and shortcuts.
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
            <OpenApps />
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
