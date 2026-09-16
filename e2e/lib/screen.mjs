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

/** Saves a PNG of the window, and answers its path and how much of it is drawn on. */
export async function shoot(window, name) {
  mkdirSync(SHOTS, { recursive: true })
  const png = Buffer.from(await window.takeScreenshot(), 'base64')
  const path = join(SHOTS, `${name}.png`)
  writeFileSync(path, png)
  return { path, ink: inkOf(png) }
}

/**
 * The fraction of a PNG's pixels that are not its background colour.
 *
 * Read from the pixels the window really drew, not from the DOM: boxes can be
 * laid out and still be covered, transparent or off screen, and a sum of their
 * areas called a blank window drawn on. The background is the most common
 * colour; "not it" is a difference a person could see. Coarse on purpose — a
 * pixel baseline fails on a font rendered differently and then gets deleted.
 */
export function inkOf(png) {
  const { width, height, channels, pixels } = decodePng(png)
  const counts = new Map()
  const at = (i) => (pixels[i] << 16) | (pixels[i + 1] << 8) | pixels[i + 2]
  for (let i = 0; i < pixels.length; i += channels * 7) {
    const colour = at(i)
    counts.set(colour, (counts.get(colour) ?? 0) + 1)
  }
  const background = [...counts].sort((a, b) => b[1] - a[1])[0][0]
  const [br, bg, bb] = [background >> 16, (background >> 8) & 255, background & 255]
  let drawn = 0
  for (let i = 0; i < pixels.length; i += channels) {
    const far = Math.abs(pixels[i] - br) + Math.abs(pixels[i + 1] - bg) + Math.abs(pixels[i + 2] - bb)
    if (far > 24) drawn += 1
  }
  return drawn / (width * height)
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
