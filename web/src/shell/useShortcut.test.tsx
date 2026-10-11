import { cleanup, fireEvent, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { recordKeys, resetAllKeys } from './shortcuts'
import { listen, useShortcut, whereOf } from './useShortcut'

/*
 * The one listener, driven with real events: what the registry decides is
 * only worth anything if the press reaches whoever took the shortcut, and
 * nobody else.
 */

const letGo: Array<() => void> = []
afterEach(() => {
  for (const one of letGo.splice(0)) one()
  cleanup()
  resetAllKeys()
  document.body.innerHTML = ''
})

const take = (id: Parameters<typeof listen>[0], taker = vi.fn()): typeof taker => {
  letGo.push(listen(id, taker))
  return taker
}

/** A terminal as xterm builds one: its keys arrive on a textarea inside `.xterm`,
 *  where xterm's own listener sends them to the program. */
function terminal(): { field: HTMLTextAreaElement; program: ReturnType<typeof vi.fn> } {
  const host = document.createElement('div')
  host.className = 'xterm'
  const field = document.createElement('textarea')
  host.append(field)
  document.body.append(host)
  const program = vi.fn()
  field.addEventListener('keydown', program, true)
  return { field, program }
}

describe('a press inside a terminal', () => {
  it('goes to the program unless the shortcut is allowed there', () => {
    const palette = take('palette')
    const split = take('splitRight')
    const { field, program } = terminal()

    fireEvent.keyDown(field, { key: 'k', ctrlKey: true })
    expect(palette).not.toHaveBeenCalled()
    expect(program).toHaveBeenCalledTimes(1)

    fireEvent.keyDown(field, { key: 'D', ctrlKey: true, shiftKey: true })
    expect(split).toHaveBeenCalledTimes(1)
    /* Taken before xterm saw it, so the program never gets a stray key. */
    expect(program).toHaveBeenCalledTimes(1)
  })

  it('is read as one', () => {
    expect(whereOf(terminal().field)).toBe('terminal')
    const input = document.createElement('input')
    expect(whereOf(input)).toBe('field')
    expect(whereOf(document.createElement('button'))).toBe('elsewhere')
  })
})

describe('a press anywhere else', () => {
  it('reaches whoever took the shortcut, after the page had its say', () => {
    const palette = take('palette')
    const event = fireEvent.keyDown(document.body, { key: 'k', ctrlKey: true })
    expect(palette).toHaveBeenCalledTimes(1)
    expect(event).toBe(false) // prevented
  })

  it('is left alone once something on the page took it', () => {
    const palette = take('palette')
    const box = document.createElement('div')
    box.addEventListener('keydown', (event) => event.preventDefault())
    document.body.append(box)
    fireEvent.keyDown(box, { key: 'k', ctrlKey: true })
    expect(palette).not.toHaveBeenCalled()
  })

  it('leaves a key with no modifier to a field being typed in', () => {
    const sheet = take('shortcuts')
    const input = document.createElement('input')
    document.body.append(input)
    fireEvent.keyDown(input, { key: '?', shiftKey: true })
    expect(sheet).not.toHaveBeenCalled()
    fireEvent.keyDown(document.body, { key: '?', shiftKey: true })
    expect(sheet).toHaveBeenCalledTimes(1)
  })

  it('passes over a taker that says it is not its, and goes on', () => {
    const older = take('next')
    const newer = take('next', vi.fn(() => false))
    fireEvent.keyDown(document.body, { key: 'N', ctrlKey: true, shiftKey: true })
    expect(newer).toHaveBeenCalledTimes(1)
    expect(older).toHaveBeenCalledTimes(1)
  })

  it('is not prevented when nobody wants it', () => {
    take('next', vi.fn(() => false))
    expect(fireEvent.keyDown(document.body, { key: 'N', ctrlKey: true, shiftKey: true })).toBe(true)
  })

  it('inside a dialog, fires only the window’s own keys, not the pane’s behind it', () => {
    const palette = take('palette')
    const split = take('splitRight')
    const dialog = document.createElement('div')
    dialog.setAttribute('aria-modal', 'true')
    const button = document.createElement('button')
    dialog.append(button)
    document.body.append(dialog)
    fireEvent.keyDown(button, { key: 'D', ctrlKey: true, shiftKey: true })
    expect(split).not.toHaveBeenCalled()
    fireEvent.keyDown(button, { key: 'k', ctrlKey: true })
    expect(palette).toHaveBeenCalledTimes(1)
  })

  it('answers to a key recorded a moment ago', () => {
    const palette = take('palette')
    recordKeys('palette', { key: 'j', ctrlKey: true }, false)
    fireEvent.keyDown(document.body, { key: 'j', ctrlKey: true })
    expect(palette).toHaveBeenCalledTimes(1)
  })
})

function Pane({ shown, onSplit }: { shown: boolean; onSplit: () => void }): null {
  useShortcut('splitRight', onSplit, shown)
  return null
}

describe('a pane that is mounted but not in front', () => {
  it('does not take its key; the one in front does', () => {
    /* Every terminal tab stays mounted, and each used to split together. */
    const hidden = vi.fn()
    const front = vi.fn()
    render(
      <>
        <Pane shown onSplit={front} />
        <Pane shown={false} onSplit={hidden} />
      </>,
    )
    fireEvent.keyDown(document.body, { key: 'D', ctrlKey: true, shiftKey: true })
    expect(front).toHaveBeenCalledTimes(1)
    expect(hidden).not.toHaveBeenCalled()
  })
})
