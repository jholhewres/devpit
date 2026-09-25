import { useCallback } from 'react'

import type { Card, CardDetail, DeleteRefusal } from '../gen/bindings'
import type { Tab } from './strip'
import { ask, commands } from './live'
import type { UseBoard } from './useBoard'
import { liveWork, stopLiveWork, type LiveWork } from './liveWork'
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
  chat: () => void
  copyBranch?: () => void
  archive: (force: boolean) => Promise<string | null>
  remove: (force: boolean) => Promise<DeleteRefusal | string | null>
  /** What is still going on the card, read when it is about to end. */
  liveWork: () => Promise<LiveWork>
  /** Closes the card's terminals and stops its runs. */
  stopLive: (live: LiveWork) => Promise<string | null>
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
  open: readonly Tab[] = [],
): Promise<string | null> {
  const answer = await ask(() => commands.cardTerminal(projectId, cardId))
  if (!answer.data) return answer.error ?? 'the terminal did not open'
  const { tabId, layout } = answer.data
  /* A tab already open is only brought to the front, and the agent it was
     asked for is dropped on the way — so it is sent to the pane the tab has. */
  if (launch && open.some((tab) => tab.id === tabId)) {
    show('term', { id: tabId, title, cardId: answer.data.cardId })
    return (await ask(() => commands.sessionLaunchAgent(projectId, layout.focusedId, launch, null))).error
  }
  show('term', { id: tabId, title, cardId: answer.data.cardId, ...(launch ? { launch } : {}) })
  return null
}

/** Puts a card's branch on the clipboard. Answers why it could not, or null. */
export async function copyBranch(projectId: string, cardId: string): Promise<string | null> {
  const answer = await ask(() => commands.cardDetail(projectId, cardId))
  const branch = answer.data?.worktree?.branch
  if (!branch) return answer.error ?? 'this card has no branch yet'
  /* After an awaited read WebKit may no longer count this as the click that
     asked, and refuse: said, rather than nothing happening. */
  try {
    await navigator.clipboard.writeText(branch)
  } catch {
    return `could not put ${branch} on the clipboard`
  }
  return null
}

/** The words a card's chat opens with: what the card says, and the files pinned to it. */
export function cardDraft(detail: Pick<CardDetail, 'card' | 'pinned'>): string {
  const pinned = detail.pinned.map((pin) => `- ${pin.path}`)
  return [
    `# ${detail.card.title}`,
    detail.card.body.trim(),
    pinned.length > 0 ? ['Pinned files:', ...pinned].join('\n') : '',
  ]
    .filter(Boolean)
    .join('\n\n')
}

/** Opens a conversation about a card, in its checkout, with the card in its
    composer and nothing sent. One account installed is the answer; with
    several, the chat asks on its first turn. Answers the refusal, or null. */
export async function openCardChat(
  projectId: string,
  cardId: string,
  show: (kind: 'chat', tab: Partial<Tab>) => void,
): Promise<string | null> {
  const [detail, profiles] = await Promise.all([
    ask(() => commands.cardDetail(projectId, cardId)),
    ask(() => commands.agentProfiles()),
  ])
  if (!detail.data) return detail.error ?? 'the card could not be read'
  const installed = (profiles.data ?? []).filter((profile) => profile.path !== null)
  const only = installed.length === 1 ? installed[0]!.id : null
  const answer = await ask(() => commands.cardChat(projectId, cardId, only))
  if (!answer.data) return answer.error ?? 'the chat did not open'
  show('chat', { id: answer.data, draft: cardDraft(detail.data) })
  return null
}

export function useCardActs(
  projectId: string | null,
  live: UseBoard,
  onArchived: (cardId: string) => void,
): (card: Card) => CardBoardActs {
  const { show, closeNow } = useShell()
  const { reload, report } = live

  return useCallback(
    (card: Card): CardBoardActs => ({
      terminal: () => {
        if (projectId) void openCardTerminal(projectId, card.id, card.title, show).then((refused) => refused && report(refused))
      },
      chat: () => {
        if (projectId) void openCardChat(projectId, card.id, show).then((refused) => refused && report(refused))
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
      liveWork: async () => {
        if (!projectId) return { tabs: [], runs: [] }
        const answer = await ask(() => commands.cardDetail(projectId, card.id))
        return liveWork(answer.data?.sessions ?? [])
      },
      stopLive: (going) => (projectId ? stopLiveWork(projectId, card.id, going, closeNow) : Promise.resolve('no project open')),
      archived: () => onArchived(card.id),
      problem: report,
    }),
    [projectId, show, closeNow, reload, report, onArchived],
  )
}
