import { describe, expect, it } from 'vitest'

import { jump, marked, noMarks, toneOf, type Marks } from './marks'

/* A shell's whole story for one command: prompt, start, end. */
const ran = (marks: Marks, row: number, code: string | null): Marks =>
  marked(marked(marked(marks, 'prompt', null, row), 'running', null, row), 'finished', code, row)

describe('a terminal as a sequence of commands', () => {
  it('opens a block where the prompt was drawn', () => {
    const marks = marked(noMarks, 'prompt', null, 12)
    expect(marks.blocks).toEqual([{ at: 12, code: null, running: false }])
  })

  it('carries the exit code the shell reported', () => {
    const marks = ran(noMarks, 3, '1')
    expect(marks.blocks[0]).toMatchObject({ at: 3, code: 1, running: false })
  })

  it('keeps a command marked while it runs', () => {
    const marks = marked(marked(noMarks, 'prompt', null, 0), 'running', null, 0)
    expect(marks.blocks[0]!.running).toBe(true)
    expect(toneOf(marks.blocks[0]!)).toBe('running')
  })

  it('does not read a missing code as a zero', () => {
    /* Some shells report the end with no code for a bare Enter. Painting that
       green would claim a command succeeded when none ran. */
    const marks = ran(noMarks, 0, null)
    expect(marks.blocks[0]!.code).toBeNull()
    expect(toneOf(marks.blocks[0]!)).toBe('none')
  })

  it('moves a redrawn prompt rather than opening a second block', () => {
    /* A resize, a Ctrl-C or a prompt widget repainting the line all report a
       new prompt with no command in between. */
    const twice = marked(marked(noMarks, 'prompt', null, 4), 'prompt', null, 9)
    expect(twice.blocks).toHaveLength(1)
    expect(twice.blocks[0]!.at).toBe(9)
  })

  it('ignores a command start with no prompt before it', () => {
    /* Half a story. Inventing a block would put a mark on a row nobody typed
       at. */
    expect(marked(noMarks, 'running', null, 5)).toBe(noMarks)
  })

  it('ignores an end with nothing running', () => {
    expect(marked(marked(noMarks, 'prompt', null, 1), 'finished', '0', 1).blocks[0]!.code)
      .toBeNull()
  })

  it('leaves the marks alone for what it does not model', () => {
    const marks = marked(noMarks, 'prompt', null, 0)
    expect(marked(marks, 'cwd', '/tmp', 0)).toBe(marks)
    expect(marked(marks, 'title', 'vim', 0)).toBe(marks)
  })

  it('tells a failure from a success', () => {
    expect(toneOf({ at: 0, code: 0, running: false })).toBe('ok')
    expect(toneOf({ at: 0, code: 127, running: false })).toBe('failed')
  })
})

describe('jumping between commands', () => {
  const marks = ran(ran(ran(noMarks, 2, '0'), 10, '1'), 30, '0')

  it('goes to the command before where you are', () => {
    expect(jump(marks, 30, 'back')).toBe(10)
  })

  it('goes to the one after', () => {
    expect(jump(marks, 2, 'forward')).toBe(10)
  })

  it('has nowhere to go past the ends', () => {
    /* Null rather than the first row: scrolling to the top and calling it an
       answer is worse than not moving. */
    expect(jump(marks, 2, 'back')).toBeNull()
    expect(jump(marks, 30, 'forward')).toBeNull()
  })

  it('has nowhere to go with no commands at all', () => {
    expect(jump(noMarks, 0, 'back')).toBeNull()
  })
})
