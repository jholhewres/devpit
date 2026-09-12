import { useCallback, useEffect, useRef, useState } from 'react'

import type { KnownApp, OpenApp } from '../gen/bindings'
import { useAway } from './away'
import { ask, commands } from './live'

/*
 * The apps a folder can be handed to.
 *
 * `path.open` asks the desktop what opens a folder and the desktop says "a
 * file manager", which is never what someone meant by opening a project. So
 * this is a short list they keep.
 *
 * One column of it is measured rather than stored: whether the command is
 * actually on this machine. Asked of the shell and not of `PATH`, because an
 * editor launched by a shell function has no file on `PATH` and would be
 * greyed out for no reason.
 */

export function OpenApps(): React.JSX.Element {
  const [apps, setApps] = useState<readonly OpenApp[]>([])
  const [known, setKnown] = useState<readonly KnownApp[]>([])
  const [adding, setAdding] = useState(false)
  const [custom, setCustom] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const menu = useRef<HTMLDivElement>(null)

  const load = useCallback(() => {
    void ask(() => commands.appsList()).then((answer) => setApps(answer.data ?? []))
    void ask(() => commands.appsKnown()).then((answer) => setKnown(answer.data ?? []))
  }, [])

  useEffect(load, [load])
  useAway(menu, () => setAdding(false), adding)

  const add = (label: string, command: string): void => {
    void ask(() => commands.appsAdd(label, command)).then((answer) => {
      setProblem(answer.error)
      if (!answer.data) return
      setApps(answer.data)
      setAdding(false)
      setCustom(false)
    })
  }

  const remove = (id: string): void => {
    void ask(() => commands.appsRemove(id)).then((answer) => {
      setProblem(answer.error)
      if (answer.data) setApps(answer.data)
    })
  }

  const has = (command: string): boolean => apps.some((app) => app.command === command)

  return (
    <section className="apps">
      <div className="prefs__hrow">
        <div>
          <span className="pref__t">Open in</span>
          <span className="pref__d">
            The apps offered when you open a project or a worktree somewhere else.
          </span>
        </div>
        <div className="apps__add" ref={menu}>
          <button className="btn" onClick={() => setAdding((was) => !was)}>
            Add app
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg>
          </button>
          {adding && (
            <div className="apps__menu" role="menu">
              {known.map((one) => (
                <button
                  className="apps__opt"
                  key={one.id}
                  role="menuitem"
                  disabled={has(one.command)}
                  onClick={() => add(one.label, one.command)}
                >
                  <span>{one.label}</span>
                  {has(one.command) && <span className="apps__had">Added</span>}
                </button>
              ))}
              <button className="apps__opt" role="menuitem" onClick={() => { setCustom(true); setAdding(false) }}>
                Custom app&hellip;
              </button>
            </div>
          )}
        </div>
      </div>

      {custom && <Custom onCancel={() => setCustom(false)} onAdd={add} />}

      {apps.length === 0 && !custom && (
        <p className="acc__note">No apps yet. Add one and it appears in every project&rsquo;s menu.</p>
      )}

      {apps.map((app) => (
        <div className="oapp" key={app.id}>
          <span className="oapp__b">
            <span className="oapp__t">{app.label}</span>
            <span className="oapp__c">{app.command}</span>
          </span>
          {/* Measured, not assumed: the row says so rather than failing when
              it is clicked, which is the moment it would be most confusing. */}
          {!app.installed && <span className="oapp__no">not on this machine</span>}
          <button className="btn" data-danger onClick={() => remove(app.id)}>
            Remove
          </button>
        </div>
      ))}

      {problem && <p className="wtb__no">{problem}</p>}
    </section>
  )
}

/* A name and a program. Two fields because they are two facts: the menu shows
   one and runs the other, and guessing either from the other gets it wrong for
   `subl` and `idea` alike. */
function Custom({
  onAdd,
  onCancel,
}: {
  onAdd: (label: string, command: string) => void
  onCancel: () => void
}): React.JSX.Element {
  const [label, setLabel] = useState('')
  const [command, setCommand] = useState('')

  return (
    <div className="oapp oapp--new">
      <label className="fld">
        <span className="fld__l">Name</span>
        <input
          className="fld__b"
          autoFocus
          value={label}
          placeholder="My editor"
          onChange={(event) => setLabel(event.target.value)}
        />
      </label>
      <label className="fld">
        <span className="fld__l">Terminal command</span>
        <input
          className="fld__b"
          value={command}
          spellCheck={false}
          placeholder="code"
          onChange={(event) => setCommand(event.target.value)}
        />
      </label>
      <div className="ask__row">
        <button className="btn" onClick={onCancel}>Cancel</button>
        <button
          className="btn btn--go"
          disabled={!label.trim() || !command.trim()}
          onClick={() => onAdd(label, command)}
        >
          Add
        </button>
      </div>
      <p className="pref__d">The command you would type in a terminal to open it. One program, no arguments.</p>
    </div>
  )
}
