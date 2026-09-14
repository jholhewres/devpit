import { describe, expect, it } from 'vitest'

import { onEnter, onTab, unit, type Edit } from './indenting'

/** The text an edit produces, and where the cursor lands, as `a|b` or `a[b]c`. */
function after(text: string, edit: Edit | null, from = 0, to = 0): string {
  // No edit means the key was left to do whatever it does, and the text and
  // the cursor are exactly where they were.
  if (!edit) {
    return from === to
      ? `${text.slice(0, from)}|${text.slice(from)}`
      : `${text.slice(0, from)}[${text.slice(from, to)}]${text.slice(to)}`
  }
  const next = text.slice(0, edit.from) + edit.insert + text.slice(edit.to)
  const [head, tail] = edit.select
  return head === tail
    ? `${next.slice(0, head)}|${next.slice(head)}`
    : `${next.slice(0, head)}[${next.slice(head, tail)}]${next.slice(tail)}`
}

/** `a|b` or `a[b]c`, taken apart into text and selection. */
function at(marked: string): [string, number, number] {
  if (marked.includes('|')) {
    const from = marked.indexOf('|')
    return [marked.replace('|', ''), from, from]
  }
  const from = marked.indexOf('[')
  const to = marked.indexOf(']') - 1
  return [marked.replace('[', '').replace(']', ''), from, to]
}

const tab = (marked: string, step = '  ', shifted = false): string => {
  const [text, from, to] = at(marked)
  return after(text, onTab(text, from, to, shifted, step), from, to)
}

const enter = (marked: string, step = '  '): string => {
  const [text, from, to] = at(marked)
  return after(text, onEnter(text, from, to, step))
}

describe('what one step in looks like in this file', () => {
  it('reads two from a file indented by two', () => {
    expect(unit('const a = {\n  b: 1,\n  c: {\n    d: 2,\n  },\n}')).toBe('  ')
  })

  it('reads four from a file indented by four', () => {
    expect(unit('fn a() {\n    let b = 1;\n    if c {\n        d();\n    }\n}')).toBe('    ')
  })

  it('reads tabs from a file that uses them', () => {
    expect(unit('x\n\ta\n\t\tb\n\tc')).toBe('\t')
  })

  it('falls back to two for a file that has not indented anything yet', () => {
    expect(unit('one line')).toBe('  ')
    expect(unit('')).toBe('  ')
  })

  it('is not fooled by a deeply nested line', () => {
    // The smallest step anybody took is the step. A file indented by two has
    // plenty of lines at four and six and none at one.
    expect(unit('a\n      deep\n  shallow\n    middle')).toBe('  ')
  })

  it('does not count whitespace on an otherwise empty line', () => {
    expect(unit('a\n   \n    real')).toBe('    ')
  })
})

describe('Tab, with nothing selected', () => {
  it('puts in a step instead of moving the focus', () => {
    // The bug: Tab left the editor entirely.
    expect(tab('|const a')).toBe('  |const a')
  })

  it('goes to the next stop rather than a fixed number of spaces', () => {
    // At column 3 with a step of two, Tab reaches column 4.
    expect(tab('a|bc')).toBe('a |bc')
    expect(tab('ab|c')).toBe('ab  |c')
  })

  it('puts in one tab where the file uses tabs', () => {
    expect(tab('|x', '\t')).toBe('\t|x')
  })

  it('counts the column from the line, not from the file', () => {
    expect(tab('hello\na|bc')).toBe('hello\na |bc')
  })
})

describe('Shift-Tab, with nothing selected', () => {
  it('backs out of the indentation', () => {
    expect(tab('    |x', '  ', true)).toBe('  |x')
  })

  it('takes off one tab where the file uses tabs', () => {
    expect(tab('\t\t|x', '\t', true)).toBe('\t|x')
  })

  it('does nothing at the start of a line', () => {
    expect(tab('|x', '  ', true)).toBe('|x')
  })

  it('does nothing in the middle of a word', () => {
    // Eating characters there would be a surprise, not an outdent.
    expect(tab('  ab|cd', '  ', true)).toBe('  ab|cd')
  })

  it('takes what there is when there is less than a step', () => {
    expect(tab(' |x', '  ', true)).toBe('|x')
  })
})

describe('Tab, over more than one line', () => {
  it('moves every line the selection touches', () => {
    expect(tab('[a\nb]\nc')).toBe('[  a\n  b]\nc')
  })

  it('moves the last line even when the selection stops halfway through it', () => {
    expect(tab('a\n[b\nc]d\ne')).toBe('a\n[  b\n  c]d\ne')
  })

  it('leaves an empty line empty', () => {
    // Whitespace on a line with nothing on it is whitespace to delete later.
    expect(tab('[a\n\nb]')).toBe('[  a\n\n  b]')
  })

  it('takes a step back off every line', () => {
    expect(tab('[    a\n    b]', '  ', true)).toBe('[  a\n  b]')
  })

  it('leaves a line alone that has nothing to give back', () => {
    expect(tab('[a\n  b]', '  ', true)).toBe('[a\nb]')
  })

  it('keeps the lines selected, so it can be pressed again', () => {
    const once = tab('[a\nb]')
    expect(once).toContain('[')
    expect(once).toContain(']')
  })
})

describe('Enter', () => {
  it('opens the next line where this one opened', () => {
    expect(enter('    let a = 1|')).toBe('    let a = 1\n    |')
  })

  it('starts at the margin when this line did', () => {
    expect(enter('a|')).toBe('a\n|')
  })

  it('takes one more step after something that opens', () => {
    expect(enter('fn a() {|')).toBe('fn a() {\n  |')
    expect(enter('  const a = [|')).toBe('  const a = [\n    |')
    expect(enter('def f():|')).toBe('def f():\n  |')
  })

  it('ignores trailing space when deciding whether a line opened', () => {
    expect(enter('fn a() {   |')).toBe('fn a() {   \n  |')
  })

  it('replaces the selection rather than leaving it behind', () => {
    expect(enter('  a[bc]d')).toBe('  a\n  |d')
  })

  it('uses the file\'s own step for the extra one', () => {
    expect(enter('fn a() {|', '\t')).toBe('fn a() {\n\t|')
  })
})
