/*
 * devpit Plus is not built yet; its waiting list lives on devpit.app/pro.
 * The link says only where in the app it was clicked, so the list counts
 * which need brought people there — nothing else leaves the app.
 */

export const PLUS = 'https://devpit.app/pro'

/** The places the app offers the list: where the need for Plus shows. */
export type PlusFrom = 'app-account' | 'app-remote' | 'app-channels'

export const plusLink = (from: PlusFrom): string => `${PLUS}?${new URLSearchParams({ from }).toString()}`
