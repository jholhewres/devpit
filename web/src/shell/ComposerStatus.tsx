import { useEffect, useMemo, useRef, useState } from 'react'

import type { Message, Part } from '../gen/bindings'
import { elapsed, running, tasksOf } from './backgroundTasks'
import { checklistOf, done } from './checklist'
import { useMessagesPick, type ChatMessages } from './chatMessages'
import { ask, commands } from './live'

/*
 * What the turn has going on, above the composer where the hands are: work it
 * left running in the background, and the checklist it is working through.
 *
 * Both read the newest answer only. A checklist from three turns ago is done
 * or abandoned, and either way it is not what the agent is doing now.
 */

const NONE: readonly Part[] = []

/* The newest answer's parts other than its text, item by item the same while only its text grows. */
const workOf = (all: readonly Message[]): readonly Part[] =>
  all.findLast((one) => one.role === 'assistant')?.parts.filter((part) => part.kind !== 'text') ?? NONE

export function ComposerStatus({
  conversationId,
  messages,
}: {
  conversationId: string
  messages: ChatMessages
}): React.JSX.Element | null {
  /* Read from the end, and only when the work changes: the clock below ticks
     every second and must not walk the thread, nor a streamed batch draw it. */
  const work = useMessagesPick(messages, workOf)
  const { tasks, list } = useMemo(() => ({ tasks: tasksOf(work).filter(running), list: checklistOf(work) }), [work])

  /* When each task was first seen here. The CLI sends no start time, and
     "running for" is the question a person glancing at the strip is asking. */
  const since = useRef(new Map<string, number>())
  const [now, setNow] = useState(() => Date.now())
  for (const task of tasks) if (!since.current.has(task.id)) since.current.set(task.id, Date.now())
  useEffect(() => {
    if (tasks.length === 0) return
    /* A window nobody is looking at does not need its clock ticking. */
    const timer = window.setInterval(() => {
      if (!document.hidden) setNow(Date.now())
    }, 1000)
    return () => window.clearInterval(timer)
  }, [tasks.length])

  if (tasks.length === 0 && list.items.length === 0) return null

  return (
    <div className="bgt">
      {tasks.map((task) => (
        <div className="bgt__row" key={task.id}>
          <span className="arow__live" aria-label="Running" />
          <span className="bgt__k">{task.kind === 'local_agent' ? 'Subagent' : 'Command'}</span>
          <span className="bgt__d" title={task.description}>
            {task.description}
          </span>
          <span className="bgt__t">{elapsed(since.current.get(task.id) ?? now, now)}</span>
          <button
            className="chip"
            aria-label={`Stop ${task.description}`}
            onClick={() => void ask(() => commands.chatStopTask(conversationId, task.id))}
          >
            Stop
          </button>
        </div>
      ))}

      {list.items.length > 0 && (
        <details className="ckl">
          <summary className="ckl__h">
            {list.items.filter(done).length} of {list.items.length} done
          </summary>
          {list.items.map((item) => (
            <div className="ckl__i" key={item.id} data-done={done(item) || undefined} data-changed={list.changed.includes(item.id) || undefined}>
              <span className="ckl__box" aria-hidden="true">{done(item) ? '✓' : ''}</span>
              {item.subject}
            </div>
          ))}
        </details>
      )}
    </div>
  )
}
