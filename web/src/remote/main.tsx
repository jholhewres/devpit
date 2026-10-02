import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import '@xterm/xterm/css/xterm.css'
import './remote.css'
import { RemoteApp } from './RemoteApp'

/*
 * The remote viewer: this machine's terminals, board, questions and chats,
 * from another device on the person's tailnet. Served by the machine itself.
 */
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <RemoteApp />
  </StrictMode>,
)
