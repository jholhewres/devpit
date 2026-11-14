import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ignoringWhitespace, parse } from './diff'
import { DiffFiles } from './DiffFiles'
import { spans } from './markdown'
import { Markdown } from './MarkdownView'
import { clashWith, defaultKeys, shortcutNamed, whenOf } from './shortcuts'
import { kept } from './strip'
import { LookSettings } from './ViewSettings'
import { rereadViewPrefs, setViewPref, viewPrefs } from './viewPrefs'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

afterEach(() => {
  cleanup()
  localStorage.clear()
  rereadViewPrefs()
})

describe('LaTeX maths', () => {
  it('stays text while off, as a dollar sign always was', () => {
    expect(spans('area $a^2$ here')).toEqual([{ kind: 'text', text: 'area $a^2$ here' }])
  })

  it('reads $…$, $$…$$, \\(…\\) and \\[…\\] while on', () => {
    expect(spans('area $a^2$ here', true)).toEqual([
      { kind: 'text', text: 'area ' },
      { kind: 'math', text: 'a^2', display: false },
      { kind: 'text', text: ' here' },
    ])
    expect(spans('$$\\sum_i x_i$$', true)).toEqual([{ kind: 'math', text: '\\sum_i x_i', display: true }])
    expect(spans('\\(x\\) and \\[y\\]', true)).toEqual([
      { kind: 'math', text: 'x', display: false },
      { kind: 'text', text: ' and ' },
      { kind: 'math', text: 'y', display: true },
    ])
  })

  it('leaves prices and code alone', () => {
    expect(spans('costs $5 and $10 today', true)).toEqual([{ kind: 'text', text: 'costs $5 and $10 today' }])
    expect(spans('`$x$`', true)).toEqual([{ kind: 'code', text: '$x$' }])
  })

  it('draws a formula as MathML when on, and the source as text when off', async () => {
    const { container, unmount } = render(<Markdown source="So $x^2$." />)
    expect(container.querySelector('math')).toBeNull()
    expect(container.textContent).toContain('$x^2$')
    unmount()

    setViewPref('latex', true)
    const drawn = render(<Markdown source="So $x^2$." />)
    await waitFor(() => expect(drawn.container.querySelector('.md__math math')).not.toBeNull())
    expect(drawn.container.querySelector('msup')).not.toBeNull()
  })
})

const WHITESPACE_ONLY = [
  'diff --git a/a.ts b/a.ts',
  '@@ -1,3 +1,3 @@',
  ' const a = 1',
  '-if (a){',
  '+if (a) {',
  ' }',
  'diff --git a/b.ts b/b.ts',
  '@@ -1,2 +1,2 @@',
  '-old',
  '+new',
  ' same',
].join('\n')

describe('whitespace in diffs', () => {
  it('drops a hunk whose only change is spacing, and keeps a real change', () => {
    const [spacing, real] = parse(WHITESPACE_ONLY)
    expect(ignoringWhitespace(spacing!.hunks)).toEqual([])
    expect(ignoringWhitespace(real!.hunks)).toEqual(real!.hunks)
  })

  it('draws a spaced line once, as it is now, beside the lines that really changed', () => {
    const [hunk] = parse(['diff --git a/c b/c', '@@ -1,2 +1,2 @@', '-a  =  1', '-b', '+a = 1', '+c'].join('\n'))[0]!.hunks
    expect(ignoringWhitespace([hunk!])[0]!.rows.map((row) => row.kind)).toEqual(['del', 'del', 'add', 'add'])
    const [moved] = parse(['diff --git a/c b/c', '@@ -1,2 +1,2 @@', '-a  =  1', '+a = 1', ' b', '-x', '+y'].join('\n'))[0]!.hunks
    expect(ignoringWhitespace([moved!])[0]!.rows).toEqual([
      { kind: 'same', text: 'a = 1' },
      { kind: 'same', text: 'b' },
      { kind: 'del', text: 'x' },
      { kind: 'add', text: 'y' },
    ])
  })

  it('hides the spacing-only file from a card’s changes once chosen', () => {
    const { container, unmount } = render(<DiffFiles diff={WHITESPACE_ONLY} />)
    expect(container.querySelectorAll('.cdiff__h')).toHaveLength(2)
    unmount()
    setViewPref('diffWhitespace', 'ignore')
    const ignored = render(<DiffFiles diff={WHITESPACE_ONLY} />)
    expect(ignored.container.querySelectorAll('.cdiff__h')).toHaveLength(1)
  })
})

describe('files in a diff', () => {
  it('open folded, as before, until chosen open', () => {
    const { container, unmount } = render(<DiffFiles diff={WHITESPACE_ONLY} />)
    expect([...container.querySelectorAll('details.cdiff__f')].map((one) => one.hasAttribute('open'))).toEqual([false, false])
    unmount()
    setViewPref('diffCollapsed', false)
    const opened = render(<DiffFiles diff={WHITESPACE_ONLY} />)
    expect([...opened.container.querySelectorAll('details.cdiff__f')].map((one) => one.hasAttribute('open'))).toEqual([true, true])
  })
})

describe('Settings → Appearance', () => {
  it('keeps each pick a row makes', () => {
    render(<LookSettings />)
    fireEvent.click(screen.getByRole('radio', { name: 'Blue and orange' }))
    fireEvent.click(screen.getByRole('radio', { name: 'Stacked' }))
    fireEvent.click(within('Text size', 'Large'))
    expect(viewPrefs()).toMatchObject({ diffColours: 'safe', diffLayout: 'stacked', textSize: 'large' })
    expect(document.documentElement.dataset.diffColours).toBe('safe')
    expect(within('Text size', 'Large').getAttribute('aria-checked')).toBe('true')
  })
})

const within = (group: string, name: string): HTMLElement => {
  const box = screen.getByRole('radiogroup', { name: group })
  return [...box.querySelectorAll<HTMLElement>('[role=radio]')].find((one) => one.textContent === name)!
}

describe('the new keys', () => {
  it('take keys nothing else answers to where they work', () => {
    for (const id of ['goToTab', 'reopenTab', 'stashDraft', 'sendBackground'] as const) {
      for (const spec of defaultKeys(id, false)) expect(clashWith(id, spec, false), `${id} ${spec}`).toBeNull()
    }
  })

  it('say when they act', () => {
    expect(whenOf(shortcutNamed('stashDraft'))).toBe('When the composer has focus.')
    expect(whenOf(shortcutNamed('steerQueued'))).toBe('When the composer has focus and a turn is running.')
    expect(whenOf(shortcutNamed('reopenTab'))).toBeNull()
  })
})

describe('reopening a closed tab', () => {
  it('brings back a chat or a file, newest last, and never a terminal whose shells ended', () => {
    const chat = { id: 'c1', kind: 'chat' as const }
    const file = { id: 'file:a.ts', kind: 'file' as const, path: 'a.ts' }
    const term = { id: 't1', kind: 'term' as const }
    expect(kept(kept(kept([], chat), term), file)).toEqual([chat, file])
    /* Closed twice, it is kept once, at the end. */
    expect(kept([chat, file], chat)).toEqual([file, chat])
  })
})
