import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { EnvironmentCopy } from './EnvironmentCopy'

afterEach(cleanup)

const copied: string[] = []
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: async (text: string) => {
    copied.push(text)
  },
}))
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { diagnosticsEnvironment: () => ({ text: 'claude: not found\n' }) },
}))

describe('copy environment info', () => {
  it('puts the report on the clipboard and says so', async () => {
    render(<EnvironmentCopy />)
    fireEvent.click(screen.getByText('Copy environment info'))
    await waitFor(() => expect(screen.getByText('Copied.')).toBeTruthy())
    expect(copied).toEqual(['claude: not found\n'])
  })
})
