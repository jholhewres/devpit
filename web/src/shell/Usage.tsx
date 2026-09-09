/*
 * What the agents cost, in the shape the prototype showed.
 *
 * Still the prototype's numbers: nothing reads a real bill yet. It is on the
 * M9 list, with every other control that draws something it does not know.
 */
export function Usage(): React.JSX.Element {
  return (
          <div className="use">
            <div className="use__top">
              <div>
                <h1 className="prefs__h" style={{margin: '0'}}>Usage</h1>
                <div className="use__when">Aug 10 to Sep 8</div>
              </div>
              <div className="seg2">
                <button aria-selected="true">Daily</button>
                <button aria-selected="false">Monthly</button>
                <button aria-selected="false">Projects</button>
              </div>
              <button className="card2__go">Last 30 days <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m6 9 6 6 6-6" /></svg></button>
            </div>

            <div className="use__grid">
              <div>
                <div className="use__label">Raw token cost</div>
                <div className="use__big">$7,211.18<span style={{color: 'var(--text-3)'}}>*</span></div>
                <div className="use__note">* if billed at the full API rate</div>

                <div className="bar">
                  <div className="bar__top"><span className="bar__n">Claude Code</span><span className="bar__v">$7,183.70</span></div>
                  <div className="bar__track"><div className="bar__fill" style={{width: '99.6%'}}></div></div>
                  <div className="bar__sub">99.6% of cost · 11.9B tokens</div>
                </div>
                <div className="bar">
                  <div className="bar__top"><span className="bar__n">Codex</span><span className="bar__v">$27.48</span></div>
                  <div className="bar__track"><div className="bar__fill" style={{width: '2%', background: 'var(--text-3)'}}></div></div>
                  <div className="bar__sub">0.4% of cost · 44.8M tokens</div>
                </div>
              </div>

              <div>
                <div className="use__hrow"><span className="use__h">Daily cost</span></div>
                <svg width="100%" viewBox="0 0 640 190" role="img" aria-label="Daily cost, Aug 10 to Sep 8" preserveAspectRatio="none" style={{height: '190px'}}>
                  <defs>
                    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="var(--accent)" stop-opacity="0.30" />
                      <stop offset="100%" stopColor="var(--accent)" stop-opacity="0.02" />
                    </linearGradient>
                  </defs>
                  <line x1="0" y1="12.0" x2="640" y2="12.0" stroke="var(--border)" strokeWidth="1" />
                  <line x1="0" y1="88.0" x2="640" y2="88.0" stroke="var(--border)" strokeWidth="1" />
                  <line x1="0" y1="164" x2="640" y2="164" stroke="var(--border)" strokeWidth="1" />
                  <path d="M0.0 130.6 L22.1 45.4 L44.1 94.1 L66.2 118.4 L88.3 75.8 L110.3 27.2 L132.4 66.7 L154.5 103.2 L176.6 124.5 L198.6 85.0 L220.7 30.2 L242.8 57.6 L264.8 97.1 L286.9 112.3 L309.0 72.8 L331.0 18.1 L353.1 54.6 L375.2 91.0 L397.2 118.4 L419.3 100.2 L441.4 60.6 L463.4 81.9 L485.5 106.2 L507.6 121.4 L529.7 88.0 L551.7 39.4 L573.8 69.8 L595.9 103.2 L617.9 115.4 L640.0 94.1 L640 164 L0 164 Z" fill="url(#fade)" />
                  <path d="M0.0 130.6 L22.1 45.4 L44.1 94.1 L66.2 118.4 L88.3 75.8 L110.3 27.2 L132.4 66.7 L154.5 103.2 L176.6 124.5 L198.6 85.0 L220.7 30.2 L242.8 57.6 L264.8 97.1 L286.9 112.3 L309.0 72.8 L331.0 18.1 L353.1 54.6 L375.2 91.0 L397.2 118.4 L419.3 100.2 L441.4 60.6 L463.4 81.9 L485.5 106.2 L507.6 121.4 L529.7 88.0 L551.7 39.4 L573.8 69.8 L595.9 103.2 L617.9 115.4 L640.0 94.1" fill="none" stroke="var(--accent)" strokeWidth="1.6" strokeLinejoin="round" />
                  <path d="M0.0 162.5 L22.1 156.4 L44.1 159.4 L66.2 161.0 L88.3 157.9 L110.3 154.9 L132.4 157.9 L154.5 161.0 L176.6 162.5 L198.6 159.4 L220.7 154.9 L242.8 156.4 L264.8 159.4 L286.9 161.0 L309.0 157.9 L331.0 154.9 L353.1 156.4 L375.2 159.4 L397.2 161.0 L419.3 159.4 L441.4 157.9 L463.4 159.4 L485.5 161.0 L507.6 161.0 L529.7 159.4 L551.7 156.4 L573.8 157.9 L595.9 161.0 L617.9 161.0 L640.0 159.4" fill="none" stroke="var(--text-3)" strokeWidth="1.2" strokeLinejoin="round" />
                  <text x="0" y="182" fontSize="11" fill="var(--ghost)" fontFamily="ui-monospace, monospace">Aug 10</text>
                  <text x="320" y="182" fontSize="11" fill="var(--ghost)" textAnchor="middle" fontFamily="ui-monospace, monospace">Aug 25</text>
                  <text x="640" y="182" fontSize="11" fill="var(--ghost)" textAnchor="end" fontFamily="ui-monospace, monospace">Sep 8</text>
                </svg>
              </div>
            </div>

            <div className="tiles">
            <div className="tile2"><div className="tile2__l">Processed tokens</div><div className="tile2__v">11.9B</div><div className="tile2__s">442M per active day</div></div>
            <div className="tile2"><div className="tile2__l">Cached input</div><div className="tile2__v">11.7B</div><div className="tile2__s">100% of observed input</div></div>
            <div className="tile2"><div className="tile2__l">Uncached input</div><div className="tile2__v">2.08M</div><div className="tile2__s">183M cache writes</div></div>
            <div className="tile2"><div className="tile2__l">Output</div><div className="tile2__v">20.6M</div><div className="tile2__s">includes 62.1K reasoning</div></div>
            <div className="tile2"><div className="tile2__l">Cache savings</div><div className="tile2__v">$52,380</div><div className="tile2__s">7.3&times; the raw token cost</div></div>
            </div>

            <div className="use__cols">
              <div>
                <div className="use__hrow"><span className="use__h">Breakdown</span>
                  <span className="seg2" style={{marginLeft: 'auto'}}><button aria-selected="true">Model</button><button aria-selected="false">Day</button></span>
                </div>
              <div className="mrow mrow--h"><span className="mrow__n" style={{color: 'inherit'}}><b style={{background: 'none'}}></b><span style={{fontFamily: 'inherit'}}>Model</span></span><span>Cost</span><span>Share</span><span>Tokens</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-opus-5</span></span><span>$6,906.33</span><span>95.8%</span><span>11.4B</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-fable-5-1</span></span><span>$126.02</span><span>1.7%</span><span>94.8M</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-sonnet-5</span></span><span>$90.55</span><span>1.3%</span><span>297M</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--text-3)'}}></b><span>gpt-5.6-sol</span></span><span>$20.86</span><span>0.3%</span><span>28.6M</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--accent)'}}></b><span>claude-haiku-4-5</span></span><span>$9.60</span><span>0.1%</span><span>23.6M</span></div>
              <div className="mrow"><span className="mrow__n"><b style={{background: 'var(--text-3)'}}></b><span>gpt-5.6-terra</span></span><span>$6.39</span><span>0.1%</span><span>16M</span></div>
                <div className="use__scan">Scanned 279 transcripts · 30,327 usage records · 0.9s</div>
              </div>

              <div>
                <div className="use__hrow"><span className="use__h">Cost quality</span></div>
                <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Provider reported</span></span><span>0.0%</span></div>
                <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Model priced</span></span><span>99.9%</span></div>
                <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Unpriced</span></span><span>0.1%</span></div>
                <div className="mrow" style={{gridTemplateColumns: '1fr 76px'}}><span className="mrow__n" style={{color: 'var(--text-3)'}}><span style={{fontFamily: 'inherit'}}>Cache savings</span></span><span>$52,380</span></div>
              </div>
            </div>
          </div>
  )
}
