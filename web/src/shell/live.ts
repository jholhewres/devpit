import { commands } from '../gen/bindings'
import { inTauri } from './window'

/* One place that knows a command can fail, so no caller draws an empty
   state when it should be showing an error. */
export interface Asked<T> {
  readonly data: T | null
  readonly error: string | null
  readonly loading: boolean
}

export const waiting = <T,>(): Asked<T> => ({ data: null, error: null, loading: true })

/* The contract answers `{ status: 'ok' | 'error' }` rather than throwing, so
   unwrapping it belongs here and not in every caller. */
type Answer<T> = T | { status: 'ok'; data: T } | { status: 'error'; error: { message?: string } }

export function unwrap<T>(answer: Answer<T>): Asked<T> {
  if (answer && typeof answer === 'object' && 'status' in answer) {
    return answer.status === 'ok'
      ? { data: answer.data, error: null, loading: false }
      : { data: null, error: answer.error?.message ?? 'the command failed', loading: false }
  }
  return { data: answer as T, error: null, loading: false }
}

export async function ask<T>(call: () => Promise<Answer<T>>): Promise<Asked<T>> {
  if (!inTauri()) return { data: null, error: 'not running in the app', loading: false }
  try {
    return unwrap(await call())
  } catch (thrown) {
    const error = thrown as { message?: string } | string
    return {
      data: null,
      error: typeof error === 'string' ? error : (error.message ?? 'the command failed'),
      loading: false,
    }
  }
}

export { commands }
