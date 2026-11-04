import { describe, expect, it } from 'vitest'

import { testReport } from './testReport.ts'

describe('test report', () => {
  it("writes vitest's JSON into DEVPIT_REPORT_DIR, where evidence reads the test names", () => {
    expect(testReport({ DEVPIT_REPORT_DIR: '/runs/card/1' })).toEqual({
      reporters: ['default', 'json'],
      outputFile: { json: '/runs/card/1/vitest.json' }
    })
  })

  it('reads a relative DEVPIT_REPORT_DIR from where pnpm was started', () => {
    const report = testReport({ DEVPIT_REPORT_DIR: '.devpit/devflow/runs/local', INIT_CWD: '/repo' })
    expect(report.outputFile).toEqual({ json: '/repo/.devpit/devflow/runs/local/vitest.json' })
  })

  it('keeps the default reporter alone outside a devflow check', () => {
    expect(testReport({})).toEqual({ reporters: ['default'] })
    expect(testReport({ DEVPIT_REPORT_DIR: '' })).toEqual({ reporters: ['default'] })
  })
})
