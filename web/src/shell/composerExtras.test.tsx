import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { pastedPicture } from './pasting'
import { visible } from './diff'
import { contrastFor } from './terminal'

afterEach(cleanup)

const clipboard = (items: { kind: string; type: string; file?: File }[]): DataTransfer =>
  ({ items: items.map((item) => ({ ...item, getAsFile: () => item.file ?? null })) }) as unknown as DataTransfer

describe('a paste into the composer', () => {
  it('takes the first picture on the clipboard', () => {
    const png = new File(['x'], 'shot.png', { type: 'image/png' })
    expect(pastedPicture(clipboard([{ kind: 'string', type: 'text/plain' }, { kind: 'file', type: 'image/png', file: png }]))).toBe(png)
  })

  it('leaves text alone, so typing a paste still types', () => {
    expect(pastedPicture(clipboard([{ kind: 'string', type: 'text/plain' }]))).toBeNull()
    expect(pastedPicture(null)).toBeNull()
  })

  it('does not take a file that is not a picture', () => {
    const pdf = new File(['x'], 'a.pdf', { type: 'application/pdf' })
    expect(pastedPicture(clipboard([{ kind: 'file', type: 'application/pdf', file: pdf }]))).toBeNull()
  })
})

describe('terminal contrast', () => {
  it('keeps the old rule when nobody chose', () => {
    expect(contrastFor(true, null)).toBe(1)
    expect(contrastFor(false, null)).toBe(4.5)
  })

  it('is the choice on either ground once made', () => {
    expect(contrastFor(true, 7)).toBe(7)
    expect(contrastFor(false, 7)).toBe(7)
  })
})

describe('whitespace in a diff line', () => {
  it('is drawn only where there is some', () => {
    const { container } = render(
      <pre>
        {visible('a b').map((piece, at) => (piece.space ? <span key={at} className="diff__ws">{piece.text}</span> : piece.text))}
      </pre>,
    )
    expect(container.querySelectorAll('.diff__ws')).toHaveLength(1)
    expect(screen.getByText('·')).toBeTruthy()
  })
})

describe('the terminal contrast choice', () => {
  it('selects nothing until someone picks, and says which step is on after', async () => {
    const { TerminalContrast } = await import('./TerminalContrast')
    const { rerender } = render(<TerminalContrast value={null} onPick={() => {}} />)
    expect(screen.getAllByRole('radio').every((one) => one.getAttribute('aria-checked') === 'false')).toBe(true)
    rerender(<TerminalContrast value={7} onPick={() => {}} />)
    expect(screen.getByRole('radio', { name: 'Strong' }).getAttribute('aria-checked')).toBe('true')
  })
})

describe('copying the session id', () => {
  it('puts the CLI session id on the clipboard and says so', async () => {
    const { CopySession } = await import('./CopySession')
    const { fireEvent } = await import('@testing-library/react')
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<CopySession id="9c684740-743b-4394-8b4c-c11dd543c135" />)
    fireEvent.click(screen.getByRole('button', { name: 'Copy session ID' }))
    expect(writeText).toHaveBeenCalledWith('9c684740-743b-4394-8b4c-c11dd543c135')
    expect(await screen.findByRole('button', { name: 'Copied session ID' })).toBeTruthy()
  })
})
