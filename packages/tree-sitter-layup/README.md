# Tree-sitter Layup

A syntax grammar for Layup software diagrams, with highlighting and folding
queries, a generated C parser, and a Node binding. The language name is `layup`,
the scope is `source.layup`, and the file extension is `.layup`.

The package lives in the Layup repository and is not yet published to npm.
It is versioned independently from the diagram engine.

## Coverage

- Existing graph, decision, sequence, state-machine, model/view and presentation
  examples, including custom styles, weighted rows and typed arrows.
- The `layup 1` document prototype: scopes, quoted IDs, qualified references,
  annotations, lists and records.
- The agreed replacement vocabulary, including `type`, `style`, `node-style`,
  `edge-style`, `palette`, explicit color properties and direction properties.
- Arbitrary property names, custom declaration heads, namespaced annotations
  such as `@company.analysis(...)`, Unicode identifiers and international text.
- Inline `//` comments and nested `/* … */` comments. Comment markers inside
  strings stay string content. Strings can span lines and include escapes.
- Incremental parsing and error recovery for incomplete edits.

This is a syntax grammar, not the compiler's semantic validator. Recognition
of proposed syntax does not mean that the engine implements it. It does not
resolve IDs, check property values, select diagrams, or emit typo warnings;
use `layup lint` for those checks. Unknown diagram types can use the common
statement syntax. Arbitrary foreign body syntax may produce `ERROR` nodes and
needs a separate grammar for precise highlighting.

The highlight query uses standard captures such as `@keyword`, `@variable`,
`@property`, `@attribute`, `@type`, `@constant`, `@string`, `@string.escape`,
`@number`, `@boolean`, `@comment`, `@operator` and punctuation captures.
Keywords are contextual: `node node "Node"` highlights the first `node` as a
keyword and the ID as a variable. Custom declaration heads are types. The
folding query covers blocks, structured values, comments and multiline strings;
editors decide which ranges are actually multiline.

## Build and test

From the repository root, with Node 22.9+, pnpm, Python 3 and a C/C++ compiler:

```sh
pnpm install --frozen-lockfile
pnpm syntax:generate
pnpm syntax:test
# Or: just syntax-test
```

The CLI is pinned to Tree-sitter 0.26.11 and emits ABI 15. Generated C/JSON files
and headers are committed so editor installers can build the parser without
running JavaScript. Keep `src/scanner.c` with `src/parser.c`: the scanner handles
nested comments and string content. Compiled libraries and Node binaries are
ignored.

Tests cover corpus trees, highlight captures, every canonical example and
checkpoint, query compilation, incremental edits and malformed-input recovery.
CI also verifies that regenerating the parser produces the committed sources.
When editing the grammar, use `pnpm syntax:generate`, review corpus changes,
and rerun `pnpm syntax:test`.

## Neovim

The following uses Neovim's built-in Tree-sitter support (tested with Neovim
0.12). It does not require a particular `nvim-treesitter` plugin version.
From the repository root on macOS or Linux:

```sh
cd packages/tree-sitter-layup
# Adjust this path if your Neovim uses a different data directory.
NVIM_SITE="$HOME/.local/share/nvim/site"
mkdir -p "$NVIM_SITE/parser" "$NVIM_SITE/queries/layup"
pnpm exec tree-sitter build --output "$NVIM_SITE/parser/layup.so"
cp queries/*.scm "$NVIM_SITE/queries/layup/"
```

Add to your Neovim configuration:

```lua
vim.filetype.add({ extension = { layup = 'layup' } })
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'layup',
  callback = function()
    vim.treesitter.start()
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
  end,
})
```

Open a `.layup` file and use `:Inspect` to see its highlight captures or
`:InspectTree` to inspect its syntax tree. Rebuild and copy queries after grammar
updates; restart Neovim to load the new native parser. Other editors can build
`src/parser.c` plus `src/scanner.c` and load `queries/highlights.scm` and
`queries/folds.scm` through their Tree-sitter integration. This package does not
include a VS Code or Zed extension.

`tree-sitter.json` sets `injection-regex` to `^layup$` for hosts that discover
injection languages from grammar metadata. Markdown highlighting still needs a
host Markdown grammar with fenced-code injection support and this parser
registered as `layup`. The package does not automatically inject Rust or
JavaScript into `code "..."`, because those strings do not declare a language.

## Node binding

Within this workspace:

```sh
pnpm --filter @hxyulin/tree-sitter-layup exec node --input-type=module
```

```js
import Parser from 'tree-sitter';
import Layup from '@hxyulin/tree-sitter-layup';

const parser = new Parser();
parser.setLanguage(Layup);
const tree = parser.parse('diagram g type=graph { node api "API" }');
console.log(tree.rootNode.toString());

const query = new Parser.Query(Layup, Layup.HIGHLIGHTS_QUERY);
console.log(query.captures(tree.rootNode));
// Layup.FOLDS_QUERY contains the folding query.
```

The Node binding is ESM and supports the `tree-sitter` 0.25 runtime. Its syntax
tree is an editor tree, not Layup's structured rendering input. Native consumers
use UTF-8 byte positions; the Node runtime parses JavaScript strings as UTF-16,
so its indices and columns use UTF-16 code units. Do not pass these positions
directly to the Rust compiler's byte-span APIs.

To inspect a standalone source package, from the repository root:

```sh
pnpm --filter @hxyulin/tree-sitter-layup pack --pack-destination "$PWD/out/npm"
```

The tarball contains source, queries, metadata, Node bindings and both licenses.
It builds the native binding during installation; no prebuilt native binaries
or WASM modules are included. Non-Node editor installations only need a C
compiler for the generated parser and scanner.
