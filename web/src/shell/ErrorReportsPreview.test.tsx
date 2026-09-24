import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ErrorReportsPreview } from './ErrorReportsPreview'

afterEach(cleanup)

const body = '{\n  "reports": [{ "kind": "internal" }]\n}'
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: { errorsPreview: () => ({ waiting: 3, next: body }) },
}))

describe('the error report preview', () => {
  it('shows the exact body the next report would send', async () => {
    render(<ErrorReportsPreview />)
    fireEvent.click(screen.getByText('Show what would be sent'))
    expect(await screen.findByText(/"kind": "internal"/)).toBeTruthy()
    expect(screen.getByText(/3 kept/)).toBeTruthy()
  })
})
