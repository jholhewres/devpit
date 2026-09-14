import { describe, expect, it } from 'vitest'

import { parse, sides, visible } from './diff'

const DIFF = `diff --git a/src/a.rs b/src/a.rs
index 111..222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,3 @@
 fn main() {
-    println!("one");
+    println!("two");
 }
`

describe('a unified diff', () => {
  it('reads the file and its hunks', () => {
    const [file] = parse(DIFF)
    expect(file.path).toBe('src/a.rs')
    expect(file.hunks).toHaveLength(1)
    expect(file.hunks[0].header).toBe('@@ -1,3 +1,3 @@')
  })

  it('reads each line as what it is', () => {
    const rows = parse(DIFF)[0].hunks[0].rows
    expect(rows.map((row) => row.kind)).toEqual(['same', 'del', 'add', 'same'])
    expect(rows[2].text).toBe('    println!("two");')
  })

  /* The prefix lies: a line that adds `++foo` starts with two plus signs, and
     reading two plus signs as a header draws the addition as one. */
  it('reads a line that adds ++ as an addition', () => {
    const odd = `diff --git a/x b/x
@@ -1 +1 @@
+++ still an addition
`
    const rows = parse(odd)[0].hunks[0].rows
    expect(rows).toEqual([{ kind: 'add', text: '++ still an addition' }])
  })

  /* The `+++ b/x` line sits outside every hunk, so it is never a row. */
  it('does not read the file header as a line of the diff', () => {
    const rows = parse(DIFF)[0].hunks[0].rows
    expect(rows.some((row) => row.text.startsWith('b/src'))).toBe(false)
  })

  it('reads a rename as a rename, not as a delete plus an add', () => {
    const moved = `diff --git a/old.rs b/new.rs
similarity index 100%
rename from old.rs
rename to new.rs
`
    const [file] = parse(moved)
    expect(file.path).toBe('new.rs')
    expect(file.from).toBe('old.rs')
    expect(file.hunks).toHaveLength(0)
  })

  it('says when a file is binary rather than showing nothing', () => {
    const image = `diff --git a/logo.png b/logo.png
Binary files a/logo.png and b/logo.png differ
`
    expect(parse(image)[0].binary).toBe(true)
  })

  it('reads several files in one diff', () => {
    expect(parse(`${DIFF}${DIFF.replace(/a\.rs/g, 'b.rs')}`)).toHaveLength(2)
  })
})

describe('the side-by-side view', () => {
  it('lines a deletion up with the addition that replaced it', () => {
    const { left, right } = sides(parse(DIFF)[0].hunks[0])
    expect(left.map((row) => row?.kind)).toEqual(['same', 'del', 'same'])
    expect(right.map((row) => row?.kind)).toEqual(['same', 'add', 'same'])
  })

  it('pads the shorter side so the rows stay level', () => {
    const uneven = `diff --git a/x b/x
@@ -1,2 +1,1 @@
-one
-two
+only
`
    const { left, right } = sides(parse(uneven)[0].hunks[0])
    expect(left).toHaveLength(2)
    expect(right).toHaveLength(2)
    expect(right[1]).toBeNull()
  })
})

describe('whitespace, when asked for', () => {
  it('marks spaces and tabs and leaves the words alone', () => {
    expect(visible('  if\tx')).toEqual([
      { text: '··', space: true },
      { text: 'if', space: false },
      { text: '→   ', space: true },
      { text: 'x', space: false },
    ])
  })

  it('shows trailing whitespace, which is usually why someone looked', () => {
    expect(visible('done  ').at(-1)).toEqual({ text: '··', space: true })
  })

  it('is nothing for an empty line', () => {
    expect(visible('')).toEqual([])
  })
})
