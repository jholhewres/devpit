import { useCallback, useEffect, useState } from 'react'

import type { ChannelRules, Channels } from '../gen/bindings'
import { clock, EVENTS, flipped, minutes, offsetNow } from './channelRules'
import { ask, commands } from './live'
import { PlusLink } from './PlusLink'
import { TelegramSetup } from './TelegramSetup'

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

/*
 * Where devpit tells you things while you are away from it, and which things.
 * Nothing is sent while devpit's window is in front or devpit is paused.
 */
export function ChannelsPane(): React.JSX.Element {
  const [channels, setChannels] = useState<Channels | null>(null)
  const load = useCallback(() => void ask(() => commands.channelsRead()).then((answer) => setChannels(answer.data)), [])
  useEffect(load, [load])

  const save = (rules: ChannelRules): void => {
    const kept = rules.quiet ? { ...rules, quiet: { ...rules.quiet, offsetMinutes: offsetNow() } } : rules
    void ask(() => commands.channelsRulesSet(kept)).then((answer) => answer.data && setChannels(answer.data))
  }

  const rules = channels?.rules
  const connected = channels?.connected ?? []
  return (
    <>
      <h1 className="prefs__h">Channels</h1>
      <TelegramSetup onLinked={load} />
      <p className="acc__note">
        <PlusLink from="app-channels" said="Push on your phone, e-mail and a devpit bot that works with this computer off come with devpit Plus." />
      </p>

      {rules && connected.length > 0 && (
        <>
          <div className="acc__sub">What goes where</div>
          <table className="chmatrix">
            <thead>
              <tr>
                <th />
                {connected.map((one) => (
                  <th key={one.id}>{one.label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {EVENTS.map(([event, label]) => (
                <tr key={event}>
                  <td>{label}</td>
                  {connected.map((one) => (
                    <td key={one.id}>
                      <input
                        type="checkbox"
                        aria-label={`${label} on ${one.label}`}
                        checked={rules.routes.some((route) => route.event === event && route.channels.includes(one.id))}
                        onChange={(change) => save(flipped(rules, event, one.id, change.target.checked))}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>

          <div className="acc__sub">Quiet hours</div>
          <div className="pref">
            <span className="pref__body">
              <label className="voice__row">
                <input type="checkbox" checked={Boolean(rules.quiet)} onChange={(change) => save({ ...rules, quiet: change.target.checked ? { from: 22 * 60, to: 7 * 60, days: [], offsetMinutes: offsetNow() } : null })} />
                Send nothing during these hours — a reminder you set still goes
              </label>
              {rules.quiet && (
                <span className="voice__row">
                  <input type="time" aria-label="Quiet from" value={clock(rules.quiet.from)} onChange={(change) => rules.quiet && save({ ...rules, quiet: { ...rules.quiet, from: minutes(change.target.value) } })} />
                  to
                  <input type="time" aria-label="Quiet until" value={clock(rules.quiet.to)} onChange={(change) => rules.quiet && save({ ...rules, quiet: { ...rules.quiet, to: minutes(change.target.value) } })} />
                  {DAYS.map((day, at) => (
                    <label key={day} className="chday">
                      <input
                        type="checkbox"
                        checked={rules.quiet?.days.includes(at) ?? false}
                        onChange={(change) =>
                          rules.quiet && save({ ...rules, quiet: { ...rules.quiet, days: change.target.checked ? [...rules.quiet.days, at] : rules.quiet.days.filter((one) => one !== at) } })
                        }
                      />
                      {day}
                    </label>
                  ))}
                </span>
              )}
              <span className="pref__d">No day ticked is every day.</span>
            </span>
          </div>

          <div className="pref">
            <span className="pref__body">
              <span className="pref__t">Gather into one message</span>
              <span className="pref__d">What happens within this long goes as one message: five sessions finishing at once are one notice.</span>
            </span>
            <select value={rules.groupSeconds} onChange={(change) => save({ ...rules, groupSeconds: Number(change.target.value) })}>
              {[15, 60, 120, 300].map((seconds) => (
                <option key={seconds} value={seconds}>
                  {seconds < 60 ? `${seconds} s` : `${seconds / 60} min`}
                </option>
              ))}
            </select>
          </div>
        </>
      )}
    </>
  )
}
