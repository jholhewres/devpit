/**
 * The icon set, inline.
 *
 * Drawn on a 16px box with a 1.25 stroke so a glyph sits on the same optical
 * weight as the 12–13px text beside it. No icon font and no emoji: an emoji
 * renders differently on each platform and carries a tone this app does not
 * have.
 */

type GlyphProps = { size?: number }

function Svg({ size = 14, children }: GlyphProps & { children: React.ReactNode }): React.JSX.Element {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.25"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  )
}

export function OverviewGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <rect x="2" y="2.5" width="5" height="5" rx="1" />
      <rect x="9" y="2.5" width="5" height="3" rx="1" />
      <rect x="2" y="9.5" width="5" height="4" rx="1" />
      <rect x="9" y="7.5" width="5" height="6" rx="1" />
    </Svg>
  )
}

export function CanvasGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <rect x="2" y="2.5" width="12" height="11" rx="1.5" />
      <path d="M2 10.5l3.2-3 2.6 2.4 2.6-3.4L14 10" />
    </Svg>
  )
}

export function NotesGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M3.5 2.5h9v11h-9z" />
      <path d="M5.75 5.75h4.5M5.75 8.25h4.5M5.75 10.75h2.75" />
    </Svg>
  )
}

export function WikiGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M2.5 3.25c2.2-.9 3.7-.9 5.5.25 1.8-1.15 3.3-1.15 5.5-.25v9.5c-2.2-.9-3.7-.9-5.5.25-1.8-1.15-3.3-1.15-5.5-.25z" />
      <path d="M8 3.5v9.25" />
    </Svg>
  )
}

export function DiagnosticsGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M1.75 8h3l1.5-4 2.5 8 1.5-4h3.5" />
    </Svg>
  )
}

export function SettingsGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M2 4.5h12M2 11.5h12" />
      <circle cx="6" cy="4.5" r="1.75" />
      <circle cx="10.5" cy="11.5" r="1.75" />
    </Svg>
  )
}

export function PlusGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M8 3v10M3 8h10" />
    </Svg>
  )
}

export function CloseGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Svg>
  )
}

export function ChevronGlyph({ open = false, ...props }: GlyphProps & { open?: boolean }): React.JSX.Element {
  return (
    <Svg {...props} size={props.size ?? 12}>
      {open ? <path d="M4 6l4 4 4-4" /> : <path d="M6 4l4 4-4 4" />}
    </Svg>
  )
}

export function PanelGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <path d="M10 3v10" />
    </Svg>
  )
}

export function FolderGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <path d="M2 4.5a1 1 0 0 1 1-1h3.2l1.3 1.5H13a1 1 0 0 1 1 1v5.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z" />
    </Svg>
  )
}

export function SidebarGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <path d="M6 3v10" />
    </Svg>
  )
}

export function TerminalGlyph(props: GlyphProps): React.JSX.Element {
  return (
    <Svg {...props}>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <path d="M5 6.75L7 8.5 5 10.25M8.75 10.5h2.5" />
    </Svg>
  )
}
