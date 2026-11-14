import type { Message } from '../gen/bindings'
import type { ChatMessages } from './chatMessages'
import type { Reveal } from './useJump'
import { useShortcut } from './useShortcut'

/*
 * Reading a conversation from the keyboard: a message at a time, or straight
 * to the next tool that failed, and the row there opened or closed.
 */

export type Step = 'next' | 'previous' | 'failed'

const failedIn = (message: Message): boolean => message.parts.some((part) => part.kind === 'tool_result' && part.is_error)

/* The message a step lands on from `from`, or none past the end. */
export function stepTo(messages: readonly Message[], from: string | null, step: Step): string | null {
  const at = from ? messages.findIndex((one) => one.id === from) : -1
  if (step === 'previous') return at > 0 ? messages[at - 1].id : null
  const after = messages.slice(at + 1)
  return (step === 'next' ? after[0] : after.find(failedIn))?.id ?? null
}

/* What takes the focus in a row a step reached: for a failure, the failed
   tool when its stack is open, else the stack that holds it. */
export function landing(row: HTMLElement, step: Step): HTMLElement {
  if (step !== 'failed') return row
  return row.querySelector<HTMLElement>('.arow[data-failed] .arow__h') ?? row.querySelector<HTMLElement>('.acts__h') ?? row
}

/* Where the reader is: the row holding the focus, else the first one in view. */
function placeOf(box: HTMLElement | null): string | null {
  const focused = document.activeElement?.closest<HTMLElement>('.trow')
  if (focused && box?.contains(focused)) return focused.dataset.id ?? null
  const top = box?.getBoundingClientRect().top ?? 0
  return [...(box?.querySelectorAll<HTMLElement>('.trow') ?? [])].find((row) => row.getBoundingClientRect().bottom > top)?.dataset.id ?? null
}

export function useThreadKeys(
  mine: boolean,
  messages: ChatMessages,
  box: React.RefObject<HTMLDivElement | null>,
  reveal: React.RefObject<Reveal | null>,
): void {
  const go = (step: Step) => (): false | void => {
    const id = mine ? stepTo(messages.get(), placeOf(box.current), step) : null
    if (!id || !reveal.current) return false
    reveal.current(id, (row) => landing(row, step).focus())
  }
  useShortcut('nextMessage', go('next'))
  useShortcut('previousMessage', go('previous'))
  useShortcut('nextFailed', go('failed'))
  useShortcut('toggleRow', () => {
    const here = document.activeElement
    const row = here?.closest('.trow')
    if (!mine || !row || !box.current?.contains(row)) return false
    const toggle = here?.matches('[aria-expanded]') ? here : row.querySelector('[aria-expanded]')
    if (!(toggle instanceof HTMLElement)) return false
    toggle.click()
  })
}
