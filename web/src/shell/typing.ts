/*
 * Enter and Escape, while a character is still being composed.
 *
 * A dead key and an IME both work the same way: the first keystroke starts a
 * composition, the next one finishes it, and the browser sends `Enter` to
 * *commit* the composed character. A field that treats that Enter as "done"
 * submits halfway through a word — the accent is lost and the form closes on
 * a value the person had not finished typing.
 *
 * Every keyboard that types `á` by pressing `'` then `a` is affected, which is
 * most of the ones outside US English. This was found the way it usually is:
 * somebody typed an accented letter into the card's title.
 *
 * `isComposing` is the standard signal and it is on the event already; the
 * `229` is the older spelling of the same fact, which WebKit still emits in
 * some paths. Both are checked because checking one is how this gets fixed
 * twice.
 */

/** Whether this keystroke is part of a character still being composed. */
export function composing(event: {
  readonly nativeEvent?: { readonly isComposing?: boolean }
  readonly isComposing?: boolean
  readonly keyCode?: number
}): boolean {
  return (
    event.isComposing === true ||
    event.nativeEvent?.isComposing === true ||
    event.keyCode === 229
  )
}

/** Enter, meaning it — not Enter finishing a composition. */
export function committed(event: {
  readonly key: string
  readonly nativeEvent?: { readonly isComposing?: boolean }
  readonly isComposing?: boolean
  readonly keyCode?: number
}): boolean {
  return event.key === 'Enter' && !composing(event)
}

/** Escape, meaning it.
 *
 *  An IME uses Escape to abandon what is being composed, and a dialog that
 *  closes on that takes the whole form with the half-typed word. */
export function abandoned(event: {
  readonly key: string
  readonly nativeEvent?: { readonly isComposing?: boolean }
  readonly isComposing?: boolean
  readonly keyCode?: number
}): boolean {
  return event.key === 'Escape' && !composing(event)
}
