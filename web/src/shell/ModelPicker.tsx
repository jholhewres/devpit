import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { useAway } from './away'
import { modelName } from './chat'
import { committed } from './typing'
import type { Profile } from '../gen/bindings'

/*
 * Which account and model this conversation runs on.
 *
 * One picker for both, because they are one choice: a model belongs to an
 * account, and picking a model from another account would be changing the
 * account. Once the conversation has spoken the account is fixed, and only
 * that account's models are offered.
 *
 * The rail down the left is the account, the list is its models, and the star
 * is a shelf across all of them — the ones you actually reach for, which is
 * rarely the whole catalogue.
 */

const FAVOURITES = 'devpit.favourite-models'

interface Row {
  readonly profile: Profile
  readonly model: string
}

const keyOf = (row: Row): string => `${row.profile.id}:${row.model}`

export function ModelPicker({
  profiles,
  profileId,
  model,
  fixed,
  onPick,
}: {
  profiles: readonly Profile[]
  profileId: string | null
  model: string | null
  /** The account this conversation cannot leave, once it has one. */
  fixed: string | null
  onPick: (profileId: string, model: string) => void
}): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  const [tab, setTab] = useState<string>(profileId ?? '')
  const [stars, setStars] = useState<readonly string[]>(() => {
    try {
      return JSON.parse(localStorage.getItem(FAVOURITES) ?? '[]') as string[]
    } catch {
      return []
    }
  })
  const box = useRef<HTMLDivElement>(null)
  const field = useRef<HTMLInputElement>(null)
  useAway(box, useCallback(() => setOpen(false), []), open)

  const usable = useMemo(
    () => (fixed ? profiles.filter((one) => one.id === fixed) : profiles),
    [fixed, profiles],
  )

  useEffect(() => {
    if (!open) return
    setQuery('')
    setAt(0)
    setTab(profileId ?? usable[0]?.id ?? '')
    field.current?.focus()
  }, [open, profileId, usable])

  const needle = query.trim().toLowerCase()
  /* A search reaches every account; without one, the rail decides. Anything
     else would hide the model you just typed the name of. */
  const searched = needle ? usable : usable.filter((one) => tab === '' || one.id === tab)
  const rows: Row[] = searched.flatMap((profile) =>
    (profile.models ?? [])
      .map((one) => ({ profile, model: one }))
      .filter((row) => {
        if (!needle && tab === '' && !stars.includes(keyOf(row))) return false
        return (
          !needle ||
          `${modelName(row.model)} ${row.model} ${profile.label}`.toLowerCase().includes(needle)
        )
      }),
  )

  const star = (row: Row): void =>
    setStars((was) => {
      const next = was.includes(keyOf(row))
        ? was.filter((one) => one !== keyOf(row))
        : [...was, keyOf(row)]
      try {
        localStorage.setItem(FAVOURITES, JSON.stringify(next))
      } catch {
        /* A favourite that cannot be saved is still a favourite this session. */
      }
      return next
    })

  const choose = (row: Row | undefined): void => {
    if (!row) return
    setOpen(false)
    onPick(row.profile.id, row.model)
  }

  const here = profiles.find((one) => one.id === profileId)
  const label = here ? modelName(model ?? here.models?.[0] ?? '') : 'No agent CLI'

  return (
    <div className="ctl" ref={box}>
      <button
        className="chip"
        aria-label="Account and model"
        aria-expanded={open}
        title={here ? `${here.label} · ${label}` : undefined}
        onClick={() => setOpen((was) => !was)}
      >
        <Mark className="chip__sun" />
        <span className="ctl__l">{label}</span>
      </button>

      {open && (
        <div className="mpick" role="dialog" aria-label="Account and model">
          <div className="mpick__rail">
            <button
              className="mpick__tab"
              aria-label="Favourites"
              aria-pressed={tab === '' && !query}
              data-on={tab === '' && !query}
              onClick={() => {
                setTab('')
                setQuery('')
                setAt(0)
              }}
            >
              <Star on={false} size={15} />
            </button>
            <span className="mpick__sep" />
            {usable.map((profile) => (
              <button
                className="mpick__tab"
                key={profile.id}
                aria-label={profile.label}
                aria-pressed={tab === profile.id && !query}
                data-on={tab === profile.id && !query}
                title={profile.label}
                onClick={() => {
                  setTab(profile.id)
                  setQuery('')
                  setAt(0)
                }}
              >
                {/* The mark says which agent; the letter says which account.
                    Two accounts of one CLI wear the same mark, so the mark
                    alone would be a control that cannot be read. */}
                <Mark />
                <span className="mpick__tabl">{profile.label.slice(0, 1).toUpperCase()}</span>
              </button>
            ))}
          </div>

          <div className="mpick__main">
            <div className="mpick__q">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
              <input
                ref={field}
                className="mpick__in"
                placeholder="Search models…"
                value={query}
                spellCheck={false}
                onChange={(event) => {
                  setQuery(event.target.value)
                  setAt(0)
                }}
                onKeyDown={(event) => {
                  if (event.key === 'ArrowDown') {
                    event.preventDefault()
                    setAt((was) => (was + 1) % Math.max(rows.length, 1))
                  } else if (event.key === 'ArrowUp') {
                    event.preventDefault()
                    setAt((was) => (was - 1 + rows.length) % Math.max(rows.length, 1))
                  } else if (committed(event)) {
                    event.preventDefault()
                    choose(rows[at])
                  }
                }}
              />
            </div>

            <div className="mpick__list">
              {rows.length === 0 && (
                <div className="mpick__none">
                  {query
                    ? 'No model by that name.'
                    : tab === ''
                      ? 'Star a model to keep it here.'
                      : 'No models reported.'}
                </div>
              )}
              {rows.map((row, index) => {
                const mine = row.profile.id === profileId && row.model === model
                const starred = stars.includes(keyOf(row))
                return (
                  <div
                    className="mpick__r"
                    key={keyOf(row)}
                    role="option"
                    aria-selected={mine}
                    data-at={index === at}
                    tabIndex={0}
                    onMouseEnter={() => setAt(index)}
                    onClick={() => choose(row)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault()
                        choose(row)
                      }
                    }}
                  >
                    <span className="mpick__b">
                      <span className="mpick__n">{modelName(row.model)}</span>
                      <span className="mpick__p">
                        <Mark size={10} />
                        {row.profile.label}
                      </span>
                    </span>
                    <span
                      className="mpick__star"
                      role="button"
                      tabIndex={0}
                      aria-label={starred ? 'Remove favourite' : 'Add favourite'}
                      data-on={starred}
                      onClick={(event) => {
                        event.stopPropagation()
                        star(row)
                      }}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault()
                          event.stopPropagation()
                          star(row)
                        }
                      }}
                    >
                      <Star on={starred} size={13} />
                    </span>
                  </div>
                )
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

const Mark = ({ className, size = 12 }: { className?: string; size?: number }): React.JSX.Element => (
  <svg className={className} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" aria-hidden="true">
    <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" />
  </svg>
)

const Star = ({ on, size }: { on: boolean; size: number }): React.JSX.Element => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={on ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="m12 3 2.6 5.6 5.9.8-4.3 4.2 1.1 6-5.3-3-5.3 3 1.1-6L3.5 9.4l5.9-.8Z" />
  </svg>
)
