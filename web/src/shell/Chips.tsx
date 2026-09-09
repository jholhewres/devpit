import { effortName, MODES, modelName } from './chat'
import type { useChat } from './useChat'

/*
 * What this turn will run as: which account, which model, how hard it thinks,
 * and what it may do without asking.
 *
 * Together because they are one answer, and because two of them disappear —
 * a selector with a single option in it is a control that cannot be used.
 */

export function Chips({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element {
  return (
    <>
      <Account chat={chat} />
      <Model chat={chat} />
      <Effort chat={chat} />
      <select
        className="chip"
        value={chat.permission}
        onChange={(event) => chat.setPermission(event.target.value)}
        aria-label="Permission"
        title={MODES.find((mode) => mode.id === chat.permission)?.what}
      >
        {MODES.map((mode) => (
          <option key={mode.id} value={mode.id} title={mode.what}>
            {mode.label} — {mode.what}
          </option>
        ))}
      </select>
    </>
  )
}

/* The account, once and for good. After the first turn it reads rather than
   asks: one transcript belongs to one account. */
function Account({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element {
  if (chat.fixed) {
    const label = chat.profiles.find((profile) => profile.id === chat.fixed)?.label ?? chat.fixed
    return (
      <span className="chip" title="A conversation keeps the account it started on">
        {label}
      </span>
    )
  }
  if (chat.profiles.length === 0) {
    return <span className="chip">No agent CLI on the PATH</span>
  }
  return (
    <select
      className="chip"
      value={chat.profileId ?? ''}
      onChange={(event) => chat.pick(event.target.value)}
      aria-label="Account"
    >
      <option value="" disabled>
        Account
      </option>
      {chat.profiles.map((profile) => (
        <option key={profile.id} value={profile.id}>
          {profile.label}
        </option>
      ))}
    </select>
  )
}

function Model({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element | null {
  const models = chat.profiles.find((profile) => profile.id === chat.profileId)?.models ?? []
  if (models.length === 0) return null
  return (
    <select
      className="chip"
      value={chat.model ?? models[0]}
      onChange={(event) => chat.setModel(event.target.value)}
      aria-label="Model"
    >
      {models.map((model) => (
        <option key={model} value={model}>
          {modelName(model)}
        </option>
      ))}
    </select>
  )
}

/* No chip at all when the CLI has no such control: a selector with one
   option in it is a control that cannot be used. */
function Effort({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element | null {
  const efforts = chat.profiles.find((profile) => profile.id === chat.profileId)?.efforts ?? []
  if (efforts.length === 0) return null
  return (
    <select
      className="chip"
      value={chat.effort ?? efforts[0]}
      onChange={(event) => chat.setEffort(event.target.value)}
      aria-label="Reasoning"
    >
      {efforts.map((effort) => (
        <option key={effort} value={effort}>
          {effortName(effort)}
        </option>
      ))}
    </select>
  )
}
