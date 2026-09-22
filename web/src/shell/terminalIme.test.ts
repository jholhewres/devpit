import { afterEach, describe, expect, it, vi } from 'vitest'

import { composingKey, guardComposition, isEcho } from './terminalIme'

/* What WebKitGTK sends for `~` then `a` through IBus, as logged in a dev
   build: no compositionstart, a 229 keydown, an input from composition, then
   compositionend with the text. */
function deadKey(textarea: HTMLTextAreaElement, text: string): void {
  textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'Unidentified', keyCode: 229, bubbles: true }))
  textarea.value = text
  textarea.dispatchEvent(new InputEvent('input', { inputType: 'insertFromComposition', data: text, isComposing: true, bubbles: true }))
  textarea.dispatchEvent(new CompositionEvent('compositionend', { data: text, bubbles: true }))
}

describe('composed text into a terminal', () => {
  let box: HTMLElement
  afterEach(() => box.remove())

  const mount = () => {
    box = document.createElement('div')
    const textarea = document.createElement('textarea')
    box.append(textarea)
    document.body.append(box)
    /* Stands in for xterm: everything that reaches the textarea itself. */
    const xterm = vi.fn()
    for (const name of ['keydown', 'input', 'compositionend']) textarea.addEventListener(name, xterm)
    return { textarea, xterm }
  }

  it('sends a dead key once, and xterm never sees the composition', () => {
    const { textarea, xterm } = mount()
    const send = vi.fn()
    const undo = guardComposition(box, send)
    deadKey(textarea, 'ã')
    expect(send).toHaveBeenCalledTimes(1)
    expect(send).toHaveBeenCalledWith('ã')
    expect(xterm).not.toHaveBeenCalled()
    undo()
  })

  it('empties the textarea, so the next key has nothing left to repeat', async () => {
    const { textarea } = mount()
    const undo = guardComposition(box, vi.fn())
    deadKey(textarea, 'ã')
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(textarea.value).toBe('')
    undo()
  })

  it('lets ordinary keys through to xterm', () => {
    const { textarea, xterm } = mount()
    const undo = guardComposition(box, vi.fn())
    textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'x', keyCode: 88, bubbles: true }))
    expect(xterm).toHaveBeenCalledTimes(1)
    undo()
  })

  it('knows the echo of a commit from new typing', () => {
    const commit = { data: 'ã', at: 1000 }
    expect(isEcho({ data: 'ã', inputType: 'insertText' }, commit, 1050)).toBe(true)
    expect(isEcho({ data: 'ã', inputType: 'insertText' }, commit, 1500)).toBe(false)
    expect(isEcho({ data: 'a', inputType: 'insertText' }, commit, 1050)).toBe(false)
    expect(isEcho({ data: 'ã', inputType: 'insertText' }, null, 1050)).toBe(false)
  })

  it('treats a 229 keydown as the IME’s, whatever it says about composing', () => {
    expect(composingKey({ isComposing: false, keyCode: 229 })).toBe(true)
    expect(composingKey({ isComposing: true, keyCode: 65 })).toBe(true)
    expect(composingKey({ isComposing: false, keyCode: 65 })).toBe(false)
  })
})
