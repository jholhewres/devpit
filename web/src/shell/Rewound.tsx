import type { Message } from '../gen/bindings'
import { useShellPick } from './shellStore'

type RewoundPart = Extract<Message['parts'][number], { kind: 'rewound' }>

/* Where a forked conversation came from. The original is one click away and
   says it is whole, so nobody has to guess which copy still has the later
   turns. */
export function Rewound({ part }: { part: RewoundPart }): React.JSX.Element {
  const show = useShellPick((shell) => shell.show)
  return (
    <div className="rwnd">
      Went back to turn {part.turn} of{' '}
      <button className="rwnd__from" onClick={() => show('chat', { id: part.from_conversation })}>
        the conversation it came from
      </button>
      , which still has every turn it had. The next message goes on from here.
    </div>
  )
}
