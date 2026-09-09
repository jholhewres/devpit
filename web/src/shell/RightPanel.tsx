import { useState } from 'react'

/* Explorer or Changes: two questions about the same tree, so one is
   answered at a time rather than both being half-visible. */

export function RightPanel(): React.JSX.Element {
  const [view, setView] = useState<'tree' | 'changes'>('tree')

  return (
    <aside className="rp">
        <div className="rp__bar">
          <button className="rtab" aria-selected={view === 'tree'} data-rview="tree" onClick={() => setView('tree')}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg>
            Explorer
          </button>
          <button className="rtab" aria-selected={view === 'changes'} data-rview="changes" onClick={() => setView('changes')}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v12M8 7l4-4 4 4" /><path d="M20 21H4" /></svg>
            Changes
            <span className="rtab__n">9</span>
          </button>
          <span className="drag"></span>
        </div>

        <div className="rview" data-rview="tree" data-open={String(view === 'tree')}>
          <div className="ex__head">
            <span className="ex__proj">devpit</span>
            <button className="sq26 tip" data-tip="Collapse all" aria-label="Collapse all"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h16M4 12h16M4 17h16" /></svg></button>
            <button className="sq26 tip" data-tip="Refresh" aria-label="Refresh"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-2.6-6.4" /><path d="M21 3v6h-6" /></svg></button>
            <button className="sq26 tip" data-tip="More" aria-label="More"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></svg></button>
          </div>

          <div className="ex__search">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
            <span className="ex__q" id="exQ">Search</span>
            <span className="ex__flags">
              <button className="flag tip" data-tip="Match case" aria-pressed="false">Aa</button>
              <button className="flag tip" data-tip="Whole word" aria-pressed="false"><u>ab</u></button>
              <button className="flag tip" data-tip="Regular expression" aria-pressed="false">.*</button>
            </span>
          </div>

          <div className="seg">
            <button className="segb" aria-selected="true" data-ex="names">Names</button>
            <button className="segb" aria-selected="false" data-ex="contents">Contents</button>
          </div>

          <div className="exmode" data-exmode="contents" hidden>
            <div className="scope">
              <div className="scope__l">Files to include</div>
              <div className="scope__f">*.rs, crates/**</div>
              <div className="scope__l">Files to exclude</div>
              <div className="scope__f">target/**, *.lock</div>
            </div>
            <div className="exempty">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
              <div className="exempty__t">Type to search in files</div>
              <div className="exempty__d">Matches come back grouped by file, with the line around each one.</div>
            </div>
          </div>

          <div className="exmode" data-exmode="names">
          <div className="tree">
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">apps</span></button>
              <button className="row" style={{paddingLeft: '18px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">desktop</span></button>
              <button className="row openfile" data-ctx="file" data-path="apps/desktop/src" data-name="files.rs" style={{paddingLeft: '30px'}}><span className="row__chev"></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="row__n">files.rs</span><span className="row__g g-m">M</span></button>
              <button className="row openfile" data-ctx="file" data-path="apps/desktop/src" data-name="listener.rs" style={{paddingLeft: '30px'}}><span className="row__chev"></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="row__n">listener.rs</span><span className="row__g g-m">M</span></button>
              <button className="row openfile" data-ctx="file" data-path="apps/desktop/src" data-name="roots.rs" style={{paddingLeft: '30px'}}><span className="row__chev"></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="row__n">roots.rs</span><span className="row__g g-a">A</span></button>
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">crates</span></button>
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">web</span></button>
              <button className="row openfile" data-ctx="file" data-path="" data-name="Cargo.toml" style={{paddingLeft: '6px'}}><span className="row__chev"></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="row__n">Cargo.toml</span></button>
              <button className="row openfile" data-ctx="file" data-path="" data-name="Makefile" style={{paddingLeft: '6px'}}><span className="row__chev"></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="row__n">Makefile</span></button>
          </div>
          </div>
        </div>

        <div className="rview" data-rview="changes">
          <div className="git">
            <div className="git__head">
              <div className="git__row">
                <button className="pr"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></svg>Create PR</button>
                <span className="drag"></span>
                <button className="sq26" aria-label="Search"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg></button>
                <button className="sq26" aria-label="More"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /></svg></button>
              </div>

              <div className="git__branch">main</div>
              <div className="git__up">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12h14M12 5l7 7-7 7" /></svg>
                origin/main
                <span className="drag"></span>
                <button className="sq26" aria-label="Open on the remote"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 3h6v6" /><path d="M10 14 21 3" /><path d="M21 14v7H3V3h7" /></svg></button>
              </div>

              <div className="msg">
                <span className="msg__ph">Message</span>
                <button className="msg__ai" aria-label="Write it for me"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v4M12 17v4M3 12h4M17 12h4" /><path d="m6.3 6.3 2.8 2.8M14.9 14.9l2.8 2.8M17.7 6.3l-2.8 2.8M9.1 14.9l-2.8 2.8" /></svg></button>
              </div>

              <div className="stage">
                <button className="stage__go"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14" /></svg>Stage All</button>
                <button className="stage__more" aria-label="More staging options"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></button>
              </div>
            </div>

            <div className="git__body">
              <button className="sect">
                <span className="sect__chev"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span>
                <span className="sect__t">Changes</span><span className="sect__n">9</span>
                <span className="sect__all">View all</span>
              </button>
              <button className="gitrow" style={{paddingLeft: '8px'}}><span className="gitrow__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="gitrow__n">apps/desktop/src</span><span className="gitrow__n2">5</span></button>
              <button data-ctx="file" className="gitrow gitrow--file openfile" data-path="" data-name="files.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">files.rs</span><span className="gitrow__end"><span className="add">+12</span><span className="del">&minus;3</span><span style={{color: 'var(--warning)'}}>M</span></span></button>
              <button data-ctx="file" className="gitrow gitrow--file openfile" data-path="" data-name="files_tests.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">files_tests.rs</span><span className="gitrow__end"><span className="add">+36</span><span style={{color: 'var(--warning)'}}>M</span></span></button>
              <button data-ctx="file" className="gitrow gitrow--file openfile" data-path="" data-name="listener.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">listener.rs</span><span className="gitrow__end"><span className="add">+17</span><span className="del">&minus;2</span><span style={{color: 'var(--warning)'}}>M</span></span></button>
              <button data-ctx="file" className="gitrow gitrow--new openfile" data-path="" data-name="listener_tests.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">listener_tests.rs</span><span className="gitrow__end"><span className="add">+85</span><span style={{color: 'var(--success)'}}>A</span></span></button>
              <button data-ctx="file" className="gitrow gitrow--new openfile" data-path="" data-name="roots.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">roots.rs</span><span className="gitrow__end"><span className="add">+35</span><span style={{color: 'var(--success)'}}>A</span></span></button>
              <button className="gitrow" style={{paddingLeft: '8px'}}><span className="gitrow__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="gitrow__n">crates/pty/src</span><span className="gitrow__n2">1</span></button>
              <button data-ctx="file" className="gitrow gitrow--file openfile" data-path="" data-name="ring.rs" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">ring.rs</span><span className="gitrow__end"><span className="add">+55</span><span style={{color: 'var(--warning)'}}>M</span></span></button>
              <button className="gitrow" style={{paddingLeft: '8px'}}><span className="gitrow__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="gitrow__n">web/src/theme</span><span className="gitrow__n2">1</span></button>
              <button data-ctx="file" className="gitrow gitrow--file openfile" data-path="" data-name="tokens.css" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">tokens.css</span><span className="gitrow__end"><span className="add">+24</span><span className="del">&minus;8</span><span style={{color: 'var(--warning)'}}>M</span></span></button>

              <button className="sect" style={{marginTop: '8px'}}>
                <span className="sect__chev"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span>
                <span className="sect__t">Untracked files</span><span className="sect__n">1</span>
                <span className="sect__all">View all</span>
              </button>
              <button className="gitrow" style={{paddingLeft: '8px'}}><span className="gitrow__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="gitrow__n">web/src/theme</span><span className="gitrow__n2">1</span></button>
              <button data-ctx="file" className="gitrow gitrow--new openfile" data-path="" data-name="tokens.test.ts" style={{paddingLeft: '22px'}}><span className="gitrow__chev"></span><span className="gitrow__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span><span className="gitrow__n">tokens.test.ts</span><span className="gitrow__end"><span className="add">+96</span><span style={{color: 'var(--success)'}}>U</span></span></button>
            </div>

            <div className="commits">
              <span className="sect__chev"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span>
              <span className="commits__t">Commits</span>
              <span className="drag"></span>
              <button className="sq26" aria-label="History"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg></button>
              <button className="sq26" aria-label="Refresh"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-2.6-6.4" /><path d="M21 3v6h-6" /></svg></button>
            </div>
          </div>
        </div>
      </aside>
  )
}
