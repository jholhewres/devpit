import { useEffect, useRef, useState } from 'react'

import type { Attachment } from '../gen/bindings'

/*
 * What is typed in a conversation's composer and not yet sent, kept per
 * conversation so leaving the chat — another project, a reload — does not
 * throw it away. Per viewer, and only a convenience: storage that refuses
 * leaves the composer working as before.
 */

const KEY = (conversationId: string): string => `devpit.chatDraft.${conversationId}`
const FILES = (conversationId: string): string => `devpit.chatDraftFiles.${conversationId}`

/** How long after the last key the draft is written: every key would write a long draft whole each time. */
export const DRAFT_WAIT = 250

export function keptDraft(conversationId: string): string | null {
  try {
    return localStorage.getItem(KEY(conversationId))
  } catch {
    return null
  }
}

export function keepDraft(conversationId: string, text: string): void {
  try {
    if (text) localStorage.setItem(KEY(conversationId), text)
    else localStorage.removeItem(KEY(conversationId))
  } catch {
    // Refused: the draft lives as long as the composer does.
  }
}

const isAttachment = (one: unknown): one is Attachment =>
  typeof one === 'object' && one !== null && ['name', 'path', 'kind'].every((key) => typeof (one as Record<string, unknown>)[key] === 'string')

/** The files attached to the draft and not yet sent. */
export function keptFiles(conversationId: string): Attachment[] {
  try {
    const kept: unknown = JSON.parse(localStorage.getItem(FILES(conversationId)) ?? '[]')
    return Array.isArray(kept) ? kept.filter(isAttachment) : []
  } catch {
    return []
  }
}

export function keepFiles(conversationId: string, files: readonly Attachment[]): void {
  try {
    if (files.length > 0) localStorage.setItem(FILES(conversationId), JSON.stringify(files))
    else localStorage.removeItem(FILES(conversationId))
  } catch {
    // Refused, as above.
  }
}

/** The composer's text: a card's draft first, then what was left unsent. */
export function useDraft(conversationId: string, given: string | undefined): [string, (text: string | ((was: string) => string)) => void] {
  const [text, setText] = useState(() => given ?? keptDraft(conversationId) ?? '')
  const latest = useRef(text)
  useEffect(() => {
    latest.current = text
    const later = setTimeout(() => keepDraft(conversationId, text), DRAFT_WAIT)
    return () => clearTimeout(later)
  }, [conversationId, text])
  /* Leaving writes what the wait had not yet. */
  useEffect(() => () => keepDraft(conversationId, latest.current), [conversationId])
  return [text, setText]
}

const STASH = (conversationId: string): string => `devpit.chatStash.${conversationId}`

export function keptStash(conversationId: string): string | null {
  try {
    return localStorage.getItem(STASH(conversationId))
  } catch {
    return null
  }
}

/** The stash key swaps what is typed with what was put aside: either may be empty. */
export function swapStash(conversationId: string, typed: string): string {
  const back = keptStash(conversationId) ?? ''
  try {
    if (typed.trim()) localStorage.setItem(STASH(conversationId), typed)
    else localStorage.removeItem(STASH(conversationId))
  } catch {
    return typed
  }
  return back
}
