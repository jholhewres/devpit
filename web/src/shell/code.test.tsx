import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Code } from './Code'
import { ofName, ofPath } from './languages'
import { Painted } from './Painted'

afterEach(cleanup)

/*
 * The editor is a textarea with a painting behind it, and the two things that
 * can go wrong are both covered here: the field stops being a real field, or
 * the painting stops matching it.
 */

describe('the field is still a field', () => {
  it('holds the text and reports what was typed', () => {
    const changed = vi.fn()
    render(<Code text="fn main() {}" language={ofName('rust')} onChange={changed} />)
    const field = screen.getByRole('textbox') as HTMLTextAreaElement
    expect(field.value).toBe('fn main() {}')

    fireEvent.change(field, { target: { value: 'fn other() {}' } })
    expect(changed).toHaveBeenCalledWith('fn other() {}')
  })

  it('is a textarea and not something pretending to be one', () => {
    // `typing.ts` exists because a dead key sends Enter to commit an accent,
    // and that fix only holds while the browser's own field does the typing.
    render(<Code text="x" language={null} />)
    expect(screen.getByRole('textbox').tagName).toBe('TEXTAREA')
  })

  it('can be read-only without losing its text', () => {
    render(<Code text="const a = 1" language={ofName('ts')} readOnly />)
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).readOnly).toBe(true)
  })
})

describe('the painting matches the field', () => {
  it('paints exactly the characters the field holds', () => {
    // Drift here is a caret sitting off the letters, which is the one thing
    // that makes this technique unusable.
    const text = 'fn main() {\n    let x = "hi"; // done\n}\n'
    const { container } = render(<Code text={text} language={ofName('rust')} />)
    const painted = container.querySelector('.code__paint')!.textContent
    // The painting carries one extra newline so the last line has a line to
    // sit on; everything before it is the text.
    expect(painted).toBe(`${text}\n`)
  })

  it('is hidden from anything that reads the page', () => {
    // The field already carries the text. A second copy would be read twice.
    const { container } = render(<Code text="x" language={null} />)
    expect(container.querySelector('.code__paint')!.getAttribute('aria-hidden')).toBe('true')
  })

  it('colours a keyword, a string and a comment differently', () => {
    const { container } = render(
      <Code text={'let a = "s"; // c'} language={ofName('rust')} />,
    )
    const kinds = [...container.querySelectorAll('.tok')].map((one) => one.className)
    expect(kinds).toContain('tok tok--word')
    expect(kinds).toContain('tok tok--string')
    expect(kinds).toContain('tok tok--comment')
  })

  it('paints nothing in particular for a language it does not know', () => {
    const { container } = render(<Code text="just words here" language={null} />)
    expect(container.querySelectorAll('.tok')).toHaveLength(0)
    expect(container.querySelector('.code__paint')!.textContent).toBe('just words here\n')
  })
})

describe('markup in a file is text, structurally', () => {
  it.each([
    '<script>alert(1)</script>',
    '<img src=x onerror=alert(1)>',
    '</span><b>escaped?</b>',
  ])('draws %s as characters', (hostile) => {
    // Not a sanitiser — there is no path from here to innerHTML at all. This
    // is the reason the highlighter is written rather than installed: every
    // one worth installing hands back a string of HTML.
    const { container } = render(<Painted text={hostile} language={ofName('ts')} />)
    expect(container.textContent).toBe(hostile)
    expect(container.querySelector('script')).toBeNull()
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('b')).toBeNull()
  })

  it('keeps every character of a file it does colour', () => {
    const source = 'const a = "</span>" // <b>\n'
    const { container } = render(<Painted text={source} language={ofPath('a.ts')} />)
    expect(container.textContent).toBe(source)
  })
})

describe('the line numbers', () => {
  it('counts the lines the file has', () => {
    const { container } = render(<Code text={'a\nb\nc'} language={null} />)
    expect(container.querySelector('.code__lines')!.textContent).toBe('1\n2\n3\n')
  })

  it('counts one for an empty file, because there is a line to type on', () => {
    const { container } = render(<Code text="" language={null} />)
    expect(container.querySelector('.code__lines')!.textContent).toBe('1\n')
  })

  it('counts the empty last line a trailing newline makes', () => {
    const { container } = render(<Code text={'a\n'} language={null} />)
    expect(container.querySelector('.code__lines')!.textContent).toBe('1\n2\n')
  })

  it('is one text node rather than one element per line', () => {
    // Ten thousand elements laid out on every keystroke, for a column nobody
    // clicks, is the whole reason this is a string.
    const { container } = render(<Code text={'x\n'.repeat(500)} language={null} />)
    expect(container.querySelector('.code__lines')!.children).toHaveLength(0)
  })

  it('is hidden from anything that reads the page', () => {
    const { container } = render(<Code text="a" language={null} />)
    expect(container.querySelector('.code__lines')!.getAttribute('aria-hidden')).toBe('true')
  })
})

/*
 * Tab was leaving the editor, because that is what Tab does in a textarea.
 * Keeping the field was worth it — selection, undo, composition and every
 * accessibility affordance come free — so the keys it does not handle are
 * handled by hand, and this is where that is checked end to end.
 */
describe('the keys an editor has to have', () => {
  /** Types a key into the field and reports what the editor tried to write. */
  function press(
    text: string,
    key: string,
    at: number,
    over: { shiftKey?: boolean } = {},
  ): { text: string; prevented: boolean } {
    let written = text
    // Rendered and torn down here, so a test may press more than one key
    // without two editors answering to the same role.
    const view = render(<Code text={text} language={null} onChange={(next) => (written = next)} />)
    const field = view.getByRole('textbox') as HTMLTextAreaElement
    field.setSelectionRange(at, at)
    // jsdom has no execCommand, so the fallback path is what runs — which is
    // the path that has to work when a browser refuses the command too.
    // `cancelable`, or `preventDefault()` is a no-op and the test proves
    // nothing about whether the key was taken.
    const event = new KeyboardEvent('keydown', {
      key,
      bubbles: true,
      cancelable: true,
      ...over,
    })
    fireEvent(field, event)
    const said = { text: written === text ? field.value : written, prevented: event.defaultPrevented }
    view.unmount()
    return said
  }

  it('indents instead of moving the focus', () => {
    const { text, prevented } = press('const a', 'Tab', 0)
    expect(prevented).toBe(true)
    expect(text).toBe('  const a')
  })

  it('backs out of the indent with shift', () => {
    // One step, and the step is the file's own: this file indents by four, so
    // one outdent is four. A file indented by two would give back two.
    expect(press('    x', 'Tab', 4, { shiftKey: true }).text).toBe('x')
    // Cursor just past the two spaces on line two; the file's step is two,
    // because two is the smallest indent anything in it took.
    expect(press('a\n  b\n    c', 'Tab', 4, { shiftKey: true }).text).toBe('a\nb\n    c')
  })

  it('keeps the indentation on the next line', () => {
    const { text } = press('    let a = 1', 'Enter', 13)
    expect(text).toBe('    let a = 1\n    ')
  })

  it('gives the focus back on Escape, so Tab can move on', () => {
    // Trapping Tab with no way out is a pane somebody cannot leave.
    render(<Code text="x" language={null} />)
    const field = screen.getByRole('textbox') as HTMLTextAreaElement
    field.focus()
    expect(document.activeElement).toBe(field)
    fireEvent.keyDown(field, { key: 'Escape' })
    expect(document.activeElement).not.toBe(field)
  })

  it('leaves Tab alone in a read-only pane', () => {
    render(<Code text="x" language={null} readOnly />)
    const field = screen.getByRole('textbox')
    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true })
    fireEvent(field, event)
    expect(event.defaultPrevented).toBe(false)
  })

  it('leaves every key alone while a character is being composed', () => {
    // An IME can take Tab to pick a candidate. Stealing it then would indent
    // the file instead of choosing a character.
    render(<Code text="x" language={null} />)
    const field = screen.getByRole('textbox')
    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true })
    Object.defineProperty(event, 'isComposing', { value: true })
    fireEvent(field, event)
    expect(event.defaultPrevented).toBe(false)
  })
})
