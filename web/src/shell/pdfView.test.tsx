import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { bytesOf, PAGES_AT_ONCE, PdfView } from './PdfView'

const opened = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { pathOpen: (path: string) => (opened(path), null) },
}))

let pages = 3
let broken = false
const drawn = vi.fn()
vi.mock('pdfjs-dist/build/pdf.worker.min.mjs?url', () => ({ default: '/worker.js' }))
vi.mock('pdfjs-dist', () => ({
  GlobalWorkerOptions: { workerSrc: '' },
  getDocument: () => ({
    promise: broken
      ? Promise.reject(new Error('not a pdf'))
      : Promise.resolve({
          numPages: pages,
          getPage: async (n: number) => ({
            getViewport: ({ scale }: { scale: number }) => ({ width: 600 * scale, height: 800 * scale }),
            render: () => ((drawn(n), { promise: Promise.resolve() })),
          }),
        }),
  }),
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  pages = 3
  broken = false
})

describe('a PDF in a tab', () => {
  it('reads the bytes a data URL carries', () => {
    expect([...bytesOf('data:application/pdf;base64,JVBERg==')]).toEqual([37, 80, 68, 70])
  })

  it('draws its pages, and offers the system viewer', async () => {
    render(<PdfView dataUrl="data:application/pdf;base64,JVBERg==" fullPath="/w/a.pdf" name="a.pdf" />)
    await waitFor(() => expect(screen.getAllByLabelText(/^Page \d$/)).toHaveLength(3))
    await waitFor(() => expect(drawn).toHaveBeenCalledTimes(3))
    fireEvent.click(screen.getByText('Open in the system viewer'))
    await waitFor(() => expect(opened).toHaveBeenCalledWith('/w/a.pdf'))
  })

  it('draws a long one a stretch at a time', async () => {
    pages = PAGES_AT_ONCE + 4
    render(<PdfView dataUrl="data:application/pdf;base64,JVBERg==" fullPath="/w/a.pdf" name="a.pdf" />)
    await waitFor(() => expect(screen.getAllByLabelText(/^Page \d+$/)).toHaveLength(PAGES_AT_ONCE))
    fireEvent.click(screen.getByText('Show 4 more'))
    expect(screen.getAllByLabelText(/^Page \d+$/)).toHaveLength(PAGES_AT_ONCE + 4)
  })

  it('says so when it cannot be read, with the way out', async () => {
    broken = true
    render(<PdfView dataUrl="data:application/pdf;base64,AAAA" fullPath="/w/b.pdf" name="b.pdf" />)
    await waitFor(() => screen.getByText('This window could not draw b.pdf.'))
    expect(screen.getByText('Open in the system viewer')).toBeTruthy()
  })
})
