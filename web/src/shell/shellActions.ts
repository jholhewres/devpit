import { useCallback, useMemo } from 'react'

/*
 * The shell's own actions, stable: a component that picks one with
 * `useShellPick` then re-renders only for what it reads. Written inline in
 * the provider they were new on every change, and every pick holding one
 * changed with them.
 */
export function useShellActions({
  setSide,
  setFiles,
  setPalette,
  pickProject,
  closePrefs,
  startSignIn,
  endSignOut,
}: {
  setSide: React.Dispatch<React.SetStateAction<boolean>>
  setFiles: React.Dispatch<React.SetStateAction<boolean>>
  setPalette: React.Dispatch<React.SetStateAction<boolean>>
  pickProject: (id: string) => void
  closePrefs: () => void
  startSignIn: () => Promise<void>
  endSignOut: () => Promise<void>
}) {
  const toggleSide = useCallback(() => setSide((was) => !was), [setSide])
  const toggleFiles = useCallback(() => setFiles((was) => !was), [setFiles])
  const openPalette = useCallback(() => setPalette(true), [setPalette])
  const closePalette = useCallback(() => setPalette(false), [setPalette])
  const setProject = useCallback(
    (id: string) => {
      pickProject(id)
      closePrefs()
    },
    [pickProject, closePrefs],
  )
  const signIn = useCallback(() => void startSignIn(), [startSignIn])
  const signOut = useCallback(() => {
    void endSignOut()
    /* Signing out from inside Settings leaves a screen about an account
       that is gone. */
    closePrefs()
  }, [endSignOut, closePrefs])
  return useMemo(
    () => ({ toggleSide, toggleFiles, openPalette, closePalette, setProject, signIn, signOut }),
    [toggleSide, toggleFiles, openPalette, closePalette, setProject, signIn, signOut],
  )
}
