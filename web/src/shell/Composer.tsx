import { useImperativeHandle, useRef } from 'react'

import type { Project } from '../gen/bindings'
import { Asked } from './Asked'
import { Chips } from './Chips'
import { ready, withHeard } from './chat'
import { ChatWhere } from './ChatWhere'
import { ComposerFiles } from './ComposerFiles'
import { ComposerStatus } from './ComposerStatus'
import { ComposerTools } from './ComposerTools'
import { useDraftShortcut } from './draftShortcut'
import { JumpToEnd } from './JumpToEnd'
import { MentionMenu } from './MentionMenu'
import { SessionsWaiting } from './OrchestratorChat'
import { OwnCommand } from './OwnCommand'
import { Queued } from './Queued'
import { SecretWarning } from './SecretWarning'
import { SendButton } from './SendButton'
import { SkillPills } from './SkillPills'
import { SlashMenu } from './SlashMenu'
import { targetOf } from './stop'
import type { Tab } from './strip'
import { committed } from './typing'
import type { Chat } from './useChat'
import { useDraft } from './useDraft'
import { useMention } from './useMention'
import { useQueue } from './useQueue'
import { useSlash } from './useSlash'
import { useStop } from './useStop'

/* What the conversation above may do to what is being written. */
export type ComposerHandle = { put: (text: string) => void }

/*
 * Where a message is written. Its own component, holding the text: a key
 * pressed here draws the composer again, never the conversation above it.
 */
export function Composer({
  tab,
  chat,
  project,
  mine,
  away,
  toEnd,
  handle,
  pasted,
}: {
  tab: Tab
  chat: Chat
  project: Project | null
  mine: boolean
  away: boolean
  toEnd: () => void
  handle: React.Ref<ComposerHandle>
  pasted: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void
}): React.JSX.Element {
  /* A chat opened from a card starts with the card in the composer, unsent. */
  const [prompt, setPrompt] = useDraft(tab.id, tab.draft)
  const slash = useSlash(chat.profileId, prompt, setPrompt, Boolean(project?.orchestrator))
  const field = useRef<HTMLTextAreaElement>(null)
  const mention = useMention(project?.id ?? null, prompt, field, setPrompt)
  useImperativeHandle(handle, () => ({ put: (text) => (setPrompt(text), field.current?.focus()) }), [setPrompt])

  /* Typed while a turn runs, a message waits for it rather than being refused. */
  const queue = useQueue(chat.sending, chat.say, chat.steer)
  const sendDraft = useDraftShortcut(project?.orchestrator ?? null, setPrompt)
  const send = (): void => {
    if (!ready(prompt, false, chat.profileId)) return
    if (sendDraft(prompt) || slash.ran(prompt)) return setPrompt('')
    if (chat.sending) queue.add(prompt)
    else chat.say(prompt, (back) => setPrompt((was) => was || back))
    setPrompt('')
    toEnd()
  }

  /* A stop hands what was queued back to the composer: it was meant for a
     turn that is no longer going to happen as planned. */
  const halt = (): void => {
    const back = queue.takeAll()
    if (back) setPrompt((was) => [back, was].filter(Boolean).join('\n\n'))
    chat.stop()
  }

  /* The turn in flight is the one still streaming; arming is tied to it so a
     press that armed against a finished turn cannot stop the next one. */
  const showEsc = useStop(
    chat.sending && mine,
    targetOf(tab.id, chat.messages.find((one) => one.streaming)?.turnId ?? null),
    halt,
  )

  return (
    <div className="composer">
      <div className="composer__col">
        <JumpToEnd away={away} onJump={toEnd} />
        {/* The question sits above the composer, where your hands are — not
            in the thread, where it scrolls away from you. */}
        {/* A session stopped on the person, answered from here: the one
            thing about the others that cannot wait for a glance aside. */}
        {project?.orchestrator && <SessionsWaiting profileId={project.orchestrator} />}
        <Asked questions={chat.asked} onAnswer={chat.answer} />
        <ComposerStatus conversationId={tab.id} messages={chat.messages} />
        <SecretWarning text={prompt} />
        <Queued queue={queue} />

        <div className="composer__in">
          <ComposerFiles chat={chat} />

          <SkillPills picked={chat.skills} onChange={chat.setSkills} profile={chat.profiles.find((one) => one.id === chat.profileId)} />
          <SlashMenu slash={slash} />
          <MentionMenu menu={mention} />
          <OwnCommand slash={slash} from={tab.id} profileId={chat.profileId} orchestrator={project?.orchestrator ?? null} onClose={() => (slash.closeOwn(), field.current?.focus())} />
          <textarea
            className="composer__ph"
            placeholder="Do anything… @ for a file, / for a command"
            ref={field}
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            onPaste={pasted}
            onKeyDown={(event) => {
              /* Enter sends, Shift+Enter a new line; `committed` keeps an accent's Enter out. */
              if (slash.keyDown(event) || mention.keyDown(event)) return
              if (committed(event) && !event.shiftKey) {
                event.preventDefault()
                send()
              }
            }}
          />

          <div className="composer__row">
            {/* A control takes focus and gives it back: the cursor was mid-sentence. Cancelling
                the press instead would be cheaper and swallow the click that opens the menu. */}
            <Chips chat={chat} refocus={() => field.current?.focus()} />
            <ComposerTools chat={chat} onHeard={(text) => (setPrompt((was) => withHeard(was, text)), field.current?.focus())} />
            <SendButton sending={chat.sending} showEsc={showEsc} can={ready(prompt, chat.sending, chat.profileId)} onSend={send} onStop={halt} />
          </div>
        </div>

        <ChatWhere />
      </div>
    </div>
  )
}
