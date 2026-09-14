import { describe, expect, it } from 'vitest'

import { KNOWN, ofName, ofPath } from './languages'
import { tokens, TOO_BIG, type Kind } from './highlight'

/* Every spelling the tables accept, so the guard walks all of them rather
   than the handful a test author happened to think of. */
const NAMED = [...KNOWN, 'typescript', 'c++', 'c#', 'golang', 'postgres', 'dockerfile', 'sh']
const PATHS = [
  'a.rs', 'a.tsx', 'a.py', 'a.go', 'a.rb', 'a.php', 'a.java', 'a.kt', 'a.swift', 'a.cs',
  'a.scala', 'a.hs', 'a.ex', 'a.lua', 'a.pl', 'a.r', 'a.dart', 'a.zig', 'a.nim', 'a.clj',
  'a.erl', 'a.ml', 'a.fs', 'a.jl', 'a.sol', 'a.ps1', 'a.gradle', 'a.sql', 'a.scss', 'a.html',
  'a.json', 'a.toml', 'a.yml', 'a.ini', 'a.nix', 'a.tf', 'a.mk', 'a.graphql', 'a.proto',
  'a.vim', 'a.c', 'a.cpp', 'a.sh', 'Makefile', 'Dockerfile', '.zshrc',
]

/** The coloured runs, as `kind:text` pairs — plain stretches left out. */
const lit = (text: string, language: string): string[] =>
  tokens(text, ofName(language))
    .filter((one) => one.kind !== 'plain')
    .map((one) => `${one.kind}:${one.text}`)

/** Everything back, in order — the invariant that matters most. */
const whole = (text: string, language: string): string =>
  tokens(text, ofName(language))
    .map((one) => one.text)
    .join('')

describe('nothing is lost or invented', () => {
  const samples: ReadonlyArray<readonly [string, string]> = [
    ['rust', 'fn main() { let x = 1; /* hi */ "s" }'],
    ['ts', 'const a = `x${y}z` // done'],
    ['python', 'def f(): return "a" # c'],
    ['json', '{"a": 1, "b": true}'],
    ['sql', 'SELECT * FROM t -- all'],
    ['rust', ''],
    ['ts', '\n\n\n'],
    ['rust', 'unterminated "string'],
    ['rust', '/* never closed'],
  ]

  it.each(samples)('%s keeps every character', (language, text) => {
    expect(whole(text, language)).toBe(text)
  })

  it('leaves text it has no language for alone', () => {
    const text = 'whatever this is'
    expect(tokens(text, null)).toEqual([{ text, kind: 'plain' }])
  })

  it('gives an empty file no tokens at all', () => {
    expect(tokens('', ofName('rust'))).toEqual([])
  })
})

describe('the traps', () => {
  it('reads a Rust lifetime as a lifetime, not an open string', () => {
    // The classic one. As a string it would paint the rest of the file.
    const said = lit("fn f<'a>(x: &'a str) -> &'a str { x }", 'rust')
    expect(said.filter((one) => one.startsWith('string:'))).toEqual([])
    expect(said).toContain('word:fn')
  })

  it('still reads a Rust character literal as a string', () => {
    expect(lit("let c = 'x';", 'rust')).toContain("string:'x'")
    expect(lit("let c = '\\n';", 'rust')).toContain("string:'\\n'")
  })

  it('nests Rust block comments and stops at the right close', () => {
    // Not nesting swallows `let after`, which is half the file below it.
    const said = lit('/* a /* b */ c */ let after = 1;', 'rust')
    expect(said[0]).toBe('comment:/* a /* b */ c */')
    expect(said).toContain('word:let')
  })

  it('does not nest them where the language does not', () => {
    const said = lit('/* a /* b */ const after = 1', 'ts')
    expect(said[0]).toBe('comment:/* a /* b */')
    expect(said).toContain('word:const')
  })

  it('ends an unterminated quote at the newline, not at the end of the file', () => {
    // Somebody halfway through typing a quote must not repaint everything
    // under it.
    const said = lit('let a = "oops\nlet b = 1', 'rust')
    expect(said).toContain('string:"oops')
    expect(said).toContain('word:let')
  })

  it('lets a backtick span lines, because that is what it is for', () => {
    const said = lit('const a = `one\ntwo`\nconst b = 1', 'ts')
    expect(said).toContain('string:`one\ntwo`')
    expect(said).toContain('word:const')
  })

  it('does not end a string on an escaped quote', () => {
    expect(lit('let a = "say \\"hi\\" now"; let b = 1', 'rust')).toContain(
      'string:"say \\"hi\\" now"',
    )
  })

  it('does not find a keyword inside a longer word', () => {
    const said = lit('let iffy = format(1)', 'rust')
    expect(said).not.toContain('word:if')
    expect(said).toContain('word:let')
  })

  it('does not find a keyword inside a string or a comment', () => {
    expect(lit('// let x', 'rust')).toEqual(['comment:// let x'])
    expect(lit('"let x"', 'rust')).toEqual(['string:"let x"'])
  })
})

describe('numbers', () => {
  it.each([
    ['1', 'number:1'],
    ['0xFF', 'number:0xFF'],
    ['1_000', 'number:1_000'],
    ['1.5', 'number:1.5'],
    ['1e-9', 'number:1e-9'],
    ['2u32', 'number:2u32'],
  ])('reads %s as one number', (text, expected) => {
    expect(lit(`let a = ${text};`, 'rust')).toContain(expected)
  })

  it('does not start a number inside an identifier', () => {
    expect(lit('let a1 = 2;', 'rust')).not.toContain('number:1')
  })
})

describe('which language', () => {
  it.each([
    ['src/main.rs', 'rust'],
    ['a/b/App.tsx', 'ts'],
    ['Cargo.toml', 'toml'],
    ['x.json', 'json'],
    ['deep/nested/file.py', 'python'],
  ])('reads %s as %s', (path, id) => {
    expect(ofPath(path)?.id).toBe(id)
  })

  it('reads a dotfile by its own name', () => {
    expect(ofPath('/home/someone/.zshrc')?.id).toBe('shell')
  })

  it('has no language for something it does not know', () => {
    expect(ofPath('notes.xyz')).toBeNull()
    expect(ofPath('LICENSE')).toBeNull()
  })

  it('takes the name a fence used', () => {
    expect(ofName('rust')?.id).toBe('rust')
    expect(ofName('TypeScript')?.id).toBe('ts')
    expect(ofName('')).toBeNull()
  })
})

describe('a file too big to colour', () => {
  it('is handed back whole and plain rather than chewed on', () => {
    // Still editable, in one colour. Which is what every editor did until
    // recently, and beats an editor that stutters under the typing.
    const huge = 'fn main() {}\n'.repeat(Math.ceil(TOO_BIG / 13) + 1)
    const said = tokens(huge, ofName('rust'))
    expect(said).toHaveLength(1)
    expect(said[0]!.kind satisfies Kind).toBe('plain')
    expect(said[0]!.text).toBe(huge)
  })

  it('colours one just under the line', () => {
    const big = 'fn a() {}\n'.repeat(100)
    expect(big.length).toBeLessThan(TOO_BIG)
    expect(tokens(big, ofName('rust')).length).toBeGreaterThan(1)
  })

  it('is set above the largest source file this repository has', () => {
    // 80KB of generated bindings. A ceiling under the files somebody actually
    // opens is a ceiling that only ever turns the colour off.
    expect(TOO_BIG).toBeGreaterThan(80_000)
  })
})

describe('the whole table holds together', () => {
  it('never points at a language nobody defined', () => {
    // `pick` throws on a dangling id, so walking every entry is the proof.
    // A table that pointed at a missing language would be a colour that
    // silently never appears, in whichever file happens to have that suffix.
    for (const name of NAMED) expect(ofName(name)).not.toBeNull()
    for (const path of PATHS) expect(ofPath(path)).not.toBeNull()
  })

  it('knows the languages this editor claims', () => {
    expect(KNOWN.length).toBeGreaterThan(40)
    expect(new Set(KNOWN).size).toBe(KNOWN.length)
  })
})

describe('the long tail of real filenames', () => {
  it.each([
    ['Makefile', 'make'],
    ['Dockerfile', 'docker'],
    ['Cargo.lock', 'toml'],
    ['Gemfile', 'ruby'],
    ['.gitconfig', 'ini'],
    ['.vimrc', 'vim'],
    ['/a/b/.prettierrc', 'json'],
    ['App.test.tsx', 'ts'],
    ['main.spec.ts', 'ts'],
    ['schema.proto', 'protobuf'],
    ['main.tf', 'terraform'],
    ['flake.nix', 'nix'],
    ['index.svelte', 'html'],
    ['build.gradle', 'groovy'],
    ['Program.cs', 'csharp'],
    ['lib.hs', 'haskell'],
    ['mix.exs', 'elixir'],
    ['Contract.sol', 'solidity'],
  ])('reads %s as %s', (path, id) => {
    expect(ofPath(path)?.id).toBe(id)
  })

  it('looks past a suffix somebody added to the end', () => {
    // The same rule that makes `App.test.tsx` TypeScript: when the last
    // extension means nothing, the one before it is the file's own. A backup
    // of a Rust file is Rust.
    expect(ofPath('main.rs.bak')?.id).toBe('rust')
    expect(ofPath('App.tsx.orig')?.id).toBe('ts')
  })

  it('still has nothing to say about a name it does not know', () => {
    expect(ofPath('notes.xyz')).toBeNull()
    expect(ofPath('LICENSE')).toBeNull()
    expect(ofPath('')).toBeNull()
  })
})

describe('comment markers that are not slashes', () => {
  it.each([
    ['python', '# a', 'comment:# a'],
    ['sql', '-- a', 'comment:-- a'],
    ['lua', '-- a', 'comment:-- a'],
    ['clojure', '; a', 'comment:; a'],
    ['erlang', '% a', 'comment:% a'],
    ['vim', '" a', 'comment:" a'],
  ])('%s uses %s', (language, text, expected) => {
    expect(lit(text, language)).toContain(expected)
  })

  it('takes either marker where a language accepts both', () => {
    expect(lit('# one', 'ini')).toEqual(['comment:# one'])
    expect(lit('; two', 'ini')).toEqual(['comment:; two'])
  })

  it('nests where the language nests and not where it does not', () => {
    expect(lit('{- a {- b -} c -} x', 'haskell')[0]).toBe('comment:{- a {- b -} c -}')
    expect(lit('(* a (* b *) c *) x', 'ocaml')[0]).toBe('comment:(* a (* b *) c *)')
    expect(lit('/* a /* b */ c */', 'java')[0]).toBe('comment:/* a /* b */')
  })
})

describe('a name is not a value', () => {
  it('draws a JSON key apart from its string value', () => {
    // The flat wash this fixes: `"name": "devpit"` was one colour, so a JSON
    // file had no shape at all.
    const said = lit('{"name": "devpit", "private": true}', 'json')
    expect(said).toContain('property:"name"')
    expect(said).toContain('string:"devpit"')
    expect(said).toContain('property:"private"')
  })

  it('finds the key across the spaces somebody left', () => {
    expect(lit('{"a"   : 1}', 'json')).toContain('property:"a"')
  })

  it('does not call a value a key just because a colon follows', () => {
    // `{"a": "b"}` — `"b"` is followed by `}`, not by a colon.
    const said = lit('{"a": "b"}', 'json')
    expect(said).toContain('string:"b"')
    expect(said).not.toContain('property:"b"')
  })

  it('takes a bare word where the language allows one', () => {
    expect(lit('name: devpit', 'yaml')).toContain('property:name')
    expect(lit('color: red;', 'css')).toContain('property:color')
  })

  it('uses the mark the language actually uses', () => {
    // TOML separates with `=`, so a colon means nothing there.
    expect(lit('name = "devpit"', 'toml')).toContain('property:name')
    expect(lit('name: "devpit"', 'toml')).not.toContain('property:name')
  })

  it('leaves TypeScript alone, where a colon is as likely to be a ternary', () => {
    // `x ? "a" : "b"` would make `"a"` a key. Not worth a wrong colour on
    // every ternary in the codebase.
    const said = lit('const x = flag ? "a" : "b"', 'ts')
    expect(said).toContain('string:"a"')
    expect(said.some((one) => one.startsWith('property:'))).toBe(false)
  })

  it('still keeps every character', () => {
    const text = '{\n  "a": 1,\n  "b": {"c": "d"}\n}\n'
    expect(whole(text, 'json')).toBe(text)
  })
})
