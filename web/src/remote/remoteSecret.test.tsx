import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { RemoteProject } from '../gen/bindings'
import { RemoteSecret } from './RemoteSecret'

afterEach(cleanup)

const projects: RemoteProject[] = [
  { id: 'p1', name: 'api', group: null, terminals: [] },
  { id: 'p2', name: 'site', group: null, terminals: [] },
]

describe('a secret kept from the Remote', () => {
  it('goes to the machine from a password field, and leaves the page as it goes', () => {
    const send = vi.fn()
    render(<RemoteSecret projects={projects} project="p1" typing send={send} />)
    fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p2' } })
    fireEvent.change(screen.getByPlaceholderText('JEV_API_KEY'), { target: { value: ' jev_api_key ' } })
    const value = screen.getByLabelText('Value') as HTMLInputElement
    expect(value.type).toBe('password')
    fireEvent.change(value, { target: { value: 'sk-from-the-phone' } })
    fireEvent.click(screen.getByText('Keep it'))
    expect(send).toHaveBeenCalledWith({ t: 'secretSet', project: 'p2', name: 'JEV_API_KEY', value: 'sk-from-the-phone' })
    expect(value.value).toBe('')
  })

  it('is not offered to a device that may only watch', () => {
    render(<RemoteSecret projects={projects} project="p1" typing={false} send={vi.fn()} />)
    expect(screen.queryByLabelText('Value')).toBeNull()
    expect(screen.getByText(/may only watch/)).toBeTruthy()
  })
})
