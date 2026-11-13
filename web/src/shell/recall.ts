import { useRef } from 'react'

import type { Message } from '../gen/bindings'
import type { ChatMessages } from './chatMessages'
import { matches } from './shortcuts'

/*
 * ↑ in an empty composer: the prompts sent earlier in this conversation, the
 * newest first, the way a shell walks its history.
 */

/* The line `withFiles` puts before a prompt, naming only files. */
const FILES_LINE = /^@\S+(?: @\S+)*\n/

/** What the person said in this conversation, oldest first, as it was typed. */
export function promptsOf(messages: readonly Message[]): string[] {
  return messages
    .filter((one) => one.role === 'user')
    .map((one) =>
      one.parts
        .map((part) => ('text' in part ? part.text : ''))
        .join('')
        .replace(FILES_LINE, '')
        .trim(),
    )
    .filter(Boolean)
}

/**
 * Where ↑ (`up`) or ↓ takes a walk through `count` prompts from `at`: an
 * index, null for the empty composer, or undefined when the key is not the
 * walk's — ↓ with no walk begun, ↑ with nothing sent.
 */
export function walked(count: number, at: number | null, up: boolean): number | null | undefined {
  if (up) return count === 0 ? undefined : at === null ? count - 1 : Math.max(0, at - 1)
  if (at === null) return undefined
  return at >= count - 1 ? null : at + 1
}

/**
 * The composer's ↑ and ↓: walks only while the field is empty or still shows
 * the prompt the walk put there. Typing over it, or anything attached, ends
 * the walk, and the keys move the caret again.
 */
export function useRecall(
  messages: ChatMessages,
  prompt: string,
  setPrompt: (text: string) => void,
  attached: boolean,
): (event: React.KeyboardEvent<HTMLTextAreaElement>) => boolean {
  const at = useRef<number | null>(null)
  return (event) => {
    const up = matches('recallPrompt', event)
    if ((!up && !matches('recallNext', event)) || attached) return false
    const prompts = promptsOf(messages.get())
    if (at.current !== null && prompt !== prompts[at.current]) at.current = null
    if (at.current === null && prompt !== '') return false
    const next = walked(prompts.length, at.current, up)
    if (next === undefined) return false
    event.preventDefault()
    at.current = next
    setPrompt(next === null ? '' : prompts[next]!)
    return true
  }
}
