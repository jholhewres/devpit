import * as vocabulary from './vocabulary'

/*
 * The languages the editor can colour, as data.
 *
 * A table rather than a function per language: what differs between them at
 * this level is punctuation and vocabulary, not structure. Everything here
 * shares one tokeniser, and a language that needed its own would be a language
 * this file should not pretend to know.
 *
 * This is a **lexer's** view and it does not apologise for that. It finds
 * comments, strings, numbers and words. It does not know that `foo` is a
 * function or that `Bar` is in scope, because knowing that is a compiler and
 * the difference on screen is a shade of blue.
 *
 * Where the words live is `vocabulary.ts`. Two files because they are two
 * kinds of fact: the shape of a language is five fields, its vocabulary is a
 * paragraph, and together they were a table nobody could scan.
 */

export interface Language {
  readonly id: string
  /** What starts a comment that runs to the end of the line. */
  readonly line?: string
  /** A second one, for languages that accept both. */
  readonly alsoLine?: string
  /** What opens and closes a comment that spans lines. */
  readonly block?: readonly [string, string]
  /** Rust's block comments nest; C's do not, and getting this wrong swallows
   *  half a file. */
  readonly nests?: boolean
  readonly quotes: readonly string[]
  /** A `'` that is a lifetime rather than a character literal — Rust, and the
   *  handful of others that borrowed the idea. */
  readonly lifetimes?: boolean
  /* What comes after a name to mark it as one. Only set where it cannot be
     anything else: `:` after a quoted string is a key in JSON and YAML, and in
     TypeScript it is as likely to be the middle of a ternary. */
  readonly keys?: string
  /** Whether a bare word can be a key too, as in YAML and CSS. */
  readonly bareKeys?: boolean
  readonly words: ReadonlySet<string>
  /** Words drawn as types rather than as keywords. A language whose types are
   *  not a closed set has none, and that is honest. */
  readonly types?: ReadonlySet<string>
}

/** The shapes languages share, so a new one is a line and not a paragraph. */
const C_LIKE = { line: '//', block: ['/*', '*/'] as const, quotes: ['"', "'"] }
const HASH = { line: '#', quotes: ['"', "'"] }

const LANGUAGES: readonly Language[] = [
  { id: 'rust', ...C_LIKE, nests: true, quotes: ['"'], lifetimes: true,
    words: vocabulary.RUST, types: vocabulary.RUST_TYPES },
  { id: 'ts', ...C_LIKE, quotes: ['"', "'", '`'],
    words: vocabulary.TS, types: vocabulary.TS_TYPES },
  { id: 'python', ...HASH, words: vocabulary.PYTHON, types: vocabulary.PYTHON_TYPES },
  { id: 'shell', ...HASH, words: vocabulary.SHELL },
  { id: 'c', ...C_LIKE, words: vocabulary.C },
  { id: 'cpp', ...C_LIKE, words: vocabulary.CPP },
  { id: 'java', ...C_LIKE, words: vocabulary.JAVA },
  { id: 'kotlin', ...C_LIKE, words: vocabulary.KOTLIN },
  { id: 'swift', ...C_LIKE, words: vocabulary.SWIFT },
  { id: 'csharp', ...C_LIKE, words: vocabulary.CSHARP },
  { id: 'go', ...C_LIKE, quotes: ['"', '`', "'"],
    words: vocabulary.GO, types: vocabulary.GO_TYPES },
  { id: 'ruby', ...HASH, words: vocabulary.RUBY },
  { id: 'php', ...C_LIKE, alsoLine: '#', words: vocabulary.PHP },
  { id: 'scala', ...C_LIKE, words: vocabulary.SCALA },
  { id: 'haskell', line: '--', block: ['{-', '-}'], nests: true, quotes: ['"', "'"],
    words: vocabulary.HASKELL },
  { id: 'elixir', ...HASH, words: vocabulary.ELIXIR },
  { id: 'lua', line: '--', block: ['--[[', ']]'], quotes: ['"', "'"],
    words: vocabulary.LUA },
  { id: 'perl', ...HASH, words: vocabulary.PERL },
  { id: 'r', ...HASH, words: vocabulary.R },
  { id: 'dart', ...C_LIKE, words: vocabulary.DART },
  { id: 'zig', line: '//', quotes: ['"', "'"], words: vocabulary.ZIG },
  { id: 'nim', ...HASH, words: vocabulary.NIM },
  { id: 'clojure', line: ';', quotes: ['"'], words: vocabulary.CLOJURE },
  { id: 'erlang', line: '%', quotes: ['"', "'"], words: vocabulary.ERLANG },
  { id: 'ocaml', block: ['(*', '*)'], nests: true, quotes: ['"'], words: vocabulary.OCAML },
  { id: 'fsharp', line: '//', block: ['(*', '*)'], quotes: ['"'], words: vocabulary.FSHARP },
  { id: 'julia', ...HASH, block: ['#=', '=#'], nests: true, words: vocabulary.JULIA },
  { id: 'solidity', ...C_LIKE, words: vocabulary.SOLIDITY },
  { id: 'powershell', ...HASH, block: ['<#', '#>'], words: vocabulary.POWERSHELL },
  { id: 'groovy', ...C_LIKE, words: vocabulary.GROOVY },
  { id: 'sql', line: '--', block: ['/*', '*/'], quotes: ["'", '"'], words: vocabulary.SQL },
  { id: 'css', block: ['/*', '*/'], quotes: ['"', "'"], keys: ':', bareKeys: true,
    words: vocabulary.CSS },
  { id: 'html', block: ['<!--', '-->'], quotes: ['"', "'"], words: vocabulary.HTML },
  { id: 'json', quotes: ['"'], keys: ':',
    words: new Set<string>(), types: vocabulary.TRUE_FALSE_NULL },
  { id: 'toml', ...HASH, keys: '=', bareKeys: true,
    words: new Set<string>(), types: vocabulary.TRUE_FALSE },
  { id: 'yaml', ...HASH, keys: ':', bareKeys: true,
    words: new Set<string>(), types: vocabulary.YAML_SCALARS },
  { id: 'ini', ...HASH, alsoLine: ';', keys: '=', bareKeys: true,
    words: new Set<string>(), types: vocabulary.TRUE_FALSE },
  { id: 'nix', ...HASH, block: ['/*', '*/'], words: vocabulary.NIX },
  { id: 'terraform', ...HASH, block: ['/*', '*/'], words: vocabulary.TERRAFORM },
  { id: 'make', ...HASH, words: vocabulary.MAKE },
  { id: 'docker', ...HASH, words: vocabulary.DOCKER },
  { id: 'graphql', ...HASH, keys: ':', bareKeys: true, words: vocabulary.GRAPHQL },
  { id: 'protobuf', ...C_LIKE, words: vocabulary.PROTOBUF },
  { id: 'vim', line: '"', quotes: ["'"], words: vocabulary.VIM },
]

const BY_ID: ReadonlyMap<string, Language> = new Map(LANGUAGES.map((one) => [one.id, one]))

const pick = (id: string): Language => {
  const found = BY_ID.get(id)
  /* A table pointing at a language nobody defined is a colour that silently
     never appears. Caught here, at module load, rather than in a file. */
  if (!found) throw new Error(`no language called ${id}`)
  return found
}

/*
 * Extension to language.
 *
 * The backend has already decided the file is text at all — by its first
 * bytes, not by its name — so a wrong extension here costs a colour and never
 * a pane full of mojibake.
 */
const BY_EXTENSION: Readonly<Record<string, string>> = {
  rs: 'rust',
  ts: 'ts', tsx: 'ts', mts: 'ts', cts: 'ts',
  js: 'ts', jsx: 'ts', mjs: 'ts', cjs: 'ts',
  py: 'python', pyi: 'python', pyw: 'python',
  sh: 'shell', bash: 'shell', zsh: 'shell', fish: 'shell', ksh: 'shell', zshrc: 'shell',
  bashrc: 'shell', bash_profile: 'shell', zprofile: 'shell', profile: 'shell', env: 'shell',
  c: 'c', h: 'c',
  cpp: 'cpp', cc: 'cpp', cxx: 'cpp', hpp: 'cpp', hh: 'cpp', hxx: 'cpp', ino: 'cpp',
  m: 'c', mm: 'cpp',
  java: 'java',
  kt: 'kotlin', kts: 'kotlin',
  swift: 'swift',
  cs: 'csharp', csx: 'csharp',
  go: 'go',
  rb: 'ruby', rake: 'ruby', gemspec: 'ruby',
  php: 'php', phtml: 'php',
  scala: 'scala', sbt: 'scala', sc: 'scala',
  hs: 'haskell', lhs: 'haskell',
  ex: 'elixir', exs: 'elixir',
  lua: 'lua',
  pl: 'perl', pm: 'perl', t: 'perl',
  r: 'r', rmd: 'r',
  dart: 'dart',
  zig: 'zig',
  nim: 'nim', nims: 'nim',
  clj: 'clojure', cljs: 'clojure', cljc: 'clojure', edn: 'clojure',
  erl: 'erlang', hrl: 'erlang',
  ml: 'ocaml', mli: 'ocaml',
  fs: 'fsharp', fsi: 'fsharp', fsx: 'fsharp',
  jl: 'julia',
  sol: 'solidity',
  ps1: 'powershell', psm1: 'powershell', psd1: 'powershell',
  groovy: 'groovy', gradle: 'groovy',
  sql: 'sql',
  css: 'css', scss: 'css', sass: 'css', less: 'css',
  html: 'html', htm: 'html', xhtml: 'html', xml: 'html', svg: 'html', vue: 'html',
  svelte: 'html', astro: 'html', xsl: 'html', plist: 'html', rss: 'html',
  json: 'json', jsonc: 'json', json5: 'json', webmanifest: 'json', lock: 'json',
  toml: 'toml',
  yaml: 'yaml', yml: 'yaml',
  ini: 'ini', cfg: 'ini', conf: 'ini', properties: 'ini', editorconfig: 'ini', gitconfig: 'ini',
  nix: 'nix',
  tf: 'terraform', tfvars: 'terraform', hcl: 'terraform',
  mk: 'make', mak: 'make',
  graphql: 'graphql', gql: 'graphql',
  proto: 'protobuf',
  vim: 'vim', vimrc: 'vim',
}

/*
 * Whole names, for the files that have no extension at all.
 *
 * `Makefile` and `Dockerfile` are the obvious ones, and a dotfile is its own
 * name: `.zshrc` is not a `zshrc` file, it is *the* zshrc.
 */
const BY_NAME: Readonly<Record<string, string>> = {
  makefile: 'make', gnumakefile: 'make', justfile: 'make',
  dockerfile: 'docker', containerfile: 'docker',
  'cargo.lock': 'toml', 'gemfile.lock': 'toml',
  gemfile: 'ruby', rakefile: 'ruby', podfile: 'ruby', brewfile: 'ruby',
  '.zshrc': 'shell', '.bashrc': 'shell', '.bash_profile': 'shell', '.zshenv': 'shell',
  '.zprofile': 'shell', '.profile': 'shell', '.env': 'shell', '.inputrc': 'shell',
  '.gitignore': 'shell', '.dockerignore': 'shell', '.npmrc': 'ini', '.editorconfig': 'ini',
  '.gitconfig': 'ini', '.gitattributes': 'shell', '.vimrc': 'vim', '.eslintrc': 'json',
  '.prettierrc': 'json', '.babelrc': 'json',
}

/** What a fenced code block may call a language, beyond its extensions. */
const ALIASES: Readonly<Record<string, string>> = {
  typescript: 'ts', javascript: 'ts', node: 'ts', typescriptreact: 'ts', javascriptreact: 'ts',
  console: 'shell', sh: 'shell', shellsession: 'shell', bat: 'shell', cmd: 'shell',
  'c++': 'cpp', objectivec: 'c', 'objective-c': 'c', csharp: 'csharp', 'c#': 'csharp',
  golang: 'go', rust: 'rust', python3: 'python', py3: 'python', elisp: 'clojure',
  postgres: 'sql', postgresql: 'sql', mysql: 'sql', sqlite: 'sql', plpgsql: 'sql',
  'f#': 'fsharp', ocaml: 'ocaml', dockerfile: 'docker', makefile: 'make',
  hcl: 'terraform', tf: 'terraform', proto: 'protobuf', gql: 'graphql',
}

/** The language of a path, or nothing when it is not one we know. */
export function ofPath(path: string): Language | null {
  const name = (path.split(/[\\/]/).pop() ?? '').toLowerCase()
  const whole = BY_NAME[name]
  if (whole) return pick(whole)

  // The last extension, and then the one before it: `schema.sql.gz` is not
  // SQL, but `App.test.tsx` is TypeScript and `main.spec.ts` is too.
  const parts = name.split('.')
  for (const part of [parts.at(-1), parts.at(-2)]) {
    const found = part && BY_EXTENSION[part]
    if (found) return pick(found)
  }
  return null
}

/** The language a fence named, or nothing. */
export function ofName(name: string): Language | null {
  const wanted = name.trim().toLowerCase()
  const found = ALIASES[wanted] ?? BY_EXTENSION[wanted] ?? (BY_ID.has(wanted) ? wanted : null)
  return found ? pick(found) : null
}

/** Every language this build can colour, for a test that asks. */
export const KNOWN: readonly string[] = LANGUAGES.map((one) => one.id)
