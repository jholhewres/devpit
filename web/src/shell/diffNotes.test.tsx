import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { CardFindings, Finding } from '../gen/bindings'
import { DiffFiles } from './DiffFiles'
import { newLines, notesOf, tally } from './diffNotes'

const DIFF = [
  'diff --git a/src/main.rs b/src/main.rs',
  '--- a/src/main.rs',
  '+++ b/src/main.rs',
  '@@ -10,3 +10,4 @@ fn main() {',
  ' let a = 1;',
  ' let b = 2;',
  '+let lock = take();',
  ' run(a, b);',
].join('\n')

const finding = (line: number | null, severity: Finding['severity'] = 'blocking'): Finding => ({
  file: 'src/main.rs',
  line,
  severity,
  why: `the lock at ${line} is never released`,
})

const found = (standing: 'current' | 'outdated', findings: Finding[], dismissed: number[] = []): CardFindings['reviews'][number] => ({
  runId: `run_${standing}`,
  found: { findings, standing, atRevision: 'abc1234def', now: 'fff0000', rubric: null },
  dismissed,
})

describe('findings on a card diff', () => {
  it('numbers the new side of a hunk, skipping removed lines', () => {
    const hunk = { header: '@@ -10,3 +10,4 @@', rows: [{ kind: 'same' as const, text: '' }, { kind: 'del' as const, text: '' }, { kind: 'add' as const, text: '' }] }
    expect(newLines(hunk)).toEqual([10, null, 11])
  })

  it('draws a finding on the line it names, and one from another revision as outdated', () => {
    const notes = notesOf({ reviews: [found('current', [finding(12)]), found('outdated', [finding(11, 'worth')])] })
    const { container } = render(<DiffFiles diff={DIFF} notes={notes} />)
    const twelve = container.querySelector('[data-line="12"]')!.parentElement!
    expect(twelve.querySelector('.cdiff__fd')?.textContent).toContain('Blocking')
    const eleven = container.querySelector('[data-line="11"]')!.parentElement!
    const old = eleven.querySelector('.cdiff__fd')!
    expect(old.getAttribute('data-standing')).toBe('outdated')
    expect(old.textContent).toContain('Outdated · made at abc1234def')
  })

  it('counts only what was not set aside', () => {
    const notes = notesOf({ reviews: [found('current', [finding(12), finding(13, 'noted')], [0])] })
    expect(tally(notes)).toEqual({ blocking: 0, worth: 0, noted: 1 })
  })
})
