import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import { watchCsp } from './shell/csp'
import './index.css'

// Before anything renders: a policy that blocks something during startup is
// exactly the case nobody can debug from a blank window.
watchCsp()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
