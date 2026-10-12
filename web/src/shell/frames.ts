import type { Attachment, Frame, Message, Part } from '../gen/bindings'

/*
 * A conversation as its frames build it, and what the composer sends into
 * it. Nothing here reaches the app: the window's chat and the Remote's both
 * read a turn this way, and the Remote has no Tauri to reach.
 */

/* What the parts of an open message become as frames arrive. A function, so
   the test calls the rule instead of restating it. */
export function applied(messages: readonly Message[], frame: Frame): readonly Message[] {
  switch (frame.type) {
    case 'opened':
      /* A rejoined turn repeats the message the transcript already holds. */
      return messages.some((one) => one.id === frame.message.id) ? messages : [...messages, frame.message]
    case 'part':
      return messages.map((message) =>
        message.id === frame.message_id
          ? { ...message, parts: merged(message.parts, frame.part) }
          : message,
      )
    case 'call_state':
      /* Only a message still arriving has calls that move: an answered one
         is left as it is, rather than walked on every frame of a long thread. */
      return messages.map((message) => !message.streaming ? message : ({
        ...message,
        parts: message.parts.map((part) =>
          part.kind === 'tool_call' && part.id === frame.call_id
            ? { ...part, state: frame.state }
            : part,
        ),
      }))
    case 'ended':
      /* The turn is over, so nothing is still arriving. */
      /* Only the ones that were: a finished message keeps its identity, so
         its memoized turn does not draw again. */
      return messages.map((message) => (message.streaming ? { ...message, streaming: false } : message))
    default:
      return messages
  }
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
