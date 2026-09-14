/*
 * What each language's words are.
 *
 * Apart from `languages.ts`, which says how a language is *lexed* — where its
 * comments and quotes are. Two different kinds of fact, and keeping them in
 * one file made a table nobody could scan: the shape of a language is five
 * fields and its vocabulary is a paragraph.
 *
 * Every list here is the language's own reserved words, not a guess. A word in
 * the wrong list is a word drawn the wrong colour on every line it appears —
 * which is worse than one drawn plain, so a language whose set is not known is
 * given none rather than an approximation.
 */

export const words = (list: string): ReadonlySet<string> =>
  new Set(list.split(/\s+/).filter((word) => word.length > 0))

export const RUST = words(`
as async await break const continue crate dyn else enum extern fn for if impl in let loop match
mod move mut pub ref return self Self static struct super trait type union unsafe use where while
`)

export const RUST_TYPES = words(`
bool char f32 f64 i8 i16 i32 i64 i128 isize str u8 u16 u32 u64 u128 usize String Vec Option Result
Box Rc Arc RefCell Cell Mutex HashMap HashSet BTreeMap BTreeSet Some None Ok Err true false
`)

export const TS = words(`
abstract as asserts async await break case catch class const constructor continue debugger declare
default delete do else enum export extends finally for from function get if implements import in
infer instanceof interface is keyof let module namespace new of override package private protected
public readonly require return satisfies set static super switch this throw try type typeof var
void while with yield
`)

export const TS_TYPES = words(`
any bigint boolean never null number object string symbol undefined unknown true false Array
Promise Record Map Set WeakMap WeakSet Date RegExp Error JSON Math Object Function Symbol BigInt
`)

export const PYTHON = words(`
and as assert async await break class continue def del elif else except finally for from global if
import in is lambda match nonlocal not or pass raise return try while with yield
`)

export const PYTHON_TYPES = words(`
False None True bool bytes bytearray complex dict float frozenset int list object set str tuple
type self cls
`)

export const SHELL = words(`
alias bg break builtin case cd command continue declare do done echo elif else esac eval exec exit
export fg fi for function getopts hash if in jobs kill local printf pwd read readonly return select
set shift source test then time trap type ulimit umask unalias unset until wait while
`)

export const C = words(`
alignas alignof auto bool break case char const constexpr continue default do double else enum
extern float for goto if inline int long register restrict return short signed sizeof static
struct switch typedef typeof union unsigned void volatile while
`)

export const CPP = words(`
alignas alignof and asm auto bitand bitor bool break case catch char char8_t char16_t char32_t
class co_await co_return co_yield compl concept const consteval constexpr constinit const_cast
continue decltype default delete do double dynamic_cast else enum explicit export extern false
float for friend goto if inline int long mutable namespace new noexcept not nullptr operator or
private protected public register reinterpret_cast requires return short signed sizeof static
static_assert static_cast struct switch template this thread_local throw true try typedef typeid
typename union unsigned using virtual void volatile wchar_t while xor
`)

export const JAVA = words(`
abstract assert boolean break byte case catch char class const continue default do double else
enum extends final finally float for goto if implements import instanceof int interface long
native new package private protected public record return sealed short static strictfp super
switch synchronized this throw throws transient try var void volatile while yield
`)

export const KOTLIN = words(`
abstract actual annotation as break by catch class companion const constructor continue crossinline
data delegate do dynamic else enum expect external false final finally for fun get if import in
infix init inline inner interface internal is lateinit noinline null object open operator out
override package private protected public reified return sealed set super suspend tailrec this
throw true try typealias typeof val var vararg when where while
`)

export const SWIFT = words(`
actor any as associatedtype async await break case catch class continue default defer deinit do
else enum extension fallthrough false fileprivate final for func guard if import in indirect infix
init inout internal is lazy let mutating nil nonisolated open operator optional override postfix
precedencegroup prefix private protocol public repeat rethrows return self Self set some static
struct subscript super switch throw throws true try typealias var weak where while willSet
`)

export const CSHARP = words(`
abstract as async await base bool break byte case catch char checked class const continue decimal
default delegate do double dynamic else enum event explicit extern false finally fixed float for
foreach get goto if implicit in init int interface internal is lock long namespace new null object
operator out override params private protected public readonly record ref return sbyte sealed set
short sizeof stackalloc static string struct switch this throw true try typeof uint ulong
unchecked unsafe ushort using var virtual void volatile when where while yield
`)

export const GO = words(`
break case chan const continue default defer else fallthrough for func go goto if import interface
map package range return select struct switch type var
`)

export const GO_TYPES = words(`
any bool byte complex64 complex128 error float32 float64 int int8 int16 int32 int64 rune string
uint uint8 uint16 uint32 uint64 uintptr true false nil iota make new len cap append copy delete
`)

export const RUBY = words(`
alias and begin break case class def defined? do else elsif end ensure false for if in module next
nil not or redo rescue retry return self super then true undef unless until when while yield
require require_relative attr_accessor attr_reader attr_writer include extend
`)

export const PHP = words(`
abstract and array as break callable case catch class clone const continue declare default do echo
else elseif empty enddeclare endfor endforeach endif endswitch endwhile enum extends final finally
fn for foreach function global goto if implements include include_once instanceof insteadof
interface isset list match namespace new or print private protected public readonly require
require_once return static switch throw trait try unset use var while xor yield
`)

export const SCALA = words(`
abstract case catch class def do else enum export extends false final finally for given if implicit
import lazy match new null object override package private protected return sealed super then this
throw trait true try type using val var while with yield
`)

export const HASKELL = words(`
case class data default deriving do else forall foreign if import in infix infixl infixr instance
let mdo module newtype of proc rec then type where
`)

export const ELIXIR = words(`
after and case catch cond def defdelegate defexception defguard defimpl defmacro defmodule
defprotocol defstruct do else end fn for if import in nil not or quote raise receive require
rescue try unless unquote use when with
`)

export const LUA = words(`
and break do else elseif end false for function goto if in local nil not or repeat return then
true until while self
`)

export const PERL = words(`
and cmp continue do else elsif eq eval exit for foreach ge given goto gt if last le local lt my ne
next no not or our package print printf redo ref require return say sub then unless until use when
while x xor
`)

export const R = words(`
break else for function if in next repeat return while TRUE FALSE NULL NA NaN Inf Recall
`)

export const DART = words(`
abstract as assert async await base break case catch class const continue covariant default
deferred do dynamic else enum export extends extension external factory false final finally for
get hide if implements import in interface is late library mixin new null on operator part
required rethrow return sealed set show static super switch sync this throw true try typedef var
void when while with yield
`)

export const ZIG = words(`
addrspace align allowzero and anyframe anytype asm async await break callconv catch comptime const
continue defer else enum errdefer error export extern fn for if inline linksection noalias
nosuspend noinline opaque or orelse packed pub resume return struct suspend switch test threadlocal
try union unreachable usingnamespace var volatile while
`)

export const NIM = words(`
addr and as asm bind block break case cast concept const continue converter defer discard distinct
div do elif else end enum except export finally for from func if import in include interface is
isnot iterator let macro method mixin mod nil not notin object of or out proc ptr raise ref return
shl shr static template try tuple type using var when while xor yield
`)

export const CLOJURE = words(`
def defn defmacro defmulti defmethod defprotocol defrecord deftype definterface fn let letfn if
if-let if-not when when-let when-not cond condp case do doseq dotimes loop recur try catch finally
throw ns require import use quote var set! new nil true false and or not
`)

export const ERLANG = words(`
after and andalso band begin bnot bor bsl bsr bxor case catch cond div end fun if let not of or
orelse receive rem try when xor
`)

export const OCAML = words(`
and as assert asr begin class constraint do done downto else end exception external false for fun
function functor if in include inherit initializer land lazy let lor lsl lsr lxor match method mod
module mutable new nonrec object of open or private rec sig struct then to true try type val
virtual when while with
`)

export const FSHARP = words(`
abstract and as assert base begin class default delegate do done downcast downto elif else end
exception extern false finally fixed for fun function global if in inherit inline interface
internal lazy let match member module mutable namespace new not null of open or override private
public rec return select static struct then to true try type upcast use val void when while with
yield
`)

export const JULIA = words(`
abstract baremodule begin break catch const continue do else elseif end export false finally for
function global if import let local macro module mutable primitive quote return struct true try
type using where while
`)

export const SOLIDITY = words(`
abstract address anonymous as assembly bool break bytes calldata catch constant constructor
continue contract delete do else emit enum error event external fallback false for function if
immutable import indexed interface internal is library mapping memory modifier new override
payable pragma private public pure receive return returns revert storage string struct true try
type unchecked using view virtual while
`)

export const POWERSHELL = words(`
begin break catch class continue data define do dynamicparam else elseif end enum exit filter
finally for foreach from function hidden if in inlinescript param process return static switch
throw trap try until using var while workflow
`)

export const SQL = words(`
ADD ALL ALTER AND AS ASC BEGIN BETWEEN BY CASCADE CASE CAST CHECK COLLATE COLUMN COMMIT CONFLICT
CONSTRAINT CREATE CROSS DEFAULT DELETE DESC DISTINCT DO DROP ELSE END EXCEPT EXISTS FOREIGN FROM
FULL GROUP HAVING IF IN INDEX INNER INSERT INTERSECT INTO IS JOIN KEY LEFT LIKE LIMIT NOT NULL ON
OR ORDER OUTER PRIMARY REFERENCES RENAME REPLACE RETURNING RIGHT ROLLBACK SELECT SET TABLE THEN
TRANSACTION TRIGGER UNION UNIQUE UPDATE USING VALUES VIEW WHEN WHERE WITH
`)

export const CSS = words(`
@charset @container @font-face @import @keyframes @layer @media @page @property @supports and from
important not only to
`)

export const HTML = words(`
DOCTYPE html head body div span a p h1 h2 h3 h4 h5 h6 ul ol li table tr td th form input button
label select option textarea script style link meta title img svg path header footer nav section
article aside main template slot
`)

export const NIX = words(`
assert else if in inherit let or rec then with builtins import
`)

export const TERRAFORM = words(`
count data depends_on for for_each if in locals module output provider provisioner resource
terraform variable true false null
`)

export const MAKE = words(`
all clean define else endef endif export ifdef ifeq ifndef ifneq include override phony unexport
vpath
`)

export const DOCKER = words(`
ADD ARG CMD COPY ENTRYPOINT ENV EXPOSE FROM HEALTHCHECK LABEL MAINTAINER ONBUILD RUN SHELL
STOPSIGNAL USER VOLUME WORKDIR AS
`)

export const GRAPHQL = words(`
directive enum extend false fragment implements input interface mutation null on query scalar
schema subscription true type union
`)

export const PROTOBUF = words(`
bool bytes default double enum extend extensions false fixed32 fixed64 float group import int32
int64 map message oneof option optional package public repeated required reserved returns rpc
service sfixed32 sfixed64 sint32 sint64 stream string syntax to true uint32 uint64 weak
`)

export const VIM = words(`
au augroup autocmd call command echo echom elseif else endfor endfunction endif endwhile execute
finish for function if let map nmap nnoremap noremap return set setlocal silent source unlet
vnoremap while
`)

export const GROOVY = words(`
abstract as assert boolean break byte case catch char class const continue def default do double
else enum extends false final finally float for goto if implements import in instanceof int
interface long native new null package private protected public return short static strictfp super
switch synchronized this threadsafe throw throws trait transient true try void volatile while
`)

export const TRUE_FALSE_NULL = words('true false null')
export const TRUE_FALSE = words('true false')
export const YAML_SCALARS = words('true false null yes no on off ~')
