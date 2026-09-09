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
      return messages.map((message) => ({
        ...message,
        parts: message.parts.map((part) =>
          part.kind === 'tool_call' && part.id === frame.call_id
            ? { ...part, state: frame.state }
            : part,
        ),
      }))
    case 'ended':
      /* The turn is over, so nothing is still arriving. */
      return messages.map((message) => ({ ...message, streaming: false }))
    default:
      return messages
  }
}

/* Consecutive text is one paragraph, not one part per chunk. */
function merged(parts: readonly Part[], next: Part): Part[] {
  const last = parts[parts.length - 1]
  if (last?.kind === 'text' && next.kind === 'text') {
    return [...parts.slice(0, -1), { kind: 'text', text: last.text + next.text }]
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
  const installed = profiles.filter((profile) => profile.path !== null)
  return fixed ? installed.filter((profile) => profile.id === fixed) : installed
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
export const MODES: readonly { readonly id: string; readonly label: string }[] = [
  { id: 'manual', label: 'Ask' },
  { id: 'acceptEdits', label: 'Edits' },
  { id: 'bypassPermissions', label: 'Full access' },
]

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

/* What a model is called on screen.

   `default` is what the CLI is passed when nobody picked, and it is a fine
   argument and a useless label — it names no model to the person reading it. */
export const modelName = (model: string): string =>
  model === 'default' ? "The account's default" : model
