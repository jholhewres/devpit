import { useCallback } from 'react'

import type { Card, DeleteRefusal } from '../gen/bindings'
import type { Tab } from './strip'
import { ask, commands } from './live'
import type { UseBoard } from './useBoard'
import { useShell } from './useShell'

/*
 * What a card's menu does on the board, apart from the menu that offers it.
 *
 * The terminal and the branch are asked of the backend here and nowhere else
 * on the board: the card's own Work section opens the same terminal.
 */

/** What a card can have done to it from the board, besides opening and playing. */
export interface CardBoardActs {
  terminal: () => void
  copyBranch?: () => void
  archive: (force: boolean) => Promise<string | null>
  remove: (force: boolean) => Promise<DeleteRefusal | string | null>
  /** After an archive, so the board offers Undo. */
  archived: () => void
  problem: (message: string) => void
}

/** Opens the terminal on a card's checkout, making the checkout if it has none. Answers the refusal, or null. */
export async function openCardTerminal(
  projectId: string,
  cardId: string,
  title: string,
  show: (kind: 'term', tab: Partial<Tab>) => void,
  launch?: string,
): Promise<string | null> {
  const answer = await ask(() => commands.cardTerminal(projectId, cardId))
  if (!answer.data) return answer.error ?? 'the terminal did not open'
  show('term', { id: answer.data.tabId, title, cardId: answer.data.cardId, ...(launch ? { launch } : {}) })
  return null
}

/** Puts a card's branch on the clipboard. Answers why it could not, or null. */
export async function copyBranch(projectId: string, cardId: string): Promise<string | null> {
  const answer = await ask(() => commands.cardDetail(projectId, cardId))
  const branch = answer.data?.worktree?.branch
  if (!branch) return answer.error ?? 'this card has no branch yet'
  await navigator.clipboard.writeText(branch)
  return null
}

export function useCardActs(
  projectId: string | null,
  live: UseBoard,
  onArchived: (cardId: string) => void,
): (card: Card) => CardBoardActs {
  const { show } = useShell()
  const { reload, report } = live

  return useCallback(
    (card: Card): CardBoardActs => ({
      terminal: () => {
        if (projectId) void openCardTerminal(projectId, card.id, card.title, show).then((refused) => refused && report(refused))
      },
      copyBranch: card.worktreePath
        ? () => {
            if (projectId) void copyBranch(projectId, card.id).then((refused) => refused && report(refused))
          }
        : undefined,
      archive: async (force) => {
        if (!projectId) return 'no project open'
        const answer = await ask(() => commands.cardArchive(projectId, card.id, force))
        if (!answer.error) reload()
        return answer.error
      },
      remove: async (force) => {
        if (!projectId) return 'no project open'
        const answer = await ask(() => commands.cardDelete(projectId, card.id, force))
        if (answer.error) return answer.error
        if (answer.data?.refused) return answer.data.refused
        reload()
        return null
      },
      archived: () => onArchived(card.id),
      problem: report,
    }),
    [projectId, show, reload, report, onArchived],
  )
}
