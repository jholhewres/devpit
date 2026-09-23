import { describe, expect, it } from 'vitest'

import { droppedPaths, keptAsPicture } from './pasting'

describe('droppedPaths', () => {
  it('reads the paths a file manager names, decoded', () => {
    const data = { getData: (type: string) => (type === 'text/uri-list' ? 'file:///home/a/My%20shot.png\r\nfile:///home/a/b.txt\r\n' : '') }
    expect(droppedPaths(data as never)).toEqual(['/home/a/My shot.png', '/home/a/b.txt'])
  })

  it('names nothing when the drop is not files', () => {
    expect(droppedPaths(null)).toEqual([])
  })

  it('drops only the entry that does not parse, and files on another host', () => {
    const list = 'file:///home/a/%E0%A4%A.png\nfile://nas/share/x.txt\nfile://localhost/home/a/ok.txt\n'
    const data = { getData: () => list }
    expect(droppedPaths(data as never)).toEqual(['/home/a/ok.txt'])
  })
})

describe('keptAsPicture', () => {
  it('keeps what the chat can paste, and leaves the rest to be attached by path', () => {
    expect(keptAsPicture(new Blob(['x'], { type: 'image/png' }))).toBe(true)
    expect(keptAsPicture(new Blob(['x'], { type: 'image/svg+xml' }))).toBe(false)
    expect(keptAsPicture({ type: 'image/jpeg', size: 9 * 1024 * 1024 } as Blob)).toBe(false)
  })
})
