import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ProjectSettings, Scoped } from '../gen/bindings'
import { ScopedSetting } from './ScopedSetting'

const machine: Scoped = { model: 'haiku', permissionMode: null, branchPattern: 'feat/{number}-{slug}' }
let own: Record<string, Scoped> = {}
const settings = (projectId: string | null): ProjectSettings => {
  const mine = projectId ? (own[projectId] ?? {}) : {}
  const repo: Scoped = projectId ? { model: null, permissionMode: 'plan', branchPattern: null } : {}
  return {
    own: mine,
    repo,
    machine,
    applied: { model: mine.model ?? repo.model ?? machine.model, permissionMode: mine.permissionMode ?? repo.permissionMode, branchPattern: mine.branchPattern ?? machine.branchPattern },
    repoProblem: null,
  }
}
const write = vi.fn(async (projectId: string | null, field: keyof Scoped, value: string | null) => {
  if (projectId) own = { ...own, [projectId]: { ...own[projectId], [field]: value } }
  return settings(projectId)
})
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    projectList: async () => ({ projects: [{ id: 'api', name: 'api', orchestrator: null }, { id: 'crew', name: 'Release crew', orchestrator: 'claude' }] }),
    projectSettingsRead: async (projectId: string | null) => settings(projectId),
    projectSettingsWrite: (projectId: string | null, field: keyof Scoped, value: string | null) => write(projectId, field, value),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  own = {}
})

describe('settings applied per project', () => {
  it("apply to all projects first, and each project's own value wins over its repo's and the machine's", async () => {
    render(<ScopedSetting field="model" />)
    const scope = (await screen.findByLabelText('Applying settings for (Default model)')) as HTMLSelectElement
    expect(scope.value).toBe('')
    expect(screen.queryByRole('option', { name: 'Release crew' })).toBeNull()
    await waitFor(() => expect((screen.getByLabelText('Default model') as HTMLInputElement).value).toBe('haiku'))

    fireEvent.change(scope, { target: { value: 'api' } })
    await waitFor(() => expect((screen.getByLabelText('Default model') as HTMLInputElement).placeholder).toBe('haiku from all projects'))
    fireEvent.blur(screen.getByLabelText('Default model'), { target: { value: 'opus' } })
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('api', 'model', 'opus'))
  })

  it("say where a project's permission mode comes from, and keep the project's own", async () => {
    render(<ScopedSetting field="permissionMode" />)
    fireEvent.change(await screen.findByLabelText('Applying settings for (Permission mode)'), { target: { value: 'api' } })
    expect(await screen.findByRole('option', { name: 'As Plan from .devpit/settings.json' })).toBeTruthy()
    fireEvent.change(screen.getByLabelText('Permission mode'), { target: { value: 'acceptEdits' } })
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('api', 'permissionMode', 'acceptEdits'))
  })

  it("leave the machine's branch name to Settings → Worktrees and take a project's own", async () => {
    render(<ScopedSetting field="branchPattern" />)
    await screen.findByLabelText('Applying settings for (Branch name for this project)')
    expect(screen.queryByLabelText('Branch name for this project')).toBeNull()
    fireEvent.change(screen.getByLabelText('Applying settings for (Branch name for this project)'), { target: { value: 'api' } })
    fireEvent.blur(await screen.findByLabelText('Branch name for this project'), { target: { value: 'fix/{slug}' } })
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('api', 'branchPattern', 'fix/{slug}'))
  })
})
