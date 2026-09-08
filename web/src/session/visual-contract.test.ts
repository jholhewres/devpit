import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const css = readFileSync(resolve(process.cwd(), 'src/shell/shell.css'), 'utf8')

describe('terminal visual contract', () => {
  it('keeps surfaces as an overlay over the mounted terminal stage', () => {
    expect(css).toMatch(
      /\.surface\s*\{[\s\S]*?position:\s*absolute;[\s\S]*?inset:\s*0;[\s\S]*?z-index:\s*2;/
    )
    expect(css).toMatch(/\.terminal-stage--live\s*\{[\s\S]*?display:\s*flex;/)
  })

  it('maps horizontal and vertical nodes to the expected flex axes', () => {
    expect(css).toMatch(
      /\.pty-split\[data-direction='horizontal'\]\s*\{\s*flex-direction:\s*row;/
    )
    expect(css).toMatch(
      /\.pty-split\[data-direction='vertical'\]\s*\{\s*flex-direction:\s*column;/
    )
  })
})
