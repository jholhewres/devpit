import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { FileContents } from '../gen/bindings'
import { FilePane } from './FilePane'
import { Picture } from './Picture'

afterEach(cleanup)

const PNG = 'data:image/png;base64,iVBORw0KGgo='

let read: FileContents

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: { fileRead: () => read, pathRead: () => read, pathReveal: () => null },
}))
vi.mock('./useShell', () => ({
  useShell: () => ({ project: { id: 'prj' }, active: null, close: vi.fn(), markUnsaved: vi.fn() }),
}))
vi.mock('./revealLine', () => ({ useWantedLine: () => null }))

function picture(path: string, over: Partial<FileContents> = {}): FileContents {
  return {
    path,
    fullPath: `/w/${path}`,
    text: null,
    notShown: null,
    bytes: 3 * 1024 * 1024,
    kind: 'image',
    dataUrl: PNG,
    readAt: 1,
    ...over,
  } as FileContents
}

describe('a picture in a tab', () => {
  it('is drawn from the data the read handed back', async () => {
    read = picture('docs/shot.png')
    render(<FilePane tab={{ id: 't', kind: 'file', title: 'shot.png', path: 'docs/shot.png' } as never} />)
    const img = await waitFor(() => screen.getByAltText('shot.png'))
    expect(img.getAttribute('src')).toBe(PNG)
  })

  it('says why when it is too big to draw, rather than a blank tab', async () => {
    read = picture('huge.png', { dataUrl: null, notShown: 'huge.png is 9.0 MB — past the 8 MB this draws' })
    render(<FilePane tab={{ id: 't', kind: 'file', title: 'huge.png', path: 'huge.png' } as never} />)
    await waitFor(() => screen.getByText(/past the 8 MB this draws/))
    expect(screen.queryByRole('img')).toBeNull()
  })
})

describe('a picture the window cannot decode', () => {
  it('says so instead of drawing nothing', () => {
    render(<Picture src={PNG} name="broken.webp" size={10} />)
    fireEvent.error(screen.getByAltText('broken.webp'))
    expect(screen.getByText('This window could not draw broken.webp.')).toBeTruthy()
    expect(screen.queryByRole('img')).toBeNull()
  })
})
