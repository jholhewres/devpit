/*
 * Composed text into a terminal, once.
 *
 * xterm follows a composition by diffing the value of its hidden textarea,
 * which only ever grows. Under IBus on WebKitGTK the committed text was sent
 * twice, and what was left in the textarea from the last composition was sent
 * again ahead of the next key. Typed as `´a x ~a ç`, a terminal received
 * `á x ã ã ç ã ç` — recorded through xdotool into a dev build, with `od -c`
 * reading the pane.
 *
 * What WebKitGTK sends for one dead key, logged the same way: no
 * `compositionstart` at all — a keydown with keyCode 229, an `input` of type
 * `insertFromComposition`, then `compositionend` with the text. xterm sent the
 * text on the 229 keydown, by diffing its textarea, and again on
 * `compositionend`, from positions no `compositionstart` ever set.
 *
 * So the terminal takes composition over: nothing of it reaches xterm, the
 * committed text is sent once through `terminal.input`, the echo that follows
 * it is dropped, and the textarea is emptied after every commit so nothing is
 * left to be diffed into the next key. Listened for on the way down, before
 * xterm's own textarea handlers.
 */

/** How long after a commit an identical `input` is its echo, not new typing. */
const ECHO_MS = 120

export interface Commit {
  readonly data: string
  readonly at: number
}

/** Whether an `input` event is the echo of the commit just sent. */
export function isEcho(
  event: Pick<InputEvent, 'data' | 'inputType'>,
  last: Commit | null,
  now: number,
): boolean {
  return (
    last !== null &&
    event.inputType === 'insertText' &&
    event.data === last.data &&
    now - last.at < ECHO_MS
  )
}

/** Whether a keydown belongs to a composition, which the IME owns. */
export const composingKey = (event: Pick<KeyboardEvent, 'isComposing' | 'keyCode'>): boolean =>
  event.isComposing || event.keyCode === 229

/** Takes composition over for the terminal inside `box`. Returns the undo. */
export function guardComposition(box: HTMLElement, send: (data: string) => void): () => void {
  let last: Commit | null = null
  const textarea = (): HTMLTextAreaElement | null => box.querySelector('textarea')
  const empty = (): void => {
    setTimeout(() => {
      const field = textarea()
      if (field) field.value = ''
    }, 0)
  }

  const keydown = (event: KeyboardEvent): void => {
    // Not prevented: the IME still needs the key. Only kept from xterm.
    if (composingKey(event)) event.stopPropagation()
  }
  const composing = (event: CompositionEvent): void => {
    event.stopPropagation()
  }
  const committed = (event: CompositionEvent): void => {
    event.stopPropagation()
    if (event.data) {
      send(event.data)
      last = { data: event.data, at: performance.now() }
    }
    empty()
  }
  const input = (event: Event): void => {
    const typed = event as InputEvent
    if (typed.isComposing || typed.inputType === 'insertCompositionText') {
      event.stopPropagation()
      return
    }
    if (isEcho(typed, last, performance.now())) {
      event.stopPropagation()
      last = null
      empty()
    }
  }

  box.addEventListener('keydown', keydown, true)
  box.addEventListener('compositionstart', composing, true)
  box.addEventListener('compositionupdate', composing, true)
  box.addEventListener('compositionend', committed, true)
  box.addEventListener('input', input, true)
  return () => {
    box.removeEventListener('keydown', keydown, true)
    box.removeEventListener('compositionstart', composing, true)
    box.removeEventListener('compositionupdate', composing, true)
    box.removeEventListener('compositionend', committed, true)
    box.removeEventListener('input', input, true)
  }
}
