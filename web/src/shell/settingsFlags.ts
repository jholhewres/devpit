import type { Settings } from '../gen/bindings'

/* The yes/no settings, named once. */
export type Flag = 'automaticUpdates' | 'confirmStop' | 'focusMode' | 'errorReports' | 'island'

/* Null is "never asked", and three of these default to on: updates, the
   prompt that stands between somebody and losing what a terminal was doing,
   and the island, which stays out of sight until an agent does something.
   The focus mode is off — unfinished things do not become the default by
   nobody having an opinion — and so are error reports, which are opt-in. */
export const flagOn = (flags: Partial<Settings> | null, field: Flag): boolean =>
  flags?.[field] ?? (field !== 'focusMode' && field !== 'errorReports')
