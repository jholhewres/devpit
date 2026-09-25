import { describe, expect, it } from 'vitest'

import type { Part } from '../gen/bindings'
import {
  actionLabel,
  acts,
  headline,
  isAgent,
  kindOf,
  runningName,
  summary,
  targetOf,
  toolName,
  grouped,
  groupTarget,
  isGroup,
  type Act,
  type Group,
} from './acts'

const call = (id: string, name: string, input = '{}', state: 'running' | 'ok' | 'failed' = 'ok'): Part =>
  ({ kind: 'tool_call', id, name, input, state, parent: null })

const result = (callId: string, output: string, isError = false): Part =>
  ({ kind: 'tool_result', call_id: callId, output, is_error: isError, parent: null })

describe('what a tool call is', () => {
  it('reads the act through the name, whatever the spelling', () => {
    expect(kindOf('Bash')).toBe('run')
    // Claude Code 2.1.270 keeps its checklist in these, not TodoWrite.
    expect(kindOf('TaskCreate')).toBe('plan')
    expect(kindOf('TaskUpdate')).toBe('plan')
    expect(kindOf('run_command')).toBe('run')
    expect(kindOf('runTerminalCommand')).toBe('run')
    expect(kindOf('Edit')).toBe('edit')
    expect(kindOf('apply_patch')).toBe('edit')
    expect(kindOf('Read')).toBe('read')
    expect(kindOf('Grep')).toBe('find')
    expect(kindOf('Glob')).toBe('find')
    expect(kindOf('LS')).toBe('list')
    expect(kindOf('TodoWrite')).toBe('plan')
  })

  it('looks past an MCP namespace to the leaf', () => {
    /* `mcp__fs__read_file` is a read; the server it came from is not the act. */
    expect(kindOf('mcp__fs__read_file')).toBe('read')
    expect(kindOf('shell.execute')).toBe('run')
  })

  it('calls a tool it does not know a tool, rather than guessing', () => {
    expect(kindOf('mcp__figma__get_design_context')).toBe('tool')
    expect(kindOf('WebSearch')).toBe('find')
  })
})

describe('the subject on the row', () => {
  it('prefers what the caller meant over how it said it', () => {
    /* `description` is the sentence a person wrote; `command` is shell. */
    const input = JSON.stringify({ command: 'cargo test --all', description: 'Run the test suite' })
    expect(targetOf('run', input)).toBe('Run the test suite')
  })

  it('falls back to the command when nobody described it', () => {
    expect(targetOf('run', JSON.stringify({ command: 'cargo test' }))).toBe('cargo test')
  })

  it('names a file by its file, not by its path', () => {
    const input = JSON.stringify({ file_path: '/home/j/devpit/web/src/shell/Turn.tsx' })
    expect(targetOf('edit', input)).toBe('Turn.tsx')
    expect(targetOf('read', input)).toBe('Turn.tsx')
  })

  it('keeps a search to its pattern', () => {
    expect(targetOf('find', JSON.stringify({ pattern: 'ActivityKind' }))).toBe('ActivityKind')
  })

  it('cuts a command down to one line', () => {
    const input = JSON.stringify({ command: 'cat <<EOF\nline one\nline two\nEOF' })
    expect(targetOf('run', input)).toBe('cat <<EOF')
  })

  it('has nothing to say about arguments it cannot read', () => {
    expect(targetOf('run', 'not json')).toBe('')
    expect(targetOf('edit', '{}')).toBe('')
  })
})

describe('a tool that names itself badly', () => {
  it('reads the leaf as words', () => {
    expect(toolName('mcp__figma__get_design_context')).toBe('Get design context')
    expect(toolName('WebFetch')).toBe('Web Fetch')
  })
})

describe('the rows of a message', () => {
  it('pairs a result into the call it answers rather than listing it', () => {
    /* Two parts, one row: the result is the end of the call, not an event
       of its own. */
    const rows = acts([call('c1', 'Bash', JSON.stringify({ command: 'ls' })), result('c1', 'a\nb')])
    expect(rows).toHaveLength(1)
    expect(rows[0]).toMatchObject({ kind: 'run', target: 'ls', output: 'a\nb', done: true, failed: false })
  })

  it('carries the failure from the result', () => {
    const rows = acts([call('c1', 'Bash'), result('c1', 'no such file', true)])
    expect(rows[0]!.failed).toBe(true)
  })

  it('leaves a call still running unfinished', () => {
    expect(acts([call('c1', 'Read', '{}', 'running')])[0]!.done).toBe(false)
  })

  it('gives a thought a row of its own', () => {
    const rows = acts([{ kind: 'thinking', text: 'First I should read the file.\nThen edit it.', parent: null }])
    expect(rows[0]).toMatchObject({ kind: 'think', target: 'First I should read the file.' })
  })

  it('names an unclassifiable tool instead of leaving the row blank', () => {
    expect(acts([call('c1', 'mcp__figma__get_design_context')])[0]!.target)
      .toBe('Get design context')
  })
})

describe('what a group of rows amounts to', () => {
  const rows = acts([
    call('c1', 'Bash', JSON.stringify({ command: 'ls' })),
    result('c1', ''),
    call('c2', 'Bash', JSON.stringify({ command: 'pwd' })),
    result('c2', ''),
    call('c3', 'Edit', JSON.stringify({ file_path: '/a/b.ts' })),
    result('c3', ''),
  ])

  it('counts by act, in the words a person would use', () => {
    expect(summary(rows)).toBe('Ran 2 commands · 1 file edit')
  })

  it('says it is still running while one row is', () => {
    const live = acts([call('c1', 'Bash', '{}', 'running')])
    expect(summary(live)).toBe('Running 1 command')
  })

  it('names the newest row while the work is live', () => {
    /* Looking up mid-turn, you want to know what it is doing now. */
    expect(headline(rows, true)).toBe('Edit b.ts')
  })

  it('gives the tally once the work settles', () => {
    expect(headline(rows, false)).toBe('Ran 2 commands · 1 file edit')
  })

  it('says nothing when there is nothing to say', () => {
    expect(headline([], false)).toBe('')
  })
})

describe('the verb on the row', () => {
  it('is a word, not the tool name', () => {
    expect(actionLabel('run')).toBe('Run')
    expect(actionLabel('find')).toBe('Search')
  })
})

describe('what is running in a pane', () => {
  it('calls an agent what people call it', () => {
    expect(runningName('claude')).toBe('Claude Code')
    expect(runningName('CODEX')).toBe('Codex')
  })

  it('leaves an ordinary command its own name', () => {
    /* `cargo` and `vim` are already what a person would say. */
    expect(runningName('cargo')).toBe('cargo')
    expect(runningName('vim')).toBe('vim')
  })

  it('knows an agent from anything else', () => {
    expect(isAgent('claude')).toBe(true)
    expect(isAgent('zsh')).toBe(false)
  })
})

describe('a run of the same act reads as one row', () => {
  const read = (id: string, done = true, failed = false): Act => ({
    id, kind: 'read', name: 'Read', target: `${id}.rs`, input: '{}', output: '', failed, done, children: [],
  })

  it('folds three or more settled calls of one tool', () => {
    const items = grouped([read('a'), read('b'), read('c')])
    expect(items).toHaveLength(1)
    expect(isGroup(items[0]!)).toBe(true)
    expect(groupTarget(items[0] as Group)).toBe('3 files')
  })

  it('leaves two alone: a pair is a list, not a run', () => {
    expect(grouped([read('a'), read('b')]).every((item) => !isGroup(item))).toBe(true)
  })

  it('never folds a running call, which is the row being watched', () => {
    const items = grouped([read('a'), read('b'), read('c'), read('d', false)])
    expect(items).toHaveLength(2)
    expect(isGroup(items[1]!)).toBe(false)
    expect((items[1] as Act).id).toBe('d')
  })

  it('keeps a failure visible instead of counting it', () => {
    const items = grouped([read('a'), read('b'), read('x', true, true), read('c'), read('d'), read('e')])
    expect(items.map((item) => (isGroup(item) ? `g${item.rows.length}` : item.id))).toEqual(['a', 'b', 'x', 'g3'])
  })

  it('does not fold different tools of the same kind together', () => {
    const grep = (id: string): Act => ({ ...read(id), kind: 'find', name: 'Grep' })
    const glob = (id: string): Act => ({ ...read(id), kind: 'find', name: 'Glob' })
    expect(grouped([grep('a'), glob('b'), grep('c')]).some(isGroup)).toBe(false)
  })
})
