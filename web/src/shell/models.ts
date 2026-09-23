import type { EnvVar } from '../gen/bindings'

/*
 * What a model is called on screen.
 *
 * The CLI is passed aliases — `opus`, `sonnet` — because an alias follows the
 * provider and a dated id does not. A person reads versions, so the alias is
 * shown as the model it resolves to on Anthropic's own API, and the alias
 * itself stays beside it for anyone checking what is actually sent.
 */

/* What each alias resolves to on the Anthropic API, per the CLI's model
   configuration docs. The one table here that ages: update it with a release. */
const ALIASES: Readonly<Record<string, string>> = {
  opus: 'Opus 5.5',
  sonnet: 'Sonnet 5',
  haiku: 'Haiku 4.5',
  fable: 'Fable 5.1',
  best: 'Best available',
  opusplan: 'Opus plans, Sonnet builds',
}

/* The variables a profile uses to point an alias somewhere else — `glm` sets
   all of them, and "Opus 5.5" on a row that runs GLM would be a lie. */
const MOVED: Readonly<Record<string, string>> = {
  default: 'ANTHROPIC_MODEL',
  opus: 'ANTHROPIC_DEFAULT_OPUS_MODEL',
  sonnet: 'ANTHROPIC_DEFAULT_SONNET_MODEL',
  haiku: 'ANTHROPIC_DEFAULT_HAIKU_MODEL',
  fable: 'ANTHROPIC_DEFAULT_FABLE_MODEL',
}

const WIDE = '[1m]'

/* `claude-opus-5-5`, `claude-haiku-4-5-20251001` → family and version. */
const DATED = /^claude-([a-z]+)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/

/* What the profile's environment makes an alias run, if it moves it at all. */
export function movedTo(model: string, env: readonly EnvVar[] = []): string | null {
  const name = MOVED[model.replace(WIDE, '')]
  const value = env.find((one) => one.name === name)?.value.trim()
  return value ? value : null
}

/* One model, as a person reads it. `env` is the profile's, so an alias a
   gateway redirects is named by where it goes. */
export function modelName(model: string, env: readonly EnvVar[] = []): string {
  const moved = movedTo(model, env)
  if (moved) return modelName(moved)
  if (model === 'default') return "The account's default"
  const wide = model.endsWith(WIDE)
  const bare = wide ? model.slice(0, -WIDE.length) : model
  const dated = DATED.exec(bare)
  const named =
    ALIASES[bare] ??
    (dated
      ? `${dated[1][0].toUpperCase()}${dated[1].slice(1)} ${dated[2]}${dated[3] ? `.${dated[3]}` : ''}`
      : bare)
  return wide ? `${named} · 1M` : named
}

/* The raw id under the name: what the CLI is actually passed, and where a
   profile sends it when it moves the alias. */
export function modelHint(model: string, env: readonly EnvVar[] = []): string {
  const moved = movedTo(model, env)
  return moved ? `${model} → ${moved}` : model
}

/* A list typed as one line or one per line, in the order it was typed. */
export function modelsOf(text: string): string[] {
  const seen = new Set<string>()
  return text
    .split(/[\s,]+/)
    .filter((one) => one.length > 0 && !seen.has(one) && (seen.add(one), true))
}

export const modelsText = (models: readonly string[]): string => models.join(', ')
