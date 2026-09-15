import type { SpendDay, TokenCounts } from '../gen/bindings'

/*
 * How the usage screen says its numbers.
 */

/** A count as the contract carries it: a number the backend could not make one of is none. */
export const n = (value: number | null): number => value ?? 0

/** Dollars to the cent, and a cost too small for a cent said as such. */
export function dollars(value: number | null): string {
  const usd = n(value)
  if (usd > 0 && usd < 0.01) return '<$0.01'
  return `$${usd.toFixed(2)}`
}

/** Every token of every kind. */
export const allTokens = (counts: TokenCounts): number =>
  n(counts.input) + n(counts.output) + n(counts.cacheRead) + n(counts.cacheWrite)

/** A token count a person can read at a glance: 950, 12.4K, 3.1M. */
export function tokens(value: number | null): string {
  const count = n(value)
  if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`
  return String(Math.round(count))
}

/** When a quota window resets, from now: "resets in 2h 14m". */
export function resetIn(resetsAt: number | null, nowMs: number): string | null {
  if (resetsAt === null) return null
  const minutes = Math.max(0, Math.round((resetsAt * 1000 - nowMs) / 60_000))
  if (minutes === 0) return 'resets now'
  const days = Math.floor(minutes / 1_440)
  const hours = Math.floor((minutes % 1_440) / 60)
  const rest = minutes % 60
  if (days > 0) return `resets in ${days}d ${hours}h`
  if (hours > 0) return `resets in ${hours}h ${rest}m`
  return `resets in ${rest}m`
}

/** How long ago something happened, from now. */
export function ago(atSeconds: number | null, nowMs: number): string {
  const minutes = Math.max(0, Math.round((nowMs - n(atSeconds) * 1000) / 60_000))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  if (minutes < 1_440) return `${Math.floor(minutes / 60)}h ago`
  return `${Math.floor(minutes / 1_440)}d ago`
}

export type Metric = 'cost' | 'tokens'

/** A day's value in the chart's metric. */
export function valueOf(day: SpendDay, metric: Metric): number {
  return metric === 'cost' ? n(day.costUsd) : allTokens(day.tokens)
}

/** Each day's share of the tallest day, 0 to 1; all zero when nothing happened. */
export function heights(daily: readonly SpendDay[], metric: Metric): number[] {
  const tallest = Math.max(0, ...daily.map((day) => valueOf(day, metric)))
  return daily.map((day) => (tallest > 0 ? valueOf(day, metric) / tallest : 0))
}
