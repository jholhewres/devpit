import { ask, commands } from './live'

/* Where the person's home is, for writing it as `~`: the folder that holds
   this build's own state folder. Asked once for every terminal. */
let home: Promise<string | null> | null = null
export function homeFolder(): Promise<string | null> {
  home ??= ask(() => commands.appInfo()).then((answer) => {
    const state = answer.data?.statePath
    // `<home>/.devpit/state.db`: two steps up, not one.
    return state ? state.replace(/\/[^/]+\/[^/]+\/?$/, '') || null : null
  })
  return home
}
