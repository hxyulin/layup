# Language tools

The same language tools run in Rust, the CLI, Node and browser WebAssembly.
Rendering requires valid input. Linting collects independent syntax errors;
formatting operates on valid syntax without resolving styles or laying out
a diagram. Version 0.3 includes these language tools.

## CLI

```sh
layup lint diagram.layup             # diagnostics with source excerpts
layup lint diagram.layup --strict    # warnings also fail
layup lint guide.md --json           # diagnostics for every layup fence
layup lint - < diagram.layup         # DSL from stdin
layup fmt diagram.layup              # formatted source to stdout
layup fmt < diagram.layup            # stdin to stdout
layup fmt examples/*.layup --check    # fail on formatting differences
layup fmt examples/*.layup --write    # replace files after syntax preflight
```

`lint` exits 1 for errors, or for warnings with `--strict`. Human diagnostics
go to stderr. `--json` writes one array to stdout, including `[]` when clean.
Read failures go to stderr and fail the command. `check` retains its existing
compile/layout checks; `lint` adds authoring rules and syntax recovery. Both
accept global `--font path.ttf` options for supplied-font measurement.

`fmt` uses two-space indentation, spaces between tokens, no spaces around
`=` or `:`, and newlines instead of statement semicolons. It retains at most
one authored blank line and terminates nonempty output with a newline.
Comments stay in order; raw quoted strings, multiline contents, escapes,
numeric spellings and statement order are preserved. Formatting is idempotent.

Default output never modifies a file. Multiple inputs require `--check` or
`--write`. `--check` never writes and exits 1 for differences or syntax errors.
`--write` refuses stdin and validates every input before replacing any file:
a syntax or read error leaves every input untouched. Formatting accepts DSL
files, rather than rewriting Markdown fences. Semantic errors do not prevent
syntax formatting.

## Diagnostics and source positions

An unknown attribute can produce:

```text
diagram.layup:3:15: error[semantic]: unknown node attribute `aling`
 3 |   node worker aling=left
   |               ^^^^^
  help: did you mean `align`?
```

Diagnostics contain `line`, `column`, `severity`, `code`, `message`, an
optional `span`, optional `help`, and `related` locations. Duplicate attributes
point at both declarations; unclosed blocks point at the opening brace.
Suggestions use known node IDs, node kinds and supported diagram/node
attribute names. Older semantic and layout checks that report only a line
use a matching named token or the first token on that line as their range;
they do not all identify an exact attribute value yet.

Spans have `{ start, end, line, column, endLine, endColumn }`. Byte offsets are
zero-based UTF-8, with inclusive start and exclusive end. Lines and columns
are one-based; columns count Unicode scalar values. Tabs count as one scalar
column. Terminal excerpts use four-column tab stops and account for wide
characters and combining marks. Editors using UTF-16 positions must convert
the ranges. End-of-input ranges may be empty.

CLI JSON records wrap each diagnostic:

```json
{ "file": "diagram.layup", "fenceLine": null, "diagnostic": {
  "line": 3, "column": 15, "severity": "error", "code": "semantic",
  "message": "unknown node attribute `aling`",
  "span": { "start": 48, "end": 53, "line": 3, "column": 15, "endLine": 3, "endColumn": 20 },
  "help": "did you mean `align`?", "related": []
} }
```

Those byte offsets are illustrative. For Markdown, `fenceLine` is the
one-based opening fence line. All primary and related ranges refer to the
original Markdown file, including its UTF-8 bytes and CRLF line endings.
Non-CLI APIs report positions relative to the source string they receive.

## Lint rules and recovery

| Code | Reported condition |
| --- | --- |
| `lex/*`, `parse/*` | Invalid characters, strings, numbers, arrows, attributes, statements or blocks |
| `semantic`, `semantic/*` | Invalid model, references, duplicate IDs, geometry numbers or machine semantics |
| `layout` | Existing measurement, placement, routing, reachability or glyph warnings |
| `lint/unused-style` | Unused custom style; referenced base styles count as used |
| `lint/unused-arrow` | Unused arrow kind |
| `lint/overridden-flag` | Repeated/conflicting bare fill, font, alignment, stroke or tone flags |
| `lint/ignored-body` | A body attached to a directive that ignores its contents |
| `lint/duplicate-transition` | Identical endpoints, kind, label, styling and routing attributes |

Distinct labels or routing attributes distinguish transitions. Authoring
rules produce warnings without changing rendering. Automatic fixes are not
implemented.

Recovery stops at sibling statement boundaries while respecting braces.
Unterminated multiline strings consume the remaining source. Blocks may
nest at most 128 levels. If syntax errors exist, linting returns them without
semantic/layout checks on a partial document. With valid syntax, compilation
still stops at its first semantic error. Successful compilation allows lint
to collect layout and authoring warnings.

## Rust and JavaScript

```rust
let formatted = layup::format::format(source)?;
let diagnostics = layup::lint::lint(source);
let partial = layup::parser::parse_recovering(source);
// partial.statements and partial.errors are available for tooling.
```

`lexer::lex_lossless` retains comments. `lexer::lex_recovering` returns tokens
and lexical errors; `parser::parse_recovering` returns a partial generic
syntax tree and errors. Tokens, items, arguments and edge endpoints have
source spans. Strict `lex`, `parse` and `compile` remain `Result` APIs.
`lint_with_fonts(source, &fonts)` uses supplied fallback faces.

`Error` retains `line`, `msg` and its display format and adds
`span: Option<Box<Span>>`, `code`, `help` and `related`. Convert it with
`Diagnostic::from_error(&error)` for rich `display(source, filename, offset)`
or JSON output. Added fields affect downstream Rust struct literals and
exhaustive patterns for errors, tokens and syntax nodes.

```js
import { load } from '@hxyulin/layup';
const layup = await load();
const formatted = layup.format(source);         // bad syntax throws LayupError
const diagnostics = layup.lint(source);        // invalid DSL returns diagnostics
const measured = layup.lint(source, { fonts }); // same fallback policy as render
```

`LayupError` retains `line`, `reason` and its message format and adds `column`,
`span`, `code`, `help` and `related`. Invalid options/fonts can still throw
from `lint`. Render warnings retain `{ line, message }`; lint diagnostics are
richer. Markdown render errors include source columns and suggestion help.

## Syntax compatibility

Unicode identifiers, scoped names, paths, URLs, comments, multiline strings,
existing node kinds and edge forms continue to work. Typed arrow names may
contain hyphens (`a -data-flow-> b`); compact untyped arrows (`a->b`, `a<-b`)
work. Numbers accept fractions and exponents such as `.5` and `1e3`.

Invalid syntax is deliberately stricter: duplicate attributes are rejected,
unknown string escapes are errors rather than silently losing a backslash,
and numeric literals must be finite. Supported escapes are `\"`, `\\`,
`\n`, `\t` and `\r`; literal Windows paths need escaped backslashes.
Geometry validation rejects nonpositive canvas widths and row weights,
negative gutters/gaps and overflowing weight totals before layout.

Tests cover malformed-input recovery, Unicode ranges, formatter idempotence
and rendered equivalence across repository diagrams, CLI status and batch
preflight, and native/WASM tooling parity.
