import type { ChatCard } from './useChat'
import { useShell } from './useShell'

/*
 * The card a conversation is about, in the chat's corner.
 *
 * It opens the card while the card is on a board. Archived, the card is only a
 * name: a chip that opened nothing would be a control that lies.
 */
export function ChatCardChip({ card }: { card: ChatCard | null }): React.JSX.Element | null {
  const { show, openCard } = useShell()
  if (!card) return null
  const name = card.title ?? 'a card'
  if (!card.onBoard) {
    return (
      <span className="pcorner__card" title="This card is no longer on the board">
        {name}
      </span>
    )
  }
  return (
    <button
      className="pcorner__card"
      title="Open the card"
      onClick={() => {
        show('board')
        openCard(card.id)
      }}
    >
      {name}
    </button>
  )
}
