import mark from '../assets/brand/mark.png'
import { useShell } from './useShell'

/*
 * One pane visible at a time, in the order the strip gives them.
 *
 * The content here is the prototype's, and it stays in the markup rather
 * than in `mock/data.ts`: a board column's cards are what the backend will
 * replace wholesale, not field by field.
 */

export function Panes(): React.JSX.Element {
  const { open, active, show, close, project } = useShell()

  return (
    <section className="mid">

        <div className="mid__body">
          <div className="panes" data-empty={String(open.length === 0)}>
            <div className="blank">
              <span className="blank__mark"><img className="mark" alt="" src={mark} /></span>
              <div className="blank__name">{project}</div>
              <div className="blank__sub">Nothing open. Pick something on the left, or start here.</div>
              <div className="blank__keys">
                <button className="blank__k" onClick={() => show('chat')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg>
                  <b>New chat</b><span>&#8984;N</span>
                </button>
                <button className="blank__k" onClick={() => show('term')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg>
                  <b>New terminal</b><span>&#8984;T</span>
                </button>
                <button className="blank__k" onClick={() => show('board')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg>
                  <b>Board</b><span>&#8984;B</span>
                </button>
              </div>
            </div>


            {/* Board */}
            <div className="pane" data-pane="board" data-show={String(active === 'board')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
                <span className="pane__t"><b>Board</b> · 11 cards</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('board')} aria-label="Close board"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="board" id="board">
                <div className="blane" data-lane="inbox">
                  <div className="blane__top">
                    <span className="blane__label">Inbox</span>
                    <span className="blane__n">3</span>
                  </div>
                  <div className="blane__list">
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">Persist the sidebar width</div>
                    <div className="tile__m"><span className="tile__time">2d</span></div>
                  </div>
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">File-type icons in the tree</div>
                    <div className="tile__m"><span className="tile__time">2d</span></div>
                  </div>
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">Split the middle horizontally</div>
                    <div className="tile__m"><span className="tile__time">1d</span></div>
                  </div>
                  </div>
                  <button className="tile__add">+ Add card</button>
                  <div className="blane__fill"></div>
                </div>
                <div className="blane" data-lane="refine" data-agent="planner">
                  <div className="blane__top">
                    <span className="blane__label">Refine</span>
                    <span className="blane__n">1</span>
                    <span className="blane__agent"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>planner</span>
                  </div>
                  <div className="blane__list">
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">Board opens in the content</div>
                    <div className="tile__m"><span className="st-wait"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 9v4M12 17h.01" /><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" /></svg></span><span className="tile__agent">planner</span><span className="tile__time">needs you</span></div>
                  </div>
                  </div>
                  <button className="tile__add">+ Add card</button>
                  <div className="blane__fill"></div>
                </div>
                <div className="blane" data-lane="doing" data-agent="executor">
                  <div className="blane__top">
                    <span className="blane__label">Doing</span>
                    <span className="blane__n">1</span>
                    <span className="blane__agent"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>executor</span>
                  </div>
                  <div className="blane__list">
                  <div className="tile" role="button" tabIndex={0} data-ctx="card" aria-current="true">
                    <div className="tile__t">Rebuild the shell on GPUI</div>
                    <div className="tile__m"><span className="st-work spin"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round"><path d="M21 12a9 9 0 1 1-6.2-8.6" /></svg></span><span className="tile__agent">executor</span><span className="tile__time">3m</span></div>
                  </div>
                  </div>
                  <button className="tile__add">+ Add card</button>
                  <div className="blane__fill"></div>
                </div>
                <div className="blane" data-lane="check" data-agent="reviewer">
                  <div className="blane__top">
                    <span className="blane__label">Check</span>
                    <span className="blane__n">1</span>
                    <span className="blane__agent"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>reviewer</span>
                  </div>
                  <div className="blane__list">
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">Two read ceilings were guarding nothing</div>
                    <div className="tile__m"><span className="stat"><span className="add">+70</span> <span className="del">−35</span></span><span className="tile__time">28m</span></div>
                  </div>
                  </div>
                  <button className="tile__add">+ Add card</button>
                  <div className="blane__fill"></div>
                </div>
                <div className="blane" data-lane="ship">
                  <div className="blane__top">
                    <span className="blane__label">Ship</span>
                    <span className="blane__n">2</span>
                  </div>
                  <div className="blane__list">
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">The Changes panel opens a diff</div>
                    <div className="tile__m"><span className="tile__time">merged · 3h</span></div>
                  </div>
                  <div className="tile" role="button" tabIndex={0} data-ctx="card">
                    <div className="tile__t">Background sessions get hooks too</div>
                    <div className="tile__m"><span className="tile__time">merged · 5h</span></div>
                  </div>
                  </div>
                  <button className="tile__add">+ Add card</button>
                  <div className="blane__fill"></div>
                </div>
              </div>
            </div>

            {/* Skills */}
            <div className="pane" data-pane="skills" data-show={String(active === 'skills')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                <span className="pane__t"><b>Skills</b> · 5 agents</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('skills')} aria-label="Close skills"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="skills">
                <div className="skills__in">
                  <p className="skills__note">Which agent a lane wakes. The binding is devpit&rsquo;s &mdash; nothing else knows that Check calls the reviewer &mdash; while the definitions are ordinary agent files the CLI already understands, so one you already wrote works here unchanged. A lane with no agent moves the card and nothing else.</p>
                <div className="skill">
                  <span className="skill__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div className="skill__body">
                    <div className="skill__top">
                      <span className="skill__name">planner</span>
                      <span className="skill__lane">Refine</span>
                      <span className="skill__model">claude-opus-5</span>
                    </div>
                    <div className="skill__what">Reads the card, asks what is missing, and writes the plan back onto it. Stops for you rather than guessing.</div>
                  </div>
                </div>
                <div className="skill">
                  <span className="skill__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div className="skill__body">
                    <div className="skill__top">
                      <span className="skill__name">executor</span>
                      <span className="skill__lane">Doing</span>
                      <span className="skill__model">claude-opus-5</span>
                    </div>
                    <div className="skill__what">Takes the plan and writes the code, in the card&rsquo;s own worktree. The one skill allowed to edit files.</div>
                  </div>
                </div>
                <div className="skill">
                  <span className="skill__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div className="skill__body">
                    <div className="skill__top">
                      <span className="skill__name">reviewer</span>
                      <span className="skill__lane">Check</span>
                      <span className="skill__model">claude-opus-5</span>
                    </div>
                    <div className="skill__what">Reads the diff against the plan and reports what it cannot verify. Never edits.</div>
                  </div>
                </div>
                <div className="skill">
                  <span className="skill__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div className="skill__body">
                    <div className="skill__top">
                      <span className="skill__name">test-runner</span>
                      <span className="skill__lane">Check</span>
                      <span className="skill__model">claude-haiku-4.5</span>
                    </div>
                    <div className="skill__what">Runs the project&rsquo;s own test command and attaches the output. Cheap on purpose &mdash; it reads a exit code, not a codebase.</div>
                  </div>
                </div>
                <div className="skill">
                  <span className="skill__ico"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg></span>
                  <div className="skill__body">
                    <div className="skill__top">
                      <span className="skill__name">summariser</span>
                      <span className="skill__off">no lane</span>
                      <span className="skill__model">claude-haiku-4.5</span>
                    </div>
                    <div className="skill__what">Condenses a long session into what the next one needs. Called by hand, from the chat.</div>
                  </div>
                </div>
                </div>
              </div>
            </div>

            <div className="pane" data-pane="caps" data-show={String(active === 'caps')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2 4 6v6c0 5 3.4 9.1 8 10 4.6-.9 8-5 8-10V6Z" /><path d="m9 12 2 2 4-4" /></svg></span>
                <span className="pane__t"><b>Capabilities</b> · 3 installed, 2 on</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('caps')} aria-label="Close Capabilities"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="list">
                <div className="list__in">
                  <p className="list__note">Plugins for devpit. Install one and it lands in the workspace; switch it on and it becomes a surface you can open beside the work. Off, it is not loaded at all, so a plugin you are not using costs nothing to keep.</p>
                  <div className="list__h">Installed</div>
                  <button className="cap" role="switch" aria-checked="true">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 4h6v6H4zM14 14h6v6h-6z" /><path d="M10 7h4a3 3 0 0 1 3 3v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="cap__name">Diagram</span>
                        <span className="cap__ver">1.4.0</span>
                        <span className="cap__where">pane</span>
                      </span>
                      <span className="cap__what">Draws <code>mermaid</code> blocks in a chat instead of printing them, and opens one full size as its own pane.</span>
                    </span>
                    <span className="sw"></span>
                  </button>
                  <button className="cap" role="switch" aria-checked="true">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3Z" /><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18Z" /><path d="M2 2l7.6 7.6" /><circle cx="11" cy="11" r="2" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="cap__name">Excalidraw</span>
                        <span className="cap__ver">0.9.2</span>
                        <span className="cap__where">pane</span>
                      </span>
                      <span className="cap__what">A sketch surface attached to a card, so a drawing outlives the conversation that produced it.</span>
                    </span>
                    <span className="sw"></span>
                  </button>
                  <button className="cap" role="switch" aria-checked="false">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="cap__name">Web preview</span>
                        <span className="cap__ver">1.1.0</span>
                        <span className="cap__where">pane</span>
                      </span>
                      <span className="cap__what">Serves the project and shows it beside the terminal that builds it. Reloads when the build does.</span>
                    </span>
                    <span className="sw"></span>
                  </button>

                  <div className="list__h">Available</div>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" /><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="cap__name">Notes</span>
                        <span className="cap__ver">0.6.1</span>
                        <span className="cap__size">84 kB</span>
                      </span>
                      <span className="cap__what">A markdown page per card, kept in the worktree so it travels with the branch.</span>
                    </span>
                    <button className="cap__get" data-install="Notes">Install</button>
                  </div>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><path d="M3 7h3l2-3h8l2 3h3v13H3Z" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="cap__name">Screenshots</span>
                        <span className="cap__ver">0.3.0</span>
                        <span className="cap__size">210 kB</span>
                      </span>
                      <span className="cap__what">Captures a pane to an image and attaches it to the card or the chat.</span>
                    </span>
                    <button className="cap__get" data-install="Screenshots">Install</button>
                  </div>

                  <button className="browse">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
                    Browse capabilities
                  </button>
                </div>
              </div>
            </div>

            <div className="pane" data-pane="mcps" data-show={String(active === 'mcps')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                <span className="pane__t"><b>MCPs</b> · 2 of 4 connected</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('mcps')} aria-label="Close MCPs"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="list">
                <div className="list__in">
                  <p className="list__note">What Claude Code resolved for this project, read from its own config &mdash; devpit does not keep a second copy. Add or remove one with <code>claude mcp add</code> and it changes here. Shown because a server that failed to connect looks exactly like a tool the agent never had, until you check.</p>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="mcp__dot" style={{background: 'var(--success)'}}></span>
                        <span className="cap__name">anchored</span><span className="cap__src">user</span>
                        <span className="mcp__tools">18 tools</span>
                      <span className="cap__file"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg><span>npx @anchored/mcp --stdio</span></span>
                      </span>
                  
                    </span>
                  </div>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="mcp__dot" style={{background: 'var(--success)'}}></span>
                        <span className="cap__name">playwright</span><span className="cap__src">user</span>
                        <span className="mcp__tools">24 tools</span>
                      <span className="cap__file"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg><span>npx @playwright/mcp@latest</span></span>
                      </span>
                  
                    </span>
                  </div>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="mcp__dot" style={{background: 'var(--danger)'}}></span>
                        <span className="cap__name">github</span><span className="cap__src">project</span>
                        <span className="mcp__tools">—</span>
                      <span className="cap__file"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg><span>https://api.githubcopilot.com/mcp/</span></span>
                      </span>
                      <span className="mcp__err">400 · Authorization header is badly formatted</span>
                    </span>
                  </div>
                  <div className="cap cap--read">
                    <span className="cap__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M9 2v6M15 2v6" /><path d="M6 8h12v4a6 6 0 0 1-12 0Z" /><path d="M12 18v4" /></svg></span>
                    <span className="cap__body">
                      <span className="cap__top">
                        <span className="mcp__dot" style={{background: 'var(--ghost)'}}></span>
                        <span className="cap__name">datadog</span><span className="cap__src">plugin</span>
                        <span className="mcp__tools">—</span>
                      <span className="cap__file"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg><span>https://mcp.datadoghq.com/api/unstable/mcp-server/mcp</span></span>
                      </span>
                  
                    </span>
                  </div>
                  <button className="browse">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg>
                    Manage with claude mcp
                  </button>
                </div>
              </div>
            </div>

            <div className="pane" data-pane="files" data-show={String(active === 'files')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
                <span className="pane__t"><b>Files</b> · devpit</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('files')} aria-label="Close files"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="fx">
                <div className="fx__tree">
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">apps</span></button>
              <button className="row" style={{paddingLeft: '20px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">desktop</span></button>
              <button className="row" aria-current="true" style={{paddingLeft: '34px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">src</span></button>
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">crates</span></button>
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">web</span></button>
              <button className="row" style={{paddingLeft: '6px'}}><span className="row__chev"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m9 18 6-6-6-6" /></svg></span><span className="row__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span><span className="row__n">xtask</span></button>
                </div>
                <div className="fx__main">
                  <div className="fx__crumb">apps / desktop / <b>src</b></div>
                  <div className="fx__list">
                    <div className="fxrow fxhead"><span>Name</span><span>Git</span><span className="fxrow__z">Size</span><span className="fxrow__t">Modified</span></div>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>board.rs</span></span><span></span><span className="fxrow__z">11.4 kB</span><span className="fxrow__t">3 days ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>columns.rs</span></span><span></span><span className="fxrow__z">2.1 kB</span><span className="fxrow__t">5 days ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>contract.rs</span></span><span></span><span className="fxrow__z">1.8 kB</span><span className="fxrow__t">5 days ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>files.rs</span></span><span className="fxrow__g g-m">M</span><span className="fxrow__z">4.2 kB</span><span className="fxrow__t">4 min ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>files_tests.rs</span></span><span className="fxrow__g g-m">M</span><span className="fxrow__z">5.0 kB</span><span className="fxrow__t">4 min ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>listener.rs</span></span><span className="fxrow__g g-m">M</span><span className="fxrow__z">3.7 kB</span><span className="fxrow__t">11 min ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>listener_tests.rs</span></span><span className="fxrow__g g-a">A</span><span className="fxrow__z">2.9 kB</span><span className="fxrow__t">11 min ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>main.rs</span></span><span></span><span className="fxrow__z">3.3 kB</span><span className="fxrow__t">2 days ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>roots.rs</span></span><span className="fxrow__g g-a">A</span><span className="fxrow__z">1.2 kB</span><span className="fxrow__t">18 min ago</span></button>
                <button className="fxrow" data-ctx="file"><span className="fxrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span>steps</span></span><span></span><span className="fxrow__z">4 items</span><span className="fxrow__t">1 day ago</span></button>
                  </div>
                </div>
              </div>
            </div>

            <div className="pane" data-pane="workspace" data-show={String(active === 'workspace')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
                <span className="pane__t"><b>Workspace</b> · devpit</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('workspace')} aria-label="Close workspace"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="list">
                <div className="list__in">
                  <p className="list__note">What devpit keeps for this project. None of it is in the repository &mdash; the board, the skills and the transcripts follow the machine, not the branch, so a clone of <code>devpit</code> stays a clone of <code>devpit</code>.</p>
                  <div className="where">
                    <span className="where__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg></span>
                    <span className="where__path">~/.devpit/workspaces/<b>devpit</b></span>
                    <button className="where__go">Reveal</button>
                  </div>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span>skills/</span></span>
                      <span className="wsrow__w">Agent definitions, one markdown file each</span>
                      <span className="wsrow__z">5 files</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span>capabilities/</span></span>
                      <span className="wsrow__w">Installed plugins</span>
                      <span className="wsrow__z">5 dirs</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>mcp.json</span></span>
                      <span className="wsrow__w">Servers the agents may call</span>
                      <span className="wsrow__z">1.1 kB</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>board.db</span></span>
                      <span className="wsrow__w">Cards, lanes and their history</span>
                      <span className="wsrow__z">84 kB</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span>sessions/</span></span>
                      <span className="wsrow__w">Transcripts, one per chat</span>
                      <span className="wsrow__z">412 files</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg><span>worktrees/</span></span>
                      <span className="wsrow__w">A checkout per card in flight</span>
                      <span className="wsrow__z">3 dirs</span>
                    </button>
                    <button className="wsrow">
                      <span className="wsrow__n"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg><span>settings.json</span></span>
                      <span className="wsrow__w">What this project overrides</span>
                      <span className="wsrow__z">0.4 kB</span>
                    </button>
                </div>
              </div>
            </div>

            <div className="pane" data-pane="diagram" data-show={String(active === 'diagram')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 4h6v6H4zM14 14h6v6h-6z" /><path d="M10 7h4a3 3 0 0 1 3 3v4" /></svg></span>
                <span className="pane__t"><b>Diagram</b> &middot; board-flow.mmd</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('diagram')} aria-label="Close diagram"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="draw">
                <div className="draw__canvas">
                  <svg width="700" height="200" viewBox="0 0 700 200" role="img" aria-label="Board flow: Inbox to Refine to Doing to Check to Ship">
                    <text x="30" y="34" fontSize="12" fill="var(--text-3)" fontFamily="system-ui">How a card crosses the board</text>
                    <rect x="30" y="60" width="104" height="52" rx="9" fill="var(--raised)" stroke="hsla(220,10%,90%,0.14)" strokeWidth="1" /><text x="82" y="82" textAnchor="middle" fontSize="13" fill="var(--text)" fontFamily="system-ui">Inbox</text><path d="M138 86 L156 86" stroke="var(--text-3)" strokeWidth="1.2" fill="none" /><path d="m151 82 5 4-5 4" stroke="var(--text-3)" strokeWidth="1.2" fill="none" strokeLinecap="round" strokeLinejoin="round" /><rect x="162" y="60" width="104" height="52" rx="9" fill="var(--raised)" stroke="var(--accent)" strokeWidth="1" /><text x="214" y="82" textAnchor="middle" fontSize="13" fill="var(--text)" fontFamily="system-ui">Refine</text><text x="214" y="99" textAnchor="middle" fontSize="10.5" fill="var(--accent)" fontFamily="ui-monospace, monospace">planner</text><path d="M270 86 L288 86" stroke="var(--text-3)" strokeWidth="1.2" fill="none" /><path d="m283 82 5 4-5 4" stroke="var(--text-3)" strokeWidth="1.2" fill="none" strokeLinecap="round" strokeLinejoin="round" /><rect x="294" y="60" width="104" height="52" rx="9" fill="var(--raised)" stroke="var(--accent)" strokeWidth="1" /><text x="346" y="82" textAnchor="middle" fontSize="13" fill="var(--text)" fontFamily="system-ui">Doing</text><text x="346" y="99" textAnchor="middle" fontSize="10.5" fill="var(--accent)" fontFamily="ui-monospace, monospace">executor</text><path d="M402 86 L420 86" stroke="var(--text-3)" strokeWidth="1.2" fill="none" /><path d="m415 82 5 4-5 4" stroke="var(--text-3)" strokeWidth="1.2" fill="none" strokeLinecap="round" strokeLinejoin="round" /><rect x="426" y="60" width="104" height="52" rx="9" fill="var(--raised)" stroke="var(--accent)" strokeWidth="1" /><text x="478" y="82" textAnchor="middle" fontSize="13" fill="var(--text)" fontFamily="system-ui">Check</text><text x="478" y="99" textAnchor="middle" fontSize="10.5" fill="var(--accent)" fontFamily="ui-monospace, monospace">reviewer</text><path d="M534 86 L552 86" stroke="var(--text-3)" strokeWidth="1.2" fill="none" /><path d="m547 82 5 4-5 4" stroke="var(--text-3)" strokeWidth="1.2" fill="none" strokeLinecap="round" strokeLinejoin="round" /><rect x="558" y="60" width="104" height="52" rx="9" fill="var(--raised)" stroke="hsla(220,10%,90%,0.14)" strokeWidth="1" /><text x="610" y="82" textAnchor="middle" fontSize="13" fill="var(--text)" fontFamily="system-ui">Ship</text>
                    <text x="30" y="150" fontSize="11" fill="var(--ghost)" fontFamily="ui-monospace, monospace">a lane with an agent wakes it on arrival</text>
                  </svg>
                </div>
                <div className="draw__foot">workspace/capabilities/diagram &middot; rendered from mermaid</div>
              </div>
            </div>

            <div className="pane" data-pane="excalidraw" data-show={String(active === 'excalidraw')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3Z" /><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18Z" /><path d="M2 2l7.6 7.6" /><circle cx="11" cy="11" r="2" /></svg></span>
                <span className="pane__t"><b>Excalidraw</b> &middot; gpui-shell.excalidraw</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('excalidraw')} aria-label="Close excalidraw"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="draw">
                <div className="draw__tools">
                <button className="dtool" aria-pressed="true"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m3 3 7.1 17 2.5-7.4 7.4-2.5Z" /></svg></button>
                <button className="dtool" aria-pressed="false"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="4" y="5" width="16" height="14" rx="2" /></svg></button>
                <button className="dtool" aria-pressed="false"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="8" /></svg></button>
                <button className="dtool" aria-pressed="false"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 18 20 6" /></svg></button>
                <button className="dtool" aria-pressed="false"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h16M4 12h10M4 17h13" /></svg></button>
                  <span className="draw__sep"></span>
                  <button className="dtool" aria-pressed="false"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" /></svg></button>
                </div>
                <div className="draw__canvas">
                  <svg width="640" height="330" viewBox="0 0 640 330" role="img" aria-label="A sketch of the window layout">
                    <g stroke="var(--text-2)" fill="none" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M40 40 q140-4 262-2 t258 3 q3 100 1 210 -180 5-262 3 t-258-4 q-4-104-1-210Z" />
                      <path d="M158 41 q4 104 2 212" />
                      <path d="M452 43 q-3 102-1 210" />
                      <path d="M42 88 q260 3 516-1" />
                    </g>
                    <text x="62" y="70" fontSize="13" fill="var(--accent)" fontFamily="system-ui">top &mdash; project + branch</text>
                    <text x="60" y="130" fontSize="12.5" fill="var(--text-2)" fontFamily="system-ui">board</text>
                    <text x="60" y="150" fontSize="11" fill="var(--text-3)" fontFamily="system-ui">lanes as groups</text>
                    <text x="180" y="130" fontSize="12.5" fill="var(--text-2)" fontFamily="system-ui">panes, split</text>
                    <text x="180" y="150" fontSize="11" fill="var(--text-3)" fontFamily="system-ui">chat &middot; terminal &middot; file</text>
                    <text x="472" y="130" fontSize="12.5" fill="var(--text-2)" fontFamily="system-ui">files</text>
                    <g stroke="var(--accent)" fill="none" strokeWidth="1.6" strokeLinecap="round">
                      <path d="M196 186 q56 24 118 6" />
                      <path d="m306 186 8 6-9 7" />
                    </g>
                    <text x="196" y="216" fontSize="11.5" fill="var(--accent)" fontFamily="system-ui">opening a second one splits</text>
                  </svg>
                </div>
                <div className="draw__foot">workspace/capabilities/excalidraw &middot; saved with the card</div>
              </div>
            </div>

            {/* Chat */}
            <div className="pane" data-pane="chat" data-show={String(active === 'chat')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg></span>
                <span className="pane__t"><b>Chat</b> · working 3m</span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('chat')} aria-label="Close chat"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>

              <div className="scroll">
                <div className="thread">
                  <article className="turn">
                    <div className="said">O teto de leitura do <code>file_read</code> tem teste? Quebra ele e vê se alguém reclama.</div>
                  </article>
                  <article className="turn">
                    <div className="act-line">
                      <span className="act-line__ico"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span>
                      <span className="act-line__t">apps/desktop/src/files.rs</span>
                    </div>
                    <div className="reply">
                      <p>Quebrei. Ninguém reclamou — o teto de 2&nbsp;MB não tinha guarda nenhuma.</p>
                      <p>Extraí a regra para <code>past_the_ceiling</code> e escrevi três casos.</p>
                    </div>
                    <div className="turn__foot">
                      <button className="tfbtn" aria-label="Copy"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg></button>
                      <span className="turn__time">5:02 PM</span>
                    </div>
                  </article>
                  <article className="turn">
                    <div className="said">E o do listener?</div>
                  </article>
                  <article className="turn">
                    <div className="reply">
                      <p>Esse sobreviveu ao meu próprio teste novo. Eu mandava só o header: com teto recusa pelo tamanho, sem teto recusa pelo corpo curto — duas rotas, uma resposta.</p>
                    </div>
                    <div className="turn__foot">
                      <button className="tfbtn" aria-label="Copy"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg></button>
                      <span className="turn__time">5:06 PM</span>
                    </div>
                  </article>
                </div>
              </div>

              <div className="composer">
                <div className="composer__in">
                  <div className="composer__ph">Do anything…</div>
                  <div className="composer__row">
                    <button className="chip">
                      <span className="chip__sun"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M6.3 17.7l-1.4 1.4M19.1 4.9l-1.4 1.4" /></svg></span>
                      Default
                    </button>
                    <button className="chip">High</button>
                    <button className="chip">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg>
                      Full access
                    </button>
                    <button className="send" aria-label="Send">
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19V5M5 12l7-7 7 7" /></svg>
                    </button>
                  </div>
                </div>
                <div className="wsbar">
                  <button className="wschip">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" /></svg>
                    Local
                  </button>
                  <button className="wschip">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></svg>
                    main
                  </button>
                  <span className="drag"></span>
                </div>
              </div>
            </div>

            {/* A file, open in the middle */}
            <div className="pane pane--file" data-pane="file" data-show={String(active === 'file')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" /><path d="M14 2v5h5" /></svg></span>
                <span className="pane__t" id="fileTitle"><b>files.rs</b> · apps/desktop/src</span>
                <span className="drag"></span>
                <span className="netstat"><span className="add">+12</span><span className="del">−3</span></span>
                <button className="sq26" onClick={() => close('file')} aria-label="Close file"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="skel">
                <div className="skel__ln" style={{width: '62%'}}></div>
                <div className="skel__ln" style={{width: '38%'}}></div>
                <div className="skel__ln" style={{width: '78%'}}></div>
                <div className="skel__ln" style={{width: '54%'}}></div>
                <div className="skel__ln" style={{width: '70%'}}></div>
                <div className="skel__ln" style={{width: '44%'}}></div>
                <div className="skel__ln" style={{width: '66%'}}></div>
              </div>
              <div className="code">
    <div className="ln"><span className="ln__n">100</span><span className="ln__c"><span className="cm">/// Why a file is too big to open, or nothing.</span></span></div>
    <div className="ln"><span className="ln__n">101</span><span className="ln__c"><span className="cm">///</span></span></div>
    <div className="ln"><span className="ln__n">102</span><span className="ln__c"><span className="cm">/// A function of its own so the test calls the</span></span></div>
    <div className="ln"><span className="ln__n">103</span><span className="ln__c"><span className="cm">/// rule rather than a copy of it.</span></span></div>
    <div className="ln"><span className="ln__n">104</span><span className="ln__c"><span className="kw">fn</span> <span className="fn">past_the_ceiling</span>(path: &<span className="kw">str</span>, bytes: <span className="kw">u64</span>) -&gt; Option&lt;String&gt; {'{'}</span></div>
    <div className="ln"><span className="ln__n">105</span><span className="ln__c">    <span className="kw">if</span> bytes &lt;= MOST_BYTES {'{'}</span></div>
    <div className="ln"><span className="ln__n">106</span><span className="ln__c">        <span className="kw">return</span> None;</span></div>
    <div className="ln"><span className="ln__n">107</span><span className="ln__c">    {'}'}</span></div>
    <div className="ln"><span className="ln__n">108</span><span className="ln__c">    Some(<span className="fn">format!</span>(</span></div>
    <div className="ln"><span className="ln__n">109</span><span className="ln__c">        <span className="st">"{'{'}path{'}'} is {'{'}:.1{'}'} MB — past the {'{'}{'}'} MB this opens"</span>,</span></div>
    <div className="ln"><span className="ln__n">110</span><span className="ln__c">        bytes <span className="kw">as</span> <span className="kw">f64</span> / <span className="st">1_048_576.0</span>,</span></div>
    <div className="ln"><span className="ln__n">111</span><span className="ln__c">        MOST_BYTES / <span className="st">1_048_576</span></span></div>
    <div className="ln"><span className="ln__n">112</span><span className="ln__c">    ))</span></div>
    <div className="ln"><span className="ln__n">113</span><span className="ln__c">{'}'}</span></div>
              </div>
            </div>

            {/* Terminal */}
            <div className="pane pane--term" data-pane="term" data-show={String(active === 'term')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg></span>
                <span className="pane__t"><b>&hellip;vate/devpit</b></span>
                <span className="drag"></span>
                <button className="sq26 tip" data-tip="New terminal" aria-label="New terminal"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14" /></svg></button>
                <button className="sq26" onClick={() => close('term')} aria-label="Close terminal"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <div className="termbody"><span className="t-arrow">&#10230;</span>  <span className="t-dir">devpit</span> <span className="t-git">git:(</span><span className="t-branch">main</span><span className="t-git">)</span> <span className="t-dirty">&#10007;</span> make test

    <span className="t-ok">running 37 tests</span>
    .....................................
    test result: <span className="t-ok">ok</span>. 37 passed

    <span className="t-warn">all green</span>

    <span className="t-arrow">&#10230;</span>  <span className="t-dir">devpit</span> <span className="t-git">git:(</span><span className="t-branch">main</span><span className="t-git">)</span> <span className="t-dirty">&#10007;</span> <span className="caret"></span></div>
            </div>

          </div>
        </div>
      </section>
  )
}
