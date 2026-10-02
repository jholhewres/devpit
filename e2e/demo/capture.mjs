/*
 * Pictures and clips of the virtual screen, taken at the X level.
 *
 * Not `takeScreenshot`: this driver gives no frame (see `lib/screen.mjs`),
 * and the island and the browser page are native windows a WebDriver
 * screenshot would leave out anyway. ffmpeg reads Xvfb's framebuffer, which
 * holds exactly what a person would see.
 */

import { execFileSync, spawn } from 'node:child_process'
import { rmSync } from 'node:fs'
import { setTimeout as wait } from 'node:timers/promises'

/** One frame of `area` (physical pixels) as a PNG. */
export function still(display, area, path) {
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', ...grab(display, area), '-frames:v', '1', path])
}

/**
 * Every recorder still running. One left behind keeps a few cores busy for
 * as long as the machine is up, and the app it shares them with starts to time
 * out — which is how a failed clip made every run after it fail too.
 */
const filming = new Set()

export function stopFilming() {
  for (const ffmpeg of filming) ffmpeg.kill('SIGKILL')
}

/** The longest a clip plays, in seconds. */
const LONGEST = 11.5

/** Starts filming `area`; the answer's `stop()` writes an MP4 and a GIF. */
export function film(display, area, { mp4, gif, width = 1600, gifWidth = 960 }) {
  const raw = `${mp4}.raw.mkv`
  const ffmpeg = spawn(
    'ffmpeg',
    ['-y', '-loglevel', 'error', '-framerate', '30', ...grab(display, area), '-c:v', 'libx264', '-preset', 'ultrafast', '-qp', '0', raw],
    { stdio: ['pipe', 'ignore', 'inherit'] },
  )
  const began = Date.now()
  filming.add(ffmpeg)
  ffmpeg.on('exit', () => filming.delete(ffmpeg))
  return {
    /** Ends it and keeps nothing: the scene it was filming failed. */
    abort() {
      ffmpeg.kill('SIGKILL')
      rmSync(raw, { force: true })
    },
    async stop() {
      // At least five seconds of clip, whatever the scene took: the recorder
      // starts a moment after it is asked to, so there is a margin.
      const short = 6500 - (Date.now() - began)
      if (short > 0) await wait(short)
      ffmpeg.stdin.end('q')
      await new Promise((done) => ffmpeg.on('exit', done))
      // A scene that took longer than a clip should is played faster, not cut:
      // what matters is that the whole of it is seen.
      const took = (Date.now() - began) / 1000
      const speed = took > LONGEST ? took / LONGEST : 1
      const scale = `setpts=PTS/${speed.toFixed(3)},scale=${width}:-2:flags=lanczos`
      execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', raw, '-vf', scale, '-c:v', 'libx264', '-crf', '20', '-preset', 'slow', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', mp4])
      const palette = `setpts=PTS/${speed.toFixed(3)},fps=15,scale=${gifWidth}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`
      execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', raw, '-filter_complex', palette, gif])
      rmSync(raw, { force: true })
      return Math.min(took, LONGEST)
    },
  }
}

function grab(display, { x, y, width, height }) {
  return ['-f', 'x11grab', '-draw_mouse', '0', '-video_size', `${width}x${height}`, '-i', `${display}+${x},${y}`]
}

/** The brand mark at `size`, and the island's moods as a looping GIF. */
export function thumbnails(mark, moods, { png, gif, size = 240 }) {
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', mark, '-vf', `scale=${size}:${size}:flags=lanczos`, png])
  // Each mood held for a second, on the mark's own orange.
  const inputs = moods.flatMap((one) => ['-loop', '1', '-t', '1', '-i', one])
  const chain = moods.map((_, at) => `[${at}:v]scale=${size - 40}:${size - 40}:flags=lanczos,pad=${size}:${size}:20:20:color=0xE8622C,setsar=1[m${at}]`).join(';')
  const joined = `${chain};${moods.map((_, at) => `[m${at}]`).join('')}concat=n=${moods.length}:v=1:a=0,fps=10,split[a][b];[a]palettegen=reserve_transparent=0[p];[b][p]paletteuse`
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', ...inputs, '-filter_complex', joined, '-loop', '0', gif])
}
