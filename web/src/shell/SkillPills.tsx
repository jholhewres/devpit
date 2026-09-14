import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { added, removed } from './pills'

/*
 * The skills picked for this turn, as pills beside the attachments, and the
 * picker that adds them.
 *
 * The picker lists the default installation's skills, the same catalogue the
 * Skills panel shows — a skill that is not installed is not offered.
 */

export function SkillPills({
  picked,
  onChange,
}: {
  picked: readonly string[]
  onChange: (next: readonly string[]) => void
}): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const [installed, setInstalled] = useState<readonly string[]>([])

  useEffect(() => {
    if (!open || installed.length > 0) return
    void ask(() => commands.skillsList(null)).then((answer) =>
      setInstalled((answer.data?.skills ?? []).map((skill) => skill.name)),
    )
  }, [open, installed.length])

  return (
    <div className="pills">
      {picked.map((skill) => (
        <button key={skill} className="chip pills__p" onClick={() => onChange(removed(picked, skill))} title="Remove this skill">
          /{skill} ✕
        </button>
      ))}
      <button className="chip" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        + Skill
      </button>
      {open && (
        <div className="pills__menu" role="listbox" aria-label="Skills">
          {installed.map((skill) => (
            <button
              key={skill}
              className="pills__o"
              role="option"
              aria-selected={picked.includes(skill)}
              onClick={() => {
                onChange(added(picked, skill))
                setOpen(false)
              }}
            >
              {skill}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
