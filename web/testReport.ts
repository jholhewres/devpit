import { join, resolve } from 'node:path'

export interface TestReport {
  reporters: string[]
  outputFile?: { json: string }
}

/**
 * Vitest's reporters for a run in `env`. A Devflow check sets
 * `DEVPIT_REPORT_DIR`; the default reporter prints no passing test's name,
 * so its JSON is what lets evidence find a vitest test named in a criterion.
 */
export function testReport(env: Record<string, string | undefined>): TestReport {
  const dir = env.DEVPIT_REPORT_DIR
  if (!dir) return { reporters: ['default'] }
  // pnpm runs the script in web/; a relative folder is the caller's.
  const from = env.INIT_CWD ?? process.cwd()
  return {
    reporters: ['default', 'json'],
    outputFile: { json: join(resolve(from, dir), 'vitest.json') }
  }
}
