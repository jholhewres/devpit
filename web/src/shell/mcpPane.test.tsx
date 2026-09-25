import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Installation, Project, Server } from '../gen/bindings'
import { McpPane } from './McpPane'

afterEach(cleanup)

const listed = vi.fn()

let catalogue: Server[] = []
let installs: Installation[] = []

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    cliInstallations: () => [...installs],
    mcpList: (projectId: string | null, directory: string | null) => {
      listed(projectId, directory)
      return {
        servers: [...catalogue],
        sources: ['/home/me/.claude-2/.claude.json'],
        problem: catalogue.length === 0 ? 'no MCP server is configured for the agent CLI' : null,
        manageWith: 'claude mcp',
        directory: '/home/me/.claude-2',
      }
    },
  },
}))

const project = { id: 'p', name: 'demos' } as unknown as Project
vi.mock('./useShell', () => ({ useShell: () => ({ project, close: vi.fn() }) }))

beforeEach(() => {
  listed.mockClear()
  installs = [{ directory: '/home/me/.claude-2', profiles: ['claude2'], default: true }]
  catalogue = [
    { name: 'reports', scope: 'project', reachedBy: 'npx reports-mcp' },
    { name: 'anchored', scope: 'user', reachedBy: 'https://anchored.example/mcp' },
  ]
})

/*
 * The panel exists because a server that failed to connect looks exactly like
 * a tool the agent never had — which makes naming the installation the job.
 */

describe('the MCP panel', () => {
  it('separates what travels with the repository from what follows the person', async () => {
    render(<McpPane />)
    expect(await screen.findByText('This project')).toBeTruthy()
    expect(screen.getByText('Every project')).toBeTruthy()
  })

  it('shows how each server is reached', async () => {
    render(<McpPane />)
    expect(await screen.findByText('npx reports-mcp')).toBeTruthy()
    expect(screen.getByText('https://anchored.example/mcp')).toBeTruthy()
  })

  it('puts the scope beside the name, apart from how the server is reached', async () => {
    const { container } = render(<McpPane />)
    await screen.findByText('reports')
    const top = screen.getByText('reports').closest('.cap__top')!
    expect(top.textContent).toBe('reportsproject')
    expect(top.contains(screen.getByText('npx reports-mcp'))).toBe(false)
    expect(container.querySelector('.list__in')).toBeTruthy()
  })

  it('names the directory it read, because it is not always ~/.claude', async () => {
    render(<McpPane />)
    expect(await screen.findByText('/home/me/.claude-2')).toBeTruthy()
  })

  it('re-reads on demand, because `claude mcp add` happens in a terminal', async () => {
    render(<McpPane />)
    await screen.findByText('reports')
    catalogue = [...catalogue, { name: 'fresh', scope: 'user', reachedBy: 'node fresh' }]
    fireEvent.click(screen.getByLabelText('Refresh MCPs'))
    expect(await screen.findByText('fresh')).toBeTruthy()
    await waitFor(() => expect(listed).toHaveBeenCalledTimes(2))
  })

  it('says nothing is configured rather than looking broken', async () => {
    catalogue = []
    render(<McpPane />)
    expect(await screen.findByText('no MCP server is configured for the agent CLI')).toBeTruthy()
    // An empty catalogue is the CLI's state, not a failure of this window.
    expect(screen.getByText(/Nothing is wrong with devpit/)).toBeTruthy()
  })

  it('draws no group for a scope with nothing in it', async () => {
    catalogue = [{ name: 'anchored', scope: 'user', reachedBy: 'url' }]
    render(<McpPane />)
    await screen.findByText('anchored')
    expect(screen.queryByText('This project')).toBeNull()
  })

  it('reads the installation that was picked, for this project', async () => {
    installs = [
      { directory: '/home/me/.claude', profiles: ['claude'], default: true },
      { directory: '/home/me/.claude-glm', profiles: ['glm'], default: false },
    ]
    render(<McpPane />)
    await waitFor(() => expect(listed).toHaveBeenCalledWith('p', null))
    fireEvent.click(await screen.findByRole('radio', { name: 'glm' }))
    await waitFor(() => expect(listed).toHaveBeenCalledWith('p', '/home/me/.claude-glm'))
  })
})
