import { describe, expect, it } from 'vitest'

import { folderUnder, shellWord } from './fileDrop'

describe('shellWord', () => {
  it('leaves a plain path bare', () => {
    expect(shellWord('/home/me/src/a.ts')).toBe('/home/me/src/a.ts')
  })

  it('quotes what a shell would read, and a quote inside survives', () => {
    expect(shellWord('/tmp/my file.png')).toBe("'/tmp/my file.png'")
    expect(shellWord('/tmp/a;rm -rf ~')).toBe("'/tmp/a;rm -rf ~'")
    expect(shellWord("/tmp/it's.txt")).toBe("'/tmp/it'\\''s.txt'")
  })
})

describe('folderUnder', () => {
  const row = (path: string, kind: string): Element => {
    const element = document.createElement('div')
    element.dataset.path = path
    element.dataset.kind = kind
    const inner = document.createElement('span')
    element.appendChild(inner)
    return inner
  }

  it('goes into a folder, beside a file, and to the root on no row', () => {
    expect(folderUnder(row('src/shell', 'folder'))).toBe('src/shell')
    expect(folderUnder(row('src/shell/a.ts', 'file'))).toBe('src/shell')
    expect(folderUnder(row('README.md', 'file'))).toBe('')
    expect(folderUnder(null)).toBe('')
  })
})
