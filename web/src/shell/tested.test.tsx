import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Tested } from '../gen/bindings'
import { TestedPane } from './TestedPane'
import { countsWords, leftNoReport, unlistedWords } from './tested'

afterEach(cleanup)

const tested = (over: Partial<Tested> = {}): Tested => ({
  passed: 3,
  failed: 0,
  skipped: 0,
  failures: [],
  readFrom: ['vitest'],
  ...over,
})

let answer: Tested = tested()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { checkpointTested: () => answer },
}))

describe('what a run’s tests said, in words', () => {
  it('says the counts and where they were read from', () => {
    expect(countsWords(tested({ passed: 2, failed: 1, skipped: 1, readFrom: ['vitest', 'cargo test'] }))).toBe(
      '2 passed, 1 failed, 1 skipped — read from vitest, cargo test.',
    )
  })

  it('tells a run with no report from one whose report had nothing failing', () => {
    expect(leftNoReport(tested({ readFrom: [] }))).toBe(true)
    expect(leftNoReport(tested())).toBe(false)
  })

  it('says how many failures the list left out', () => {
    const failure = { name: 'adds', file: null, message: null }
    expect(unlistedWords(tested({ failed: 1, failures: [failure] }))).toBeNull()
    expect(unlistedWords(tested({ failed: 60, failures: [failure] }))).toBe('And 59 more not listed.')
  })
})

describe('the tests pane', () => {
  it('names each failed test, its file and why', async () => {
    answer = tested({
      passed: 1,
      failed: 1,
      failures: [{ name: 'the answer is wrong', file: 'src/fail.test.ts', message: 'expected 41 to be 42' }],
    })
    render(<TestedPane runId="run_1" />)
    await waitFor(() => expect(screen.getByText(/the answer is wrong/)).toBeTruthy())
    expect(screen.getByText(/src\/fail\.test\.ts/)).toBeTruthy()
    expect(screen.getByText('expected 41 to be 42')).toBeTruthy()
    expect(screen.getByText(/1 passed, 1 failed/)).toBeTruthy()
  })

  it('draws nothing for a run that left no report', async () => {
    answer = tested({ readFrom: [] })
    const { container } = render(<TestedPane runId="run_1" />)
    await new Promise((settled) => setTimeout(settled, 0))
    expect(container.innerHTML).toBe('')
  })
})
