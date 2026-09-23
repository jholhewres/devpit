import { Menu, MenuItem, MenuRule } from './Menu'

/*
 * A small right-click menu for the rail: its projects and its groups. The
 * app's one menu (`Menu`), so it has the keyboard, stays inside the window
 * and closes the moment another opens.
 */

export type RailItem = { label: string; glyph: React.ReactNode; act: () => void; bad?: boolean } | 'rule'

export function RailMenu({
  at,
  items,
  label = 'Project actions',
  onClose,
}: {
  at: { x: number; y: number }
  items: readonly RailItem[]
  /** What a screen reader calls the menu. */
  label?: string
  onClose: () => void
}): React.JSX.Element {
  return (
    <Menu at={at} label={label} onClose={onClose}>
      {items.map((item, index) =>
        item === 'rule' ? (
          <MenuRule key={index} />
        ) : (
          <MenuItem
            key={item.label}
            label={item.label}
            glyph={item.glyph}
            bad={item.bad}
            onPick={() => {
              onClose()
              item.act()
            }}
          />
        ),
      )}
    </Menu>
  )
}
