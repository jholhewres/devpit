import type { PaneCost, Usage } from '../gen/bindings'

/*
 * What the terminals cost, said in words.
 *
 * The numbers arrive in kibibytes and percent; everything here is about how to
 * put them on a strip a few pixels tall without lying.
 */

/** Memory, at the scale a person reads. */
export function size(kb: number): string {
  if (kb < 1024) return `${Math.round(kb)} KB`
  const mb = kb / 1024
  if (mb < 1024) return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`
  return `${(mb / 1024).toFixed(2)} GB`
}

/*
 * CPU, from tenths of a percent.
 *
 * Tenths cross the wire rather than a float: an `f64` types as `number | null`
 * because a float can be NaN and JSON has no word for it, and a percentage
 * that might be null is a percentage every caller has to second-guess.
 *
 * Not clamped at 100: an agent running four tools at once is doing four cores
 * of work, and saying 100% would hide the one thing on this strip worth
 * looking at.
 */
export function cpu(tenths: number): string {
  if (tenths < 5) return '0%'
  return tenths < 100 ? `${(tenths / 10).toFixed(1)}%` : `${Math.round(tenths / 10)}%`
}

/** Whether the total is worth drawing at all. */
export const busy = (usage: Usage): boolean =>
  usage.panes.length > 0 && (usage.memoryKb > 0 || usage.cpuTenths > 0)

/*
 * What the number means, for the tooltip.
 *
 * A total that mixes proportional and resident memory is neither, so it says
 * which one it is. Resident overstates a process tree — 44% on the machine
 * this was measured on — and a number that might be half wrong has to arrive
 * labelled rather than quietly.
 */
export const counted = (usage: Usage): string =>
  usage.proportional
    ? 'Memory shared between processes is divided among them, so the total is what is really in use.'
    : 'Some processes could only be read as resident, so shared memory is counted more than once and this total is high.'

/** The panes worth a row, largest first. Zero-cost panes are a shell. */
export const notable = (panes: readonly PaneCost[]): readonly PaneCost[] =>
  panes.filter((pane) => pane.memoryKb > 0)
