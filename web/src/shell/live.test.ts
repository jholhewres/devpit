import { describe, expect, it } from 'vitest'

import { ask } from './live'

/* Outside Tauri every call is refused rather than left pending — a screen
   waiting forever is indistinguishable from one that is broken. */
describe('asking the backend', () => {
  it('refuses outside the app instead of hanging', async () => {
    const asked = await ask(async () => 'never')
    expect(asked.loading).toBe(false)
    expect(asked.error).toBeTruthy()
    expect(asked.data).toBeNull()
  })
})

describe('unwrapping the contract', () => {
  it('takes the data out of an ok answer', async () => {
    /* inTauri() is false in tests, so the unwrap is exercised directly. */
    const { unwrap } = await import('./live')
    expect(unwrap({ status: 'ok', data: 7 })).toEqual({ data: 7, error: null, loading: false })
  })

  it('takes the message out of an error answer', async () => {
    const { unwrap } = await import('./live')
    expect(unwrap({ status: 'error', error: { message: 'no such project' } }).error).toBe(
      'no such project',
    )
  })

  it('keeps the error code, so a conflict can be told from a failure', async () => {
    const { unwrap } = await import('./live')
    expect(unwrap({ status: 'error', error: { message: 'changed on disk', code: 'conflict' } }).code).toBe('conflict')
    expect(unwrap({ status: 'error', error: {} }).code).toBeNull()
  })

  it('names the failure even when the error carries no message', async () => {
    const { unwrap } = await import('./live')
    expect(unwrap({ status: 'error', error: {} }).error).toBeTruthy()
  })

  it('passes through a command that answers plainly', async () => {
    const { unwrap } = await import('./live')
    expect(unwrap(42).data).toBe(42)
  })
})
