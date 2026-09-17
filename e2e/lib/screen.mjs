/*
 * What a screen has to be for a screenshot of it to mean anything.
 *
 * A PNG proves nothing on its own — a blank window photographs beautifully.
 * Every shot here is taken next to assertions that would fail on the blank
 * one: the landmarks exist by role or label, the container has a size, enough
 * of the picture's pixels are not background, nothing overflowed sideways, and
 * the console said nothing.
 */

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { inflateSync } from 'node:zlib'

export const SHOTS = join(process.env.E2E_ROOT ?? '.', 'target/e2e-shots')

/** Starts counting what the page complains about. Call once per window. */
export async function watchTheConsole(window) {
  await window.executeScript(function () {
    window.__e2e = { errors: [], policy: [] }
    const said = console.error.bind(console)
    console.error = function (...args) {
      window.__e2e.errors.push(args.map(String).join(' '))
      said(...args)
    }
    document.addEventListener('securitypolicyviolation', function (event) {
      window.__e2e.policy.push(event.violatedDirective + ' ' + event.blockedURI)
    })
  })
}

export async function complaints(window) {
  return window.executeScript(function () {
    return window.__e2e ?? { errors: [], policy: [] }
  })
}

/**
 * Where an element is in the screenshot, in the picture's own pixels.
 *
 * Null when nothing matches or the match has no size — a caller that measured
 * ink over `null` would be measuring the whole window again without knowing.
 */
export async function boxOf(window, selector) {
  return window.executeScript(function (selector) {
    const node = document.querySelector(selector)
    if (!node) return null
    const box = node.getBoundingClientRect()
    if (box.width < 1 || box.height < 1) return null
    const ratio = window.devicePixelRatio || 1
    return {
      x: Math.round(box.left * ratio),
      y: Math.round(box.top * ratio),
      width: Math.round(box.width * ratio),
      height: Math.round(box.height * ratio),
    }
  }, selector)
}

/** Saves a PNG of the window, and answers its path and how much of it is drawn
 *  on — of the whole picture, or of `crop` when one is given. */
/**
 * How long a screenshot is given before the suite gives up on it.
 *
 * Bounded because it is the one call here that can wait for ever: the driver
 * asks the compositor for a frame, and a window that never gives it one leaves
 * `takeScreenshot` with nothing to return and no error to raise. Unbounded, it
 * took the whole suite with it — the runner printed `TAP version 13` and then
 * nothing, and an hour later the job was cancelled with no line saying which
 * screen it died on.
 *
 * Measured on 17/09/2026: this driver gives no frame for **any** window, empty
 * or drawn, on this machine and on the CI runner both. Nothing the app draws
 * is involved — the same window answers every DOM question correctly while it
 * does it. So a missing frame is not a failed screen, and the assertions that
 * do not need a picture are what the suite still holds.
 */
const LONGEST_SHOT_MS = 10000

export async function shoot(window, name, crop = null) {
  mkdirSync(SHOTS, { recursive: true })
  const taken = await Promise.race([
    window.takeScreenshot(),
    new Promise((resolve) => setTimeout(() => resolve(null), LONGEST_SHOT_MS)),
  ]).catch(() => null)

  // `ink: null` is "nobody looked", and it is not the same as zero. A caller
  // that read it as zero would fail every screen on a machine whose driver
  // cannot photograph a window — and a caller that read it as "fine" would
  // pass a blank one. Both are wrong; saying nothing is the truth.
  if (taken === null) {
    return { path: null, ink: null }
  }

  const png = Buffer.from(taken, 'base64')
  const path = join(SHOTS, `${name}.png`)
  writeFileSync(path, png)
  return { path, ink: inkOf(png, crop) }
}

/**
 * The fraction of a PNG's pixels that are not its background colour.
 *
 * Read from the pixels the window really drew, not from the DOM: boxes can be
 * laid out and still be covered, transparent or off screen, and a sum of their
 * areas called a blank window drawn on. The background is the most common
 * colour; "not it" is a difference a person could see. Coarse on purpose — a
 * pixel baseline fails on a font rendered differently and then gets deleted.
 *
 * `crop` narrows it to one rectangle, and that is not a refinement — it is the
 * point. Over the whole window the sidebar and the title bar alone are already
 * past any threshold worth setting, so a board panel painted with its own
 * background photographed at 4.9% and passed. What is asserted has to be the
 * screen being tested, not the chrome around it.
 */
export function inkOf(png, crop = null) {
  const { width, height, channels, pixels } = decodePng(png)
  const area = clamped(crop, width, height)
  const rows = []
  for (let y = area.y; y < area.y + area.height; y += 1) {
    rows.push(y * width * channels)
  }
  const from = area.x * channels
  const to = (area.x + area.width) * channels

  const counts = new Map()
  const at = (i) => (pixels[i] << 16) | (pixels[i + 1] << 8) | pixels[i + 2]
  for (const row of rows) {
    for (let x = from; x < to; x += channels * 7) {
      const colour = at(row + x)
      counts.set(colour, (counts.get(colour) ?? 0) + 1)
    }
  }
  const background = [...counts].sort((a, b) => b[1] - a[1])[0][0]
  const [br, bg, bb] = [background >> 16, (background >> 8) & 255, background & 255]

  let drawn = 0
  for (const row of rows) {
    for (let x = from; x < to; x += channels) {
      const i = row + x
      const far =
        Math.abs(pixels[i] - br) + Math.abs(pixels[i + 1] - bg) + Math.abs(pixels[i + 2] - bb)
      if (far > 24) drawn += 1
    }
  }
  return drawn / (area.width * area.height)
}

/** A crop that is inside the picture, or the whole picture when there is none.
 *  A rectangle reaching past the edge would read another row's pixels and
 *  count them as this one's. */
function clamped(crop, width, height) {
  if (!crop) return { x: 0, y: 0, width, height }
  const x = Math.max(0, Math.min(crop.x, width - 1))
  const y = Math.max(0, Math.min(crop.y, height - 1))
  return {
    x,
    y,
    width: Math.max(1, Math.min(crop.width, width - x)),
    height: Math.max(1, Math.min(crop.height, height - y)),
  }
}

/** An 8-bit RGB or RGBA PNG, unfiltered into raw pixels. */
export function decodePng(png) {
  if (png.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG')
  let offset = 8
  let width = 0
  let height = 0
  let channels = 0
  const data = []
  while (offset < png.length) {
    const length = png.readUInt32BE(offset)
    const type = png.toString('latin1', offset + 4, offset + 8)
    const body = png.subarray(offset + 8, offset + 8 + length)
    if (type === 'IHDR') {
      width = body.readUInt32BE(0)
      height = body.readUInt32BE(4)
      const [depth, colour, , , interlace] = body.subarray(8, 13)
      if (depth !== 8 || interlace !== 0 || (colour !== 2 && colour !== 6)) {
        throw new Error(`unsupported PNG: depth ${depth}, colour ${colour}, interlace ${interlace}`)
      }
      channels = colour === 6 ? 4 : 3
    }
    if (type === 'IDAT') data.push(body)
    if (type === 'IEND') break
    offset += 12 + length
  }
  const raw = inflateSync(Buffer.concat(data))
  const stride = width * channels
  const pixels = Buffer.alloc(stride * height)
  for (let y = 0; y < height; y += 1) {
    const filter = raw[y * (stride + 1)]
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1))
    const out = y * stride
    for (let x = 0; x < stride; x += 1) {
      const left = x >= channels ? pixels[out + x - channels] : 0
      const up = y > 0 ? pixels[out + x - stride] : 0
      const corner = y > 0 && x >= channels ? pixels[out + x - stride - channels] : 0
      let value = line[x]
      if (filter === 1) value += left
      else if (filter === 2) value += up
      else if (filter === 3) value += (left + up) >> 1
      else if (filter === 4) {
        const guess = left + up - corner
        const [pa, pb, pc] = [Math.abs(guess - left), Math.abs(guess - up), Math.abs(guess - corner)]
        value += pa <= pb && pa <= pc ? left : pb <= pc ? up : corner
      }
      pixels[out + x] = value & 255
    }
  }
  return { width, height, channels, pixels }
}

/** Nothing sticks out sideways: a window that scrolls horizontally is broken. */
export async function overflowsSideways(window) {
  return window.executeScript(function () {
    return document.documentElement.scrollWidth > document.documentElement.clientWidth + 1
  })
}

/** The shell's own container, and its size. Null when there is no shell: the
 *  body always has a size, so measuring it instead proved nothing. */
export async function shellBox(window) {
  return window.executeScript(function () {
    const app = document.querySelector('.app')
    if (!app) return null
    const box = app.getBoundingClientRect()
    return { width: box.width, height: box.height }
  })
}

/** Whether an element matching this is on screen: in the page, not hidden,
 *  with a size. A landmark in a hidden tab is not a landmark on this screen. */
export async function landmark(window, selector) {
  return window.executeScript(function (selector) {
    return Array.prototype.slice.call(document.querySelectorAll(selector)).some(function (node) {
      const box = node.getBoundingClientRect()
      return box.width > 0 && box.height > 0 && node.closest('[hidden]') === null
    })
  }, selector)
}

/** Gives the window a moment to settle after something was asked of it. */
export async function settle(ms = 400) {
  await wait(ms)
}
