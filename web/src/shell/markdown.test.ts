import { describe, expect, it } from 'vitest'

import { blocks, external, resolved, spans } from './markdown'

describe('markdown blocks', () => {
  it('reads a heading with its level', () => {
    expect(blocks('### Deep')).toEqual([{ kind: 'heading', level: 3, text: 'Deep' }])
  })

  it('keeps a fenced block whole, with its language', () => {
    const found = blocks('```rust\nfn main() {}\n\n# not a heading\n```')
    expect(found).toEqual([
      { kind: 'code', language: 'rust', text: 'fn main() {}\n\n# not a heading' },
    ])
  })

  /* An unclosed fence is a fence. Reading on as markdown would render half a
     code block as headings and lists. */
  it('runs an unclosed fence to the end of the file', () => {
    const found = blocks('```\nstill code\n# still code')
    expect(found).toEqual([{ kind: 'code', language: '', text: 'still code\n# still code' }])
  })

  it('gathers a list into one block', () => {
    expect(blocks('- one\n- two')).toEqual([
      { kind: 'list', ordered: false, items: ['one', 'two'] },
    ])
  })

  it('tells an ordered list from an unordered one', () => {
    expect(blocks('1. one\n2. two')[0]).toMatchObject({ ordered: true })
  })

  /* Nothing here can produce markup, so raw HTML in the source is text. */
  it('treats raw html as text rather than as markup', () => {
    expect(blocks('<script>alert(1)</script>')).toEqual([
      { kind: 'paragraph', text: '<script>alert(1)</script>' },
    ])
  })
})

describe('markdown spans', () => {
  it('reads bold before italic, so ** is not two *', () => {
    expect(spans('a **b** c')).toEqual([
      { kind: 'text', text: 'a ' },
      { kind: 'strong', text: 'b' },
      { kind: 'text', text: ' c' },
    ])
  })

  it('reads an image apart from a link', () => {
    expect(spans('![alt](a.png)')).toEqual([{ kind: 'image', alt: 'alt', src: 'a.png' }])
    expect(spans('[label](https://x)')).toEqual([
      { kind: 'link', text: 'label', href: 'https://x' },
    ])
  })

  it('leaves code spans alone', () => {
    expect(spans('use `cargo test`')).toEqual([
      { kind: 'text', text: 'use ' },
      { kind: 'code', text: 'cargo test' },
    ])
  })
})

describe('where a link goes', () => {
  it('knows a link that leaves the machine', () => {
    expect(external('https://example.com')).toBe(true)
    expect(external('mailto:a@b.c')).toBe(true)
    expect(external('./notes.md')).toBe(false)
  })
})

describe('an image path in a markdown file', () => {
  it('resolves against the file, not the project root', () => {
    expect(resolved('docs/a/b.md', './logo.png')).toBe('docs/a/logo.png')
    expect(resolved('docs/a/b.md', '../logo.png')).toBe('docs/logo.png')
    expect(resolved('README.md', 'web/hero.png')).toBe('web/hero.png')
  })

  it('leaves an absolute or external source alone', () => {
    expect(resolved('docs/b.md', 'https://x/a.png')).toBe('https://x/a.png')
    expect(resolved('docs/b.md', '/etc/a.png')).toBe('/etc/a.png')
  })
})
