import type { Project, ProjectProposal } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * A proposal made: the folder added, its name and group given, its link to
 * the orchestrator set — with the commands the rest of the window uses, so a
 * proposal can do nothing the person could not do by hand.
 */

/** Said on the window when an orchestrator's links change, so its Boards panel reads them again. */
export const LINKS_CHANGED = 'devpit:links-changed'

/** What a proposal would do, in words. */
export function proposalWords(proposal: ProjectProposal, known: readonly Project[]): string {
  const project = known.find((one) => one.id === proposal.projectId)
  const changes: string[] = []
  if (!project) changes.push(`Add ${proposal.path} as ${proposal.name}`)
  else if (proposal.name !== project.name) changes.push(`Rename ${project.name} to ${proposal.name}`)
  if (proposal.group && proposal.group !== project?.group) changes.push(`${changes.length ? 'put it' : `Put ${project?.name ?? proposal.name}`} in ${proposal.group}`)
  if (proposal.link === true) changes.push(changes.length ? 'link it here' : `Link ${project?.name ?? proposal.name} here`)
  if (proposal.link === false) changes.push(changes.length ? 'unlink it' : `Unlink ${project?.name ?? proposal.name}`)
  return changes.join(', ')
}

/** Makes the proposal; an error in words, or null when it is done. */
export async function applied(proposal: ProjectProposal, hereId: string, known: readonly Project[]): Promise<string | null> {
  let project = known.find((one) => one.id === proposal.projectId)
  if (!project) {
    const added = await ask(() => commands.projectAdd(proposal.path))
    if (!added.data) return added.error ?? 'the folder could not be added'
    project = added.data
  }
  const name = proposal.name.trim() || project.name
  const group = proposal.group ?? project.group
  if (name !== project.name || group !== project.group) {
    const edited = await ask(() => commands.projectEdit(project.id, name, group, project.icon, project.color))
    if (edited.error) return edited.error
  }
  if (proposal.link !== null) {
    const links = await ask(() => commands.orchestratorLinks(hereId))
    if (links.error) return links.error
    const now = new Set(links.data ?? [])
    if (proposal.link) now.add(project.id)
    else now.delete(project.id)
    const linked = await ask(() => commands.orchestratorLink(hereId, [...now]))
    if (linked.error) return linked.error
    window.dispatchEvent(new CustomEvent(LINKS_CHANGED))
  }
  return null
}
