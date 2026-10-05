import type { SessionCost } from '../gen/bindings'

/* What a session has spent, in the words and the unit it is shown in. */

export type CostUnit = 'usd' | 'tokens'

const KEY = 'devpit:cost-unit'

export function unitNow(): CostUnit {
  try {
    return localStorage.getItem(KEY) === 'tokens' ? 'tokens' : 'usd'
  } catch {
    return 'usd'
  }
}

export function keepUnit(unit: CostUnit): void {
  try {
    localStorage.setItem(KEY, unit)
  } catch {
    /* Not kept: the next window starts in dollars. */
  }
}

/* A number that crossed as `null` was not one (NaN): read as nothing spent. */
const n = (value: number | null): number => value ?? 0

const dollars = (usd: number | null): string => (n(usd) < 0.01 && n(usd) > 0 ? '<$0.01' : `$${n(usd).toFixed(2)}`)

export function tokens(counted: number | null): string {
  const count = n(counted)
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
  if (count >= 1_000) return `${Math.round(count / 1_000)}k`
  return String(count)
}

const all = (cost: SessionCost): number =>
  n(cost.tokens.input) + n(cost.tokens.output) + n(cost.tokens.cacheRead) + n(cost.tokens.cacheWrite)

/* The tag: dollars or tokens, as the person chose. */
export function costWords(cost: SessionCost, unit: CostUnit): string {
  return unit === 'tokens' ? `${tokens(all(cost))} tok` : dollars(cost.costUsd)
}

/* The rest, for the tooltip. Dollars are list prices: on a plan they are what
   the tokens would cost through the API, not a bill. */
export function costDetail(cost: SessionCost): string {
  const lines = [
    `API-equivalent cost: ${dollars(cost.costUsd)}`,
    `Last turn: ${dollars(cost.lastTurnUsd)}`,
  ]
  if (n(cost.sinceSeenUsd) < n(cost.costUsd)) lines.push(`Since it was opened here: ${dollars(cost.sinceSeenUsd)}`)
  const { input, output, cacheRead, cacheWrite } = cost.tokens
  lines.push(`Tokens: ${tokens(input)} in · ${tokens(output)} out · ${tokens(cacheRead)} cache read · ${tokens(cacheWrite)} cache write`)
  if (cost.model) lines.push(`Model: ${cost.model}`)
  if (n(cost.unpricedTokens) > 0) lines.push(`${tokens(cost.unpricedTokens)} tokens of a model with no known price are not in the dollars.`)
  lines.push('Click to show tokens or dollars.')
  return lines.join('\n')
}
