/*
 * The glyph an agent wears.
 *
 * Keyed by the id the backend sends, which is the id the launch menu uses, so
 * a mark can only ever appear for an agent this build can also start. An agent
 * with no drawing of its own gets the generic one rather than nothing: a pane
 * running something we recognise has to look different from a pane running a
 * shell, even when we have not drawn its logo.
 *
 * Colour lives in CSS, on `[data-agent]`, so a theme can move them together.
 */

const MARKS: Readonly<Record<string, React.JSX.Element>> = {
  /* The asterisk. Six spokes, which is the shape people recognise. */
  claude: (
    <path d="M12 3.5v17M4.6 7.75l14.8 8.5M4.6 16.25l14.8-8.5" />
  ),
  codex: (
    <>
      <circle cx="12" cy="12" r="8.2" />
      <path d="M9.4 9.4 12 12l-2.6 2.6M13.4 14.8h3.4" />
    </>
  ),
  gemini: (
    <path d="M12 3c0 4.97 4.03 9 9 9-4.97 0-9 4.03-9 9 0-4.97-4.03-9-9-9 4.97 0 9-4.03 9-9Z" />
  ),
  opencode: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2.4" />
      <path d="m8.4 10.2-2 2 2 2M13 9.6l-2 4.8" />
    </>
  ),
  cursor: (
    <path d="m5 3 14 8.2-6.1 1.6-2.2 6.2Z" />
  ),
  copilot: (
    <>
      <path d="M4 13.5c0-3 3.6-5 8-5s8 2 8 5-3.6 5.5-8 5.5-8-2.5-8-5.5Z" />
      <path d="M9.5 13v1.6M14.5 13v1.6" />
    </>
  ),
  amp: (
    <path d="M13.6 3 5 13.6h5.4L9.6 21l8.8-10.8h-5.5Z" />
  ),
  droid: (
    <>
      <rect x="4.5" y="7.5" width="15" height="11" rx="3" />
      <path d="M9.5 12v1.6M14.5 12v1.6M12 4v3.5" />
    </>
  ),
  grok: (
    <path d="M4.5 19.5 19 5M10 19.5 19.5 10M19.5 15.5v4h-4" />
  ),
  aider: (
    <>
      <path d="M12 3.6 20 8v8l-8 4.4L4 16V8Z" />
      <path d="M12 9.2 15.6 11v3.4L12 16.2 8.4 14.4V11Z" />
    </>
  ),
  goose: (
    <path d="M17.5 4.5c-5 0-9 3.4-9 8v3.5H5.5l3 4h6c3 0 5-2.4 5-5.5 0-2-1-3-2.5-3.5 1-1.5 1.5-3.6.5-6.5Z" />
  ),
  crush: (
    <>
      <path d="M12 3.5 14.4 9l5.6.6-4.2 3.9 1.2 5.6L12 16.3 6.999 19.1l1.2-5.6L4 9.6 9.6 9Z" />
    </>
  ),
}

/* Anything recognised but undrawn. A wrench would say "a tool"; this says "a
   conversation", which is what an agent in a pane is. */
const GENERIC = (
  <>
    <path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" />
    <path d="M8.4 11.5h.01M12 11.5h.01M15.6 11.5h.01" />
  </>
)

export function AgentMark({
  agent,
  size = 14,
}: {
  agent: string
  size?: number
}): React.JSX.Element {
  return (
    <svg
      className="agmk"
      data-agent={agent}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {MARKS[agent] ?? GENERIC}
    </svg>
  )
}
