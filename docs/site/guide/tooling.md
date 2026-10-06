# Diagnostics and formatting

Source tooling shares the same Rust implementation in the CLI and browser.
Use it to catch authoring problems before a diagram enters a document or talk.

## Editor syntax highlighting

The repository includes a separate
[Tree-sitter Layup grammar](https://github.com/hxyulin/layup/tree/main/packages/tree-sitter-layup)
with highlight and folding queries. It recognizes `.layup` files, inline
`//` comments, nested `/* … */` comments, namespaced annotations, structured
values, Unicode identifiers and qualified references. The package guide
includes a Git-based Neovim installation pinned to `v0.4.0`, a custom
`nvim-treesitter` configuration and an optional Node parsing example.

The grammar covers shared document syntax and graph, sequence and state
bodies. Extension declaration heads can be highlighted before a renderer
implements them. Use
`layup lint` for authoritative language and semantic diagnostics. Custom diagram
bodies can use the shared declaration syntax; arbitrary embedded languages
require additional grammars and editor injection queries.

Editor installs fetch the grammar and queries from Git; no npm publication is
required. The optional Node binding is not published to npm. Tree-sitter
highlighting is an editor integration; the documentation playground continues
to use its existing editor.

Named documents have an optional `layup 1` revision assertion. See
[shared document syntax](/guide/language-v1) for extensions and exact typed values.
Use `layup inspect document.layup` or `engine.inspect(source)` to examine
unavailable diagram bodies and partial syntax with diagnostics, without layout.

## Lint a source file or Markdown

```sh
layup lint diagram.layup
layup lint diagram.layup --strict
layup lint guide.md --json
layup lint model.layup --view detail
```

Lint collects independent syntax errors with recovery at sibling boundaries.
For valid syntax, it compiles the document and adds authoring checks to layout
warnings. Semantic validation stops at its first error. Warnings do not change
rendering; `--strict` makes them fail CI.

| Rule | Checks |
| --- | --- |
| `layout` | Overflow, routing, reachability, missing glyphs, slide readability |
| `lint/unused-style` | Custom styles without uses; shared-model uses count across views |
| `lint/unused-arrow` | Unused edge styles |
| `lint/duplicate-transition` | Identical graph/machine transitions; sequence retries are distinct events |

Duplicate properties and unsupported bodies are errors. Unknown names can include typo suggestions and related locations. The live
editor shows rich diagnostics and selects the corresponding source when you
click one. Invalid edits retain the last successful preview; downloads of
rendered output wait for valid current source.

## Format without changing the explanation

```sh
layup fmt diagram.layup
layup fmt diagram.layup --check
layup fmt examples/*.layup --write
```

The formatter preserves comments, strings, statement order, and semantics.
It validates syntax before formatting. Batch writes preflight every input so
one syntax error does not leave a partially formatted batch. `--check` writes
nothing and exits unsuccessfully if a file needs formatting.

In JavaScript:

```js
const formatted = engine.format(source);
const diagnostics = engine.lint(source, { fonts, view: 'detail' });
```

`format` throws for invalid syntax. `lint` returns DSL errors as diagnostics;
invalid host options or font data can still throw. Automatic fixes and an LSP
are not implemented.

## Source positions

Spans use zero-based UTF-8 byte offsets with an exclusive `end`. Lines and
columns are one-based Unicode scalar positions. Tabs count as one scalar;
terminal excerpts expand them for display. CRLF is one line break.

JavaScript string offsets count UTF-16 code units. Convert a byte span before
using it with an editor selection:

```js
const bytes = new TextEncoder().encode(source);
const start = new TextDecoder().decode(bytes.subarray(0, diagnostic.span.start)).length;
```

CLI JSON diagnostics include file names; Markdown locations are absolute in
the containing file. WASM errors retain `line`, `column`, `span`, `code`,
`help`, and `related` alongside the existing error message.
