import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import { answerBeforeRestart } from './shell/beforeRestart'
import { watchCsp } from './shell/csp'
import './index.css'

// Before anything renders: a policy that blocks something during startup is
// exactly the case nobody can debug from a blank window.
watchCsp()
// And the window answers when an update is about to restart it.
answerBeforeRestart()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
