import { describe, expect, it } from 'vitest'

import { CONTENT_LEAST, dragged, fits, held, LEAST, MOST, stored, WIDE } from './sizing'

/* A window with room for anything, so a test about limits is about the limit
   it names and not about the window. */
const ROOMY = 4000

describe('a width held to what the window can give', () => {
  it('gives back what was asked for when there is room', () => {
    expect(held('sidebar', 300, ROOMY, WIDE.files)).toBe(300)
  })

  it('will not go narrower than a panel worth having', () => {
    expect(held('sidebar', 10, ROOMY, WIDE.files)).toBe(LEAST.sidebar)
    expect(held('files', 0, ROOMY, WIDE.sidebar)).toBe(LEAST.files)
  })

  it('will not go wider than a panel should be', () => {
    expect(held('sidebar', 9000, ROOMY, WIDE.files)).toBe(MOST.sidebar)
    expect(held('files', 9000, ROOMY, WIDE.sidebar)).toBe(MOST.files)
  })

  it('stops where the content would start disappearing', () => {
    // The grid would win this argument anyway. Stopping the drag is what
    // makes that visible instead of mysterious.
    const window = 900
    const wide = held('sidebar', MOST.sidebar, window, 340)
    expect(wide).toBe(window - 340 - CONTENT_LEAST)
    expect(wide).toBeLessThan(MOST.sidebar)
  })

  it('keeps the panel usable even on a window with no room at all', () => {
    // Something has to give, and it is not the panel's own floor: a panel
    // clamped to nothing is a panel that cannot be dragged back.
    expect(held('sidebar', 300, 400, 340)).toBe(LEAST.sidebar)
  })

  it('rounds, because a fraction of a pixel is not a width', () => {
    expect(held('sidebar', 300.6, ROOMY, WIDE.files)).toBe(301)
  })
})

describe('both of them, on a window that just got smaller', () => {
  it('leaves a roomy window alone', () => {
    expect(fits(WIDE, ROOMY)).toEqual(WIDE)
  })

  it('brings both in when the window cannot hold them', () => {
    const both = fits({ sidebar: 480, files: 640 }, 1000)
    expect(both.sidebar).toBeLessThanOrEqual(480)
    expect(both.files).toBeLessThanOrEqual(640)
    expect(both.sidebar + both.files + CONTENT_LEAST).toBeLessThanOrEqual(1000)
  })

  it('never squeezes either below its floor, however small the window', () => {
    const both = fits(WIDE, 200)
    expect(both.sidebar).toBe(LEAST.sidebar)
    expect(both.files).toBe(LEAST.files)
  })
})

describe('where a drag puts the edge', () => {
  it('widens the sidebar as the pointer goes right', () => {
    expect(dragged('sidebar', 100, 252, 160)).toBe(312)
  })

  it('narrows the sidebar as the pointer goes left', () => {
    expect(dragged('sidebar', 100, 252, 40)).toBe(192)
  })

  it('widens the files panel as the pointer goes left', () => {
    // The one asymmetry: they are opposite edges of the same window.
    expect(dragged('files', 900, 340, 840)).toBe(400)
  })

  it('is where it started when the pointer has not moved', () => {
    expect(dragged('sidebar', 100, 252, 100)).toBe(252)
  })
})

describe('what was on disk', () => {
  it('reads nothing as the width we ship', () => {
    expect(stored(null, null)).toEqual(WIDE)
  })

  it('holds a stored width to the same limits a drag has', () => {
    // A number edited by hand, or left by an older build with other limits,
    // must not be able to produce a panel nobody can drag back.
    expect(stored(5, 99999)).toEqual({ sidebar: LEAST.sidebar, files: MOST.files })
  })

  it('keeps a width somebody actually chose', () => {
    expect(stored(300, 420)).toEqual({ sidebar: 300, files: 420 })
  })
})
