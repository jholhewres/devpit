import { Channel, invoke } from '@tauri-apps/api/core'

import type {
  Ask,
  Attachment,
  Conversation,
  Frame,
  Message,
  Part,
  Profile,
  TurnEnd,
} from '../gen/bindings'
import { inTauri } from './window'

/* chat.send streams over a Channel, like the pty, so it is hand-written for
   the same reason: specta cannot describe the channel type. */
export interface Sent {
  readonly end: Promise<TurnEnd>
}

export function send(ask: Ask, onFrame: (frame: Frame) => void): Sent | null {
  if (!inTauri()) return null
  const channel = new Channel<Frame>()
  channel.onmessage = onFrame
  return { end: invoke('chat_send', { ask, onFrame: channel }) as Promise<TurnEnd> }
}

/* What the parts of an open message become as frames arrive. A function, so
   the test calls the rule instead of restating it. */
export function applied(messages: readonly Message[], frame: Frame): readonly Message[] {
  switch (frame.type) {
    case 'opened':
      return [...messages, frame.message]
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

/* Consecutive text is one paragraph, not one part per chunk — unless the two
   came from different speakers: a subagent's words are not the agent's. */
function merged(parts: readonly Part[], next: Part): Part[] {
  const last = parts[parts.length - 1]
  if (last?.kind === 'text' && next.kind === 'text' && last.parent === next.parent) {
    return [...parts.slice(0, -1), { kind: 'text', text: last.text + next.text, parent: last.parent }]
  }
  return [...parts, next]
}

/* The profile a conversation is tied to, or nothing before its first turn.
   Fixed on purpose: one transcript, one account. */
export function fixedTo(conversation: Conversation | null): string | null {
  return conversation?.profile ? conversation.profile : null
}

/* What the composer may offer: the installed profiles, narrowed to the one
   the conversation already belongs to. A profile off the PATH is not an
   option — picking it would only fail at send time. */
export function choices(
  profiles: readonly Profile[],
  fixed: string | null,
): readonly Profile[] {
  // `runnable` and not merely installed: the composer spawns a process, and a
  // profile the shell alone knows — a function in someone's `.zshrc` — has no
  // process to spawn. The terminal can still open it, which is why the two are
  // different answers.
  const usable = profiles.filter((profile) => profile.reach === 'runnable')
  return fixed ? usable.filter((profile) => profile.id === fixed) : usable
}

/* Whether the send button does anything. */
export function ready(prompt: string, sending: boolean, profileId: string | null): boolean {
  return !sending && profileId !== null && prompt.trim().length > 0
}

/* What a conversation has cost, or nothing when it has cost nothing yet.
   A `$0.00` on screen is noise wearing the clothes of information. */
export function money(usd: number): string | null {
  if (!(usd > 0)) return null
  return usd < 0.01 ? `$${usd.toFixed(4)}` : `$${usd.toFixed(2)}`
}

/* What the agent may do without asking.

   `manual` stops and asks, and the question arrives in the thread with accept
   and refuse on it — which is what makes it an option at all. It was left out
   while there was nowhere to answer. */
export interface Mode {
  readonly id: string
  readonly label: string
  /* What it actually does. "Ask" names nothing on its own — the whole point
     of the row is knowing what you are agreeing to. */
  readonly what: string
}

export const MODES: readonly Mode[] = [
  { id: 'manual', label: 'Supervised', what: 'Ask before commands and file changes' },
  { id: 'acceptEdits', label: 'Accept edits', what: 'Edits go through; ask before anything else' },
  { id: 'bypassPermissions', label: 'Full access', what: 'Commands and edits without asking' },
]

export const modeName = (id: string): string =>
  MODES.find((mode) => mode.id === id)?.label ?? id

/* How hard the agent is asked to think, as words rather than the CLI's own
   tokens: `xhigh` is an argument, not a label. */
const EFFORTS: Readonly<Record<string, string>> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
  xhigh: 'Extra high',
  max: 'Max',
}

export const effortName = (effort: string): string => EFFORTS[effort] ?? effort

/* Which mode stops to ask. The session is told to hold its tools only in that
   one — a turn held in a mode that never asks would wait for a question that
   never comes. */
export const ASKS = (mode: string): boolean => mode === 'manual'


/* The prompt the agent actually receives: what was typed, with each
   attachment named as a path it can open. The paths lead rather than trail,
   because the CLI reads them as context for what follows. */
export function withFiles(prompt: string, files: readonly Attachment[]): string {
  if (files.length === 0) return prompt
  return `${files.map((file) => `@${file.path}`).join(' ')}\n${prompt}`
}

/* Whether a conversation stopped with the person's message unanswered.

   The answer is written when the turn ends, so an app that closed part way
   through leaves exactly this: a question and nothing under it. Said on
   screen, it reads as what happened; left silent, it reads as a message that
   was never sent. */
export function unanswered(messages: readonly Message[], sending: boolean): boolean {
  if (sending) return false
  const last = messages.at(-1)
  return last !== undefined && last.role === 'user'
}

