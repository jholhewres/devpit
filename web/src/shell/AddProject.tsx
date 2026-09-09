/* Two ways to add a project, which is one decision — so it is one sheet
   with a choice inside, not two commands side by side. */

export function AddProject({ onClose }: { onClose: () => void }): React.JSX.Element {
  return (
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="addpj__box" role="dialog" aria-modal="true" aria-labelledby="addpjT">
        <button className="auth__x" aria-label="Close" onClick={onClose}><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
        <h2 className="addpj__t" id="addpjT">Add a project</h2>
        <p className="addpj__d">Either way it joins your project list.</p>
        <button className="addpj__o" onClick={onClose}><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span><span className="addpj__ot">Open a folder</span>
          <span className="addpj__od">A repository already on this computer. devpit reads it where it
            is and never moves it.</span></span></button>
        <button className="addpj__o" onClick={onClose}><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v12" /><path d="m8 11 4 4 4-4" /><path d="M4 19h16" /></svg><span><span className="addpj__ot">Clone a repository</span>
          <span className="addpj__od">Fetch it from a remote first, then choose where it lands.</span></span></button>
      </div>
    </div>
  )
}
