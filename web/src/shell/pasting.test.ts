import { describe, expect, it } from 'vitest'

import { droppedPaths } from './pasting'

describe('droppedPaths', () => {
  it('reads the paths a file manager names, decoded', () => {
    const data = { getData: (type: string) => (type === 'text/uri-list' ? 'file:///home/a/My%20shot.png\r\nfile:///home/a/b.txt\r\n' : '') }
    expect(droppedPaths(data as never)).toEqual(['/home/a/My shot.png', '/home/a/b.txt'])
  })

  it('names nothing when the drop is not files', () => {
    expect(droppedPaths(null)).toEqual([])
  })
})
