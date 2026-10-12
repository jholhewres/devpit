import { Channel, invoke } from '@tauri-apps/api/core'

import type {
  Ask,
  Conversation,
  Frame,
  Message,
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

/* The turns running in a conversation, from where they are: what they sent
   before this chat was open, then the rest, one turn after another. Resolves
   when the last ends — `false` at once when nothing was running. `leave`
   stops the frames: a chat that has gone would otherwise be sent every one
   until the turn ended, once per remount. */
export interface Joined {
  readonly running: Promise<boolean>
  readonly leave: () => void
}

export function rejoin(conversationId: string, onFrame: (frame: Frame) => void): Joined | null {
  if (!inTauri()) return null
  const channel = new Channel<Frame>()
  channel.onmessage = onFrame
  return {
    running: invoke('chat_rejoin', { conversationId, onFrame: channel }) as Promise<boolean>,
    leave: () => void invoke('chat_leave', { conversationId, channel: channel.id }).catch(() => {}),
  }
}

/* One opening of a conversation in a chat: a mount, or the next conversation
   shown in the same pane. A frame or an answer still on its way to one that
   closed lands nowhere — a single flag, set again by the next opening, let an
   old rejoin write one conversation's turn into another. */
export interface Opening {
  readonly open: () => boolean
  /** Kept to be left when this opening closes. */
  readonly join: (joined: Joined | null) => Joined | null
  readonly close: () => void
  /** This chat's own turn is running; a turn woken meanwhile waits for it. */
  sending: boolean
  woke: boolean
}

export function opening(): Opening {
  let open = true
  const joined = new Set<Joined>()
  return {
    open: () => open,
    join: (one) => {
      if (one && open) joined.add(one)
      else one?.leave()
      return one
    },
    close: () => {
      open = false
      for (const one of joined) one.leave()
      joined.clear()
    },
    sending: false,
    woke: false,
  }
}

/* Frames arrive many to a paint while an answer streams; they are applied
   together, once per frame drawn, rather than one render each. A timer
   stands in when no frame comes: WebKitGTK gives none to a hidden window,
   nor to some visible ones, and the whole answer then waited for the end. */
export function batched(apply: (frames: readonly Frame[]) => void): { push: (frame: Frame) => void; flush: () => void } {
  let pending: Frame[] = []
  let scheduled: { cancel: () => void } | null = null
  const flush = (): void => {
    scheduled?.cancel()
    scheduled = null
    if (pending.length === 0) return
    const frames = pending
    pending = []
    apply(frames)
  }
  const push = (frame: Frame): void => {
    pending.push(frame)
    if (scheduled) return
    const timer = setTimeout(flush, 100)
    const painted = document.hidden ? null : requestAnimationFrame(flush)
    scheduled = {
      cancel: () => {
        clearTimeout(timer)
        if (painted !== null) cancelAnimationFrame(painted)
      },
    }
  }
  return { push, flush }
}

export { applied, joined, withFiles, withHeard } from './frames'

/* A message sent into the turn in flight sits before the answer still being
   written, where the transcript keeps it. */
export function steered(messages: readonly Message[], said: Message): readonly Message[] {
  const at = messages.findLastIndex((one) => one.streaming)
  return at < 0 ? [...messages, said] : [...messages.slice(0, at), said, ...messages.slice(at)]
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

