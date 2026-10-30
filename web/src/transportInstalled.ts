/* Imported first by `main.tsx`, for this one call. A module of its own because
   imports are hoisted: a call written between them in `main.tsx` would run
   after every imported module had already evaluated. */
import { installBrowserTransport } from './transport'

installBrowserTransport()
