import type { Checked, Validity, Verdict, Whose } from '../gen/bindings'

/*
 * What a run proves, in words.
 *
 * Pure, and apart from the component, because the sentences are the feature.
 * The backend keeps three states apart on purpose — the process, the verdict,
 * whether it still holds — and a screen that collapsed them back into one
 * colour would undo the whole of that.
 *
 * Which is also why every one of these carries a word. Colour alone cannot be
 * read by everybody looking at this, and "green" is exactly the answer this
 * feature exists to stop being given for free.
 */

/** What a verdict means, said rather than coloured. */
export function verdictWords(verdict: Verdict): { word: string; why: string } {
  switch (verdict) {
    case 'passed':
      return { word: 'Passed', why: 'A report was read and everything in it passed.' }
    case 'failed':
      return { word: 'Failed', why: 'A report was read and something in it failed.' }
    case 'notRun':
      return { word: 'Not run', why: 'This check never happened.' }
    case 'inconclusive':
      return {
        word: 'No result',
        why: 'It ran and left nothing devpit can read as a result. Exit code zero is not a pass.',
      }
  }
}

/** What a validity means. Never "wrong" — about other code is a different thing. */
export function validityWords(validity: Validity): { word: string; why: string } {
  switch (validity) {
    case 'current':
      return { word: 'Current', why: 'The commit and the uncommitted work are the ones it saw.' }
    case 'stale':
      return { word: 'Stale', why: 'The code moved since. This is about what was there then.' }
    case 'unknown':
      return {
        word: 'Unknown',
        why: 'Nothing recorded what this run saw, so nothing can say whether it still holds.',
      }
  }
}

/** Whether a run is worth reading a snapshot for, or has nothing to show. */
export const saidNothing = (checked: Checked): boolean =>
  checked.ran === null && checked.evidenceVersion === null

/**
 * What the screen may claim from a `current` verdict, said once so nothing
 * elsewhere is tempted to claim more.
 *
 * Two matching fingerprints mean git sees the same tracked state. They do not
 * mean the run was isolated from an ignored file, an installed package, an
 * environment variable or the clock.
 */
export const CURRENT_MEANS = 'the same commit and the same uncommitted work — not the same machine'

/**
 * Who asked for a run and what carried it out, in words.
 *
 * Two sentences because they are two facts: Claude can ask for a test the
 * local runner executes and Codex reviews. A screen that said "run by Claude"
 * for all three would be wrong twice.
 */
export function whoseWords(whose: Whose): { asked: string; carried: string } {
  return { asked: askedWords(whose), carried: carriedWords(whose) }
}

const ASKED: Readonly<Record<string, string>> = {
  board: 'a card moved into a lane',
  card: 'the card’s own play',
  checkpoint: 'here, from Checks',
  chain: 'the lane before it, passing the card on',
}

function askedWords(whose: Whose): string {
  const how = whose.asked === null ? null : ASKED[whose.asked]
  if (!how) return 'Nobody recorded what asked for this run.'
  /* The reference is shown and never looked up. A pane that has closed is
     unavailable — finding another that shares its name would point somebody
     at work that is not this run's. */
  return whose.askedFrom === null
    ? `Asked for by ${how}.`
    : `Asked for by ${how} (${whose.askedFrom}).`
}

const CARRIED: Readonly<Record<string, string>> = {
  process: 'a command devpit started on this machine',
  agent: 'an agent session',
}

function carriedWords(whose: Whose): string {
  const what = whose.carried === null ? null : CARRIED[whose.carried]
  if (!what) return 'Nobody recorded what did the work.'
  return whose.profile === null
    ? `Carried out by ${what}.`
    : `Carried out by ${what} under ${whose.profile}.`
}
