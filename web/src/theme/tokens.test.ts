import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

// Read off disk, not imported: vitest stubs CSS modules, so `?raw` arrives
// empty and every assertion below would pass against nothing.
const css = readFileSync(resolve(process.cwd(), 'src/theme/tokens.css'), 'utf8')

/**
 * The elevation ramp, read from the stylesheet rather than restated here.
 *
 * These are the surfaces stacked from the window frame up to a hovered row,
 * and the whole design rests on them getting lighter in that order. Get one
 * out of sequence and nothing errors: hover paints a dent instead of a lift,
 * the sidebar sinks into the canvas, and it looks like a rendering bug rather
 * than a stylesheet that says so plainly.
 *
 * It reads the real file for the same reason the Rust tests call their rules
 * instead of copying them — a table of expected values typed in here would
 * pass whether or not the stylesheet still held them.
 */
function declaration(name: string): string {
  const found = new RegExp(`--${name}:\\s*([^;]+);`).exec(css)
  if (found === null) throw new Error(`--${name} is not declared in tokens.css`)
  return found[1].trim()
}

/**
 * Perceived lightness, 0 to 1.
 *
 * sRGB relative luminance, not the raw channel average: #0000ff and #ffff00
 * average the same and are nowhere near the same brightness, and a ramp
 * checked on the average would pass while looking wrong.
 */
function lightness(hex: string): number {
  const clean = hex.replace('#', '')
  const channel = (at: number): number => {
    const raw = Number.parseInt(clean.slice(at, at + 2), 16) / 255
    return raw <= 0.04045 ? raw / 12.92 : ((raw + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
}

describe('the elevation ramp', () => {
  /**
   * Frame, canvas, card, elevated. Each one sits on the one before it, so
   * each has to be lighter than the one before it.
   */
  it('gets lighter from the frame outward', () => {
    const ramp = ['sunken', 'ground', 'panel', 'raised', 'raised-hover']
    const levels = ramp.map((name) => ({ name, level: lightness(declaration(name)) }))

    for (let at = 1; at < levels.length; at += 1) {
      expect(
        levels[at].level,
        `--${levels[at].name} is not lighter than --${levels[at - 1].name}`
      ).toBeGreaterThan(levels[at - 1].level)
    }
  })

  /**
   * The sidebar is the surface always on screen, and it has to read as lifted
   * off the canvas. This is the one that was wrong: it sat seven points above
   * --ground and the window looked like one flat sheet.
   */
  it('lifts the sidebar clear of the canvas', () => {
    const canvas = lightness(declaration('ground'))
    const sidebar = lightness(declaration('sidebar'))

    expect(sidebar).toBeGreaterThan(canvas)
    // A separation, not a nudge. Below this the two surfaces read as one.
    expect(sidebar - canvas).toBeGreaterThan(0.01)
  })

  /**
   * Rows on the sidebar mix from the sidebar's own background, so hover and
   * active are ordered by how much ink goes in. Mixing from a shared token
   * is what broke when the sidebar lifted above it.
   */
  it('builds sidebar rows from the sidebar, not from a shared elevation', () => {
    const hover = declaration('sidebar-hover')
    const active = declaration('sidebar-active')

    for (const [name, value] of [
      ['sidebar-hover', hover],
      ['sidebar-active', active]
    ] as const) {
      expect(value, `--${name} should mix from var(--sidebar)`).toContain('var(--sidebar)')
    }

    const percentage = (mix: string): number => {
      const found = /(\d+)%/.exec(mix)
      if (found === null) throw new Error(`no percentage in ${mix}`)
      return Number(found[1])
    }
    expect(percentage(active)).toBeGreaterThan(percentage(hover))
  })
})
