import { useEffect, useState } from 'react'

import type { Skill } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The skills this machine has.
 *
 * The eight fixed rows that used to live here named skills nobody had
 * installed. A panel that lists what is not there is worse than an empty one,
 * because an empty one is obviously empty.
 */

const ICON = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>
)

export function SkillsPane(): React.JSX.Element {
  const [skills, setSkills] = useState<readonly Skill[]>([])
  const [problem, setProblem] = useState<string | null>(null)
  const [chosen, setChosen] = useState<string | null>(null)
  const [find, setFind] = useState('')
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    void ask(() => commands.skillsList()).then((answer) => {
      setSkills(answer.data?.skills ?? [])
      setProblem(answer.error ?? answer.data?.problem ?? null)
    })
  }, [])

  const shown = skills.filter(
    (skill) =>
      skill.name.toLowerCase().includes(find.toLowerCase()) ||
      skill.description.toLowerCase().includes(find.toLowerCase()),
  )
  const open = shown.find((skill) => skill.name === chosen) ?? shown[0]

  const copy = (): void => {
    if (!open) return
    void navigator.clipboard?.writeText(open.path).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  }

  return (
    <div className="sk">
      <div className="sk__list">
        <input
          className="sk__find"
          placeholder="Search skills…"
          value={find}
          onChange={(event) => setFind(event.target.value)}
          aria-label="Search skills"
        />
        <div className="sk__h">
          Installed <span>{skills.length}</span>
        </div>
        <div className="sk__rows">
          {shown.map((skill) => (
            <button
              className="skrow"
              key={`${skill.source}/${skill.name}`}
              aria-current={open?.name === skill.name}
              onClick={() => setChosen(skill.name)}
            >
              <span className="skrow__ico">{ICON}</span>
              <span className="skrow__b">
                <span className="skrow__n">{skill.name}</span>
                <span className="skrow__d">{skill.description}</span>
              </span>
            </button>
          ))}
        </div>
        <div className="sk__foot">
          {problem ?? `${shown.length} of ${skills.length} shown`}
        </div>
      </div>

      <div className="sk__doc">
        {open ? (
          <>
            <div className="sk__top">
              <span className="sk__mark">{ICON}</span>
              <div>
                <div className="sk__name">{open.name}</div>
                <div className="sk__from">{open.source}</div>
              </div>
            </div>
            <p className="sk__what">{open.description}</p>
            <div className="sk__acts">
              <button className="btn" onClick={() => void ask(() => commands.pathOpen(open.path))}>
                Open SKILL.md
              </button>
              <button
                className="btn"
                onClick={() => void ask(() => commands.pathReveal(open.path))}
              >
                Show in the finder
              </button>
              <button className="btn" onClick={copy}>
                {copied ? 'Copied' : 'Copy path'}
              </button>
            </div>
            <div className="sk__file">{open.path}</div>
          </>
        ) : (
          <p className="sk__what">{problem ?? 'No skill picked.'}</p>
        )}
      </div>
    </div>
  )
}
