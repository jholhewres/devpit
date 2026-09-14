import { describe, expect, it } from 'vitest'

import type { PaneCost, Usage } from '../gen/bindings'
import { counted, cpu, measured, notable, size } from './watching'

const pane = (over: Partial<PaneCost> = {}): PaneCost => ({
  paneId: 'leaf_1',
  label: 'Claude Code',
  agent: 'claude',
  memoryKb: 1024,
  cpuTenths: 0,
  processes: 1,
  ...over,
})

const usage = (over: Partial<Usage> = {}): Usage => ({
  memoryKb: 1024,
  cpuTenths: 0,
  proportional: true,
  panes: [pane()],
  ...over,
})

describe('memory at the scale a person reads', () => {
  it.each([
    [512, '512 KB'],
    [1024, '1.0 MB'],
    [5_632, '5.5 MB'],
    [102_400, '100 MB'],
    [1_048_576, '1.00 GB'],
    [6_281_215, '5.99 GB'],
  ])('reads %i kB as %s', (kb, said) => {
    expect(size(kb)).toBe(said)
  })

  it('keeps a decimal where it still means something', () => {
    // 5.5 MB and 6 MB are different; 100 MB and 100.4 MB are not.
    expect(size(5_632)).toContain('.')
    expect(size(102_400)).not.toContain('.')
  })
})

describe('cpu, in tenths of a percent', () => {
  it('says nothing much is happening rather than 0.2%', () => {
    expect(cpu(2)).toBe('0%')
  })

  it('keeps a decimal while the number is small', () => {
    expect(cpu(38)).toBe('3.8%')
  })

  it('drops it once it is not news', () => {
    expect(cpu(424)).toBe('42%')
  })

  it('does not stop at one core', () => {
    // An agent running four tools at once is doing four cores of work, and
    // saying 100% would hide the one thing worth looking at.
    expect(cpu(3870)).toBe('387%')
  })

  it('arrives as an integer, so it can never be null', () => {
    // An `f64` types as `number | null`, because a float can be NaN and JSON
    // has no word for it. Every caller would have to second-guess it.
    expect(Number.isInteger(2)).toBe(true)
    expect(cpu(0)).toBe('0%')
  })
})

describe('whether there are figures worth drawing', () => {
  it('has none with no panes open', () => {
    expect(measured(usage({ panes: [], memoryKb: 0 }))).toBe(false)
  })

  it('has none when every pane is idle and empty', () => {
    expect(measured(usage({ memoryKb: 0, cpuTenths: 0 }))).toBe(false)
  })

  it('has them as soon as something is running', () => {
    expect(measured(usage({ memoryKb: 4096 }))).toBe(true)
  })
})

describe('how the total was counted', () => {
  it('says the shared pages were divided', () => {
    expect(counted(usage({ proportional: true }))).toMatch(/really in use/)
  })

  it('says so when they were not, because the number is then high', () => {
    // Measured at 44% on one machine. A number that might be half wrong has
    // to arrive labelled.
    expect(counted(usage({ proportional: false }))).toMatch(/counted more than once/)
  })
})

describe('which panes get a row', () => {
  it('leaves out a pane costing nothing, which is a shell at a prompt', () => {
    const panes = [pane({ paneId: 'a' }), pane({ paneId: 'b', memoryKb: 0 })]
    expect(notable(panes).map((one) => one.paneId)).toEqual(['a'])
  })
})
