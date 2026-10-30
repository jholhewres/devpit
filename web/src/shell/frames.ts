import type { Attachment, Frame, Message, Part } from '../gen/bindings'

/*
 * A conversation as its frames build it, and what the composer sends into
 * it. Nothing here reaches the app: the window's chat and the Remote's both
 * read a turn this way, and the Remote has no Tauri to reach.
 */

/* What the parts of an open message become as frames arrive. A function, so
   the test calls the rule instead of restating it. */
export function applied(messages: readonly Message[], frame: Frame): readonly Message[] {
  return appliedAll(messages, [frame])
}

/*
 * A batch of frames at once: the list is copied once, not once a frame, and
 * the message a part goes to is found from the end, where the open one is.
 * Text that runs on is gathered and joined once per message, not per piece.
 */
export function appliedAll(messages: readonly Message[], frames: readonly Frame[]): readonly Message[] {
  let out: Message[] | null = null
  const list = (): Message[] => (out ??= [...messages])
  const current = (): readonly Message[] => out ?? messages
  /* Text pieces waiting to be joined, by the index of their message. */
  const pending = new Map<number, string[]>()
  const settle = (at: number): void => {
    const pieces = pending.get(at)
    if (!pieces) return
    pending.delete(at)
    const message = list()[at]!
    const last = message.parts[message.parts.length - 1]
    if (last?.kind !== 'text') return
    list()[at] = { ...message, parts: [...message.parts.slice(0, -1), { ...last, text: last.text + pieces.join('') }] }
  }
  const settleAll = (): void => {
    for (const at of [...pending.keys()]) settle(at)
  }
  for (const frame of frames) {
    switch (frame.type) {
      case 'opened':
        /* A rejoined turn repeats the message the transcript already holds. */
        if (!current().some((one) => one.id === frame.message.id)) list().push(frame.message)
        break
      case 'part': {
        const at = current().findLastIndex((one) => one.id === frame.message_id)
        if (at < 0) break
        const message = current()[at]!
        const last = message.parts[message.parts.length - 1]
        const next = frame.part
        /* Consecutive text from one speaker is one paragraph, not one part per chunk. */
        if (last?.kind === 'text' && next.kind === 'text' && last.parent === next.parent) {
          const pieces = pending.get(at)
          if (pieces) pieces.push(next.text)
          else pending.set(at, [next.text])
        } else {
          settle(at)
          list()[at] = { ...current()[at]!, parts: [...current()[at]!.parts, next] }
        }
        break
      }
      case 'call_state': {
        settleAll()
        /* Only a message still arriving has calls that move: an answered one
           is left as it is, rather than walked on every frame of a long thread. */
        current().forEach((message, at) => {
          if (!message.streaming || !message.parts.some((part) => part.kind === 'tool_call' && part.id === frame.call_id)) return
          list()[at] = {
            ...message,
            parts: message.parts.map((part) =>
              part.kind === 'tool_call' && part.id === frame.call_id ? { ...part, state: frame.state } : part,
            ),
          }
        })
        break
      }
      case 'ended':
        settleAll()
        /* Only the ones that were: a finished message keeps its identity, so
           its memoized turn does not draw again. */
        current().forEach((message, at) => {
          if (message.streaming) list()[at] = { ...message, streaming: false }
        })
        break
      default:
        break
    }
  }
  settleAll()
  return out ?? messages
}

/* An older page goes above what is drawn; a message already drawn is not drawn twice. */
export function prepended(messages: readonly Message[], page: readonly Message[]): readonly Message[] {
  const drawn = new Set(messages.map((one) => one.id))
  return [...page.filter((one) => !drawn.has(one.id)), ...messages]
}

/* A last page in place of what was drawn, keeping the messages a send opened
   while it was read, which the page cannot hold yet. */
export function retaken(page: readonly Message[], drawn: readonly Message[], sent: ReadonlySet<string>): readonly Message[] {
  return appended(page, drawn.filter((one) => sent.has(one.id)))
}

/* Saved messages replace the streamed ones of the same id; new ones go last. */
export function appended(messages: readonly Message[], saved: readonly Message[]): readonly Message[] {
  const by = new Map(saved.map((one) => [one.id, one]))
  const drawn = new Set(messages.map((one) => one.id))
  return [...messages.map((one) => by.get(one.id) ?? one), ...saved.filter((one) => !drawn.has(one.id))]
}

/* The parts as they are drawn: text that runs on is one text, however many
   pieces it was kept in. */
export const joined = (parts: readonly Part[]): Part[] => parts.reduce<Part[]>(merged, [])

/* Consecutive text is one paragraph, not one part per chunk — unless the two
   came from different speakers: a subagent's words are not the agent's. */
function merged(parts: readonly Part[], next: Part): Part[] {
  const last = parts[parts.length - 1]
  if (last?.kind === 'text' && next.kind === 'text' && last.parent === next.parent) {
    return [...parts.slice(0, -1), { kind: 'text', text: last.text + next.text, parent: last.parent }]
  }
  return [...parts, next]
}

/* The prompt the agent actually receives: what was typed, with each
   attachment named as a path it can open. The paths lead rather than trail,
   because the CLI reads them as context for what follows. */
export function withFiles(prompt: string, files: readonly Attachment[]): string {
  if (files.length === 0) return prompt
  return `${files.map((file) => `@${file.path}`).join(' ')}\n${prompt}`
}

/* What a voice message heard adds to what is already typed: after it, on the
   same line, for the person to read before sending. */
export function withHeard(typed: string, heard: string): string {
  return typed.trim() ? `${typed.trimEnd()} ${heard}` : heard
}
