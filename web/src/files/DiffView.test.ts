import { describe, expect, it } from 'vitest'
import { kindOf } from './DiffView'

/**
 * The colouring rule, called rather than restated.
 *
 * Every line of a diff is drawn from what this returns, and the one case it
 * exists to get right is invisible in a screenshot: `+++` and `---` are the
 * two header lines of every diff, and reading them as `+` and `-` paints the
 * old and new filename as one added and one removed line of code.
 */
describe('what a diff line is', () => {
  it('reads the file headers as headers, not as an add and a delete', () => {
    expect(kindOf('--- a/src/main.rs')).toBe('meta')
    expect(kindOf('+++ b/src/main.rs')).toBe('meta')
  })

  it('reads the lines that are an add and a delete', () => {
    expect(kindOf('+    let x = 1;')).toBe('added')
    expect(kindOf('-    let x = 0;')).toBe('removed')
  })

  it('names the rest of what git prints', () => {
    expect(kindOf('diff --git a/x b/x')).toBe('meta')
    expect(kindOf('index 0d41..cf31 100644')).toBe('meta')
    expect(kindOf('new file mode 100644')).toBe('meta')
    expect(kindOf('@@ -1,4 +1,6 @@ fn main() {')).toBe('hunk')
    expect(kindOf('     unchanged')).toBe('context')
    expect(kindOf('')).toBe('context')
  })

  /**
   * A line whose own content starts with the same character as its marker.
   * One character is the marker and the rest is code.
   *
   * `+++i` is the case that found the bug: an added line of C reading `++i`.
   * Matching `+++` alone drew it as a filename, which is why the header check
   * carries the space that a real `+++ b/path` always has.
   */
  it('reads only the marker, not the code behind it', () => {
    expect(kindOf('--verbose')).toBe('removed')
    expect(kindOf('+++i')).toBe('added')
    expect(kindOf('---help')).toBe('removed')
  })
})
