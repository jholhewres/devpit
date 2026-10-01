import { useEffect } from 'react'

import type { Project } from '../gen/bindings'
import { ask, commands } from './live'

/**
 * devpit's half of an orchestrator's brief, brought up to this build whenever
 * one comes in front — whichever way it came: the rail, the picker, the
 * Sessions panel, a delegation, or the window opening on it.
 *
 * It used to be the rail's click alone, so every other way in kept the brief
 * an older build had written. The app also rewrites every brief as it starts;
 * this is for the folder that changed under it since.
 */
export function useBriefRefresh(project: Pick<Project, 'id' | 'orchestrator'> | null): void {
  const front = project?.orchestrator ? project.id : null
  useEffect(() => {
    if (front) void ask(() => commands.orchestratorRefresh(front))
  }, [front])
}
