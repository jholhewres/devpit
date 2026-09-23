import type { Project } from '../gen/bindings'

/*
 * The project rail's rules, apart from its drawing.
 *
 * Order is the person's, not the clock's: the project list comes back sorted
 * by when each was last opened, so drawing it as it comes would move every
 * icon each time one is clicked — and a rail is read by where things are.
 * A project nobody has placed yet goes at the end.
 */

const ORDER_KEY = 'devpit.rail.order'

export function savedOrder(): readonly string[] {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(ORDER_KEY) ?? '[]')
    return Array.isArray(raw) ? raw.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}

export function saveOrder(order: readonly string[]): void {
  try {
    localStorage.setItem(ORDER_KEY, JSON.stringify(order))
  } catch {
    /* The order still applies for as long as the window is open. */
  }
}

/** The projects in the person's order, new ones after, in the order they
 *  first appeared. */
export function ordered(projects: readonly Project[], order: readonly string[]): readonly Project[] {
  const at = new Map(order.map((id, index) => [id, index]))
  const placed = projects.filter((project) => at.has(project.id))
  placed.sort((a, b) => at.get(a.id)! - at.get(b.id)!)
  const loose = projects
    .filter((project) => !at.has(project.id))
    .sort((a, b) => a.name.localeCompare(b.name))
  return [...placed, ...loose]
}

/** `id` put just before or just after `to` — which side is the pointer's:
 *  the upper half of a row is before it, the lower half after. */
export function placed(order: readonly string[], id: string, to: string, after: boolean): readonly string[] {
  if (id === to) return order
  const without = order.filter((one) => one !== id)
  const index = without.indexOf(to)
  if (index < 0) return order
  const at = after ? index + 1 : index
  return [...without.slice(0, at), id, ...without.slice(at)]
}

/** `id` moved to where `to` is. */
export function moved(order: readonly string[], id: string, to: string): readonly string[] {
  if (id === to) return order
  const without = order.filter((one) => one !== id)
  const index = without.indexOf(to)
  if (index < 0) return order
  const from = order.indexOf(id)
  const target = from >= 0 && from < order.indexOf(to) ? index + 1 : index
  return [...without.slice(0, target), id, ...without.slice(target)]
}

/** One or two letters for a project: the first of two words, or the first two
 *  of one. `devpit-app` is DA, `lupaphone` is LU. */
export function initials(name: string): string {
  const words = name.split(/[\s\-_.]+/).filter(Boolean)
  const letters =
    words.length >= 2 ? `${words[0]![0]}${words[1]![0]}` : (words[0] ?? name).slice(0, 2)
  return letters.toUpperCase()
}

/** A hue of the project's own, stable across runs. Projects share their
 *  workspace's accent, so the rail cannot tell them apart by it. */
export function hueOf(id: string): number {
  let hash = 0
  for (const letter of id) hash = (hash * 31 + letter.charCodeAt(0)) >>> 0
  return hash % 360
}

export interface Section {
  /** The group's name, or null for the projects in none. */
  readonly group: string | null
  readonly projects: readonly Project[]
}

/** The ordered projects, cut into their groups: the ungrouped ones first,
 *  then the groups in the order the person put them, and any group not
 *  placed yet where its first project stands. */
export function sections(list: readonly Project[], groupOrder: readonly string[] = []): readonly Section[] {
  const loose = list.filter((project) => !project.group)
  const groups: { group: string; projects: Project[] }[] = []
  for (const project of list) {
    if (!project.group) continue
    const found = groups.find((one) => one.group === project.group)
    if (found) found.projects.push(project)
    else groups.push({ group: project.group, projects: [project] })
  }
  const at = (group: string): number => {
    const placed = groupOrder.indexOf(group)
    return placed < 0 ? groupOrder.length + groups.findIndex((one) => one.group === group) : placed
  }
  groups.sort((a, b) => at(a.group) - at(b.group))
  return [...(loose.length > 0 ? [{ group: null, projects: loose }] : []), ...groups]
}

/* The order of the groups, remembered per window like the order of projects. */
const GROUPS_KEY = 'devpit.rail.groups'

export function savedGroups(): readonly string[] {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(GROUPS_KEY) ?? '[]')
    return Array.isArray(raw) ? raw.filter((one): one is string => typeof one === 'string') : []
  } catch {
    return []
  }
}

export function saveGroups(order: readonly string[]): void {
  try {
    localStorage.setItem(GROUPS_KEY, JSON.stringify(order))
  } catch {
    /* The order still applies for as long as the window is open. */
  }
}

/* Which groups are folded, remembered per window. */
const SHUT_KEY = 'devpit.rail.shut'

export function savedShut(): ReadonlySet<string> {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(SHUT_KEY) ?? '[]')
    return new Set(Array.isArray(raw) ? raw.filter((one): one is string => typeof one === 'string') : [])
  } catch {
    return new Set()
  }
}

export function saveShut(shut: ReadonlySet<string>): void {
  try {
    localStorage.setItem(SHUT_KEY, JSON.stringify([...shut]))
  } catch {
    /* Folding still applies for as long as the window is open. */
  }
}

/** What a group shows: all of it when open, nothing when folded. */
export function shown(section: Section, folded: boolean): readonly Project[] {
  return folded ? [] : section.projects
}
