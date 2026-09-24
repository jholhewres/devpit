import { useEffect, useState } from 'react'

/*
 * What is typed in a conversation's composer and not yet sent, kept per
 * conversation so leaving the chat — another project, a reload — does not
 * throw it away. Per viewer, and only a convenience: storage that refuses
 * leaves the composer working as before.
 */

const KEY = (conversationId: string): string => `devpit.chatDraft.${conversationId}`

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

/** The composer's text: a card's draft first, then what was left unsent. */
export function useDraft(conversationId: string, given: string | undefined): [string, (text: string | ((was: string) => string)) => void] {
  const [text, setText] = useState(() => given ?? keptDraft(conversationId) ?? '')
  useEffect(() => keepDraft(conversationId, text), [conversationId, text])
  return [text, setText]
}
