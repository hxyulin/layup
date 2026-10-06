# Tree-sitter Layup

Syntax highlighting, folding and incremental parsing for `.layup` diagrams.
The grammar covers Layup 0.4's shared document syntax and graph, sequence and
state-machine bodies. The language name is `layup`; its scope is `source.layup`.

**Install editor support from Git.** The released `v0.4.0` tag includes the
parser, external scanner and queries. Neovim installation does not require the
Layup rendering engine, Node.js, pnpm or an npm publication. The optional Node
binding is independently versioned at 0.1.0 and is not published to npm.

## Source and revision

| Setting | Value |
| --- | --- |
| Repository | `https://github.com/hxyulin/layup` |
| Release revision | `v0.4.0` |
| Grammar directory | `packages/tree-sitter-layup` |
| Generated parser | `src/parser.c` |
| External scanner | `src/scanner.c` |
| Parser headers | `src/tree_sitter/` |
| Highlight query | `queries/highlights.scm` |
| Folding query | `queries/folds.scm` |

Paths in the last five rows are relative to the grammar directory. The scanner
is required for nested comments and string content; compile it with the parser.
Generated sources are committed, so editor installs do not regenerate the grammar.

Pin the release tag or a commit to keep parser and queries in sync. The example
below checks out a release into a new directory:

```sh
git clone --depth 1 --branch v0.4.0 https://github.com/hxyulin/layup.git layup-tree-sitter
cd layup-tree-sitter/packages/tree-sitter-layup
```

The monorepo's release tag identifies the supported Layup syntax. The Node
package's independent version does not need to match that tag.

## Neovim

The generated parser uses Tree-sitter ABI 15. These instructions are tested with
Neovim 0.12.5. Use a Neovim build that supports ABI 15; inspect its supported
maximum with `:lua print(vim.treesitter.language_version)`.

Choose either the direct install or the `nvim-treesitter` install below, then
add the shared filetype/highlighting configuration. Layup is a custom parser;
`:TSInstall layup` needs the registration below before it can work.

### Direct install on macOS or Linux

Requirements: Git, Neovim and a C compiler available as `cc`. From the grammar
directory in the pinned checkout above:

```sh
LAYUP_NVIM_SITE="$(nvim --clean --headless '+lua io.write(vim.fn.stdpath("data") .. "/site")' +qa)"
mkdir -p "$LAYUP_NVIM_SITE/parser" "$LAYUP_NVIM_SITE/queries/layup"

case "$(uname -s)" in
  Darwin)
    cc -std=c11 -fPIC -dynamiclib -I src src/parser.c src/scanner.c \
      -o "$LAYUP_NVIM_SITE/parser/layup.so"
    ;;
  *)
    cc -std=c11 -fPIC -shared -I src src/parser.c src/scanner.c \
      -o "$LAYUP_NVIM_SITE/parser/layup.so"
    ;;
esac

cp queries/*.scm "$LAYUP_NVIM_SITE/queries/layup/"
```

This uses Neovim's data directory, including `XDG_DATA_HOME` and
`NVIM_APPNAME` when set. No Tree-sitter CLI or npm installation is needed for
this direct build. Keep your environment settings the same when installing and
opening Neovim. On Windows, use an appropriate native C toolchain or the
`nvim-treesitter` installer below.

### Install with nvim-treesitter

This example targets the current `main` API of
[nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter#adding-custom-languages).
Its requirements include Neovim 0.12+, a C compiler, `curl`, `tar` and a
Tree-sitter CLI 0.26.1+ on `PATH`; the Layup development workspace pins 0.26.11.
The configuration below was verified with
[upstream commit e289100](https://github.com/nvim-treesitter/nvim-treesitter/commit/e289100ff98969e118c702199d88b764ce9e7fdf).
Older `master` configurations use a different API; use the direct install
above if keeping that setup. Install the plugin before using this snippet.

Register the custom parser in your `init.lua` **before** the plugin's `setup`,
install or update calls:

```lua
vim.api.nvim_create_autocmd('User', {
  pattern = 'TSUpdate',
  callback = function()
    require('nvim-treesitter.parsers').layup = {
      install_info = {
        url = 'https://github.com/hxyulin/layup',
        revision = 'v0.4.0',
        location = 'packages/tree-sitter-layup',
        queries = 'packages/tree-sitter-layup/queries',
      },
    }
  end,
})
```

`location` selects the grammar directory. `queries` is relative to the
**repository root**, so this path installs the matching highlighting and folding
queries alongside the parser. The committed parser and scanner are used without
regenerating them.

Restart Neovim and run:

```vim
:TSInstall layup
```

Once installation completes, restart Neovim so it discovers the new parser and
queries.

For scripted installation, use
`require('nvim-treesitter').install({ 'layup' }):wait(300000)` and wait for it to
finish, then start a new Neovim session before opening a Layup buffer. See the
[upstream setup guide](https://github.com/nvim-treesitter/nvim-treesitter#setup)
for configuring the plugin and its installation directory.

### Enable highlighting and folds

Add this to `init.lua` for either installation method:

```lua
vim.filetype.add({ extension = { layup = 'layup' } })
vim.api.nvim_create_autocmd('FileType', {
  group = vim.api.nvim_create_augroup('LayupTreesitter', { clear = true }),
  pattern = 'layup',
  callback = function()
    vim.treesitter.start()
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
    vim.wo.foldlevel = 99
  end,
})
```

Open a `.layup` file. `:Inspect` shows highlight captures and `:InspectTree`
shows the syntax tree. Folds start open; use `zc`/`zo` to close/open a fold.
Colors come from the active colorscheme's Tree-sitter capture groups.

### Updates and troubleshooting

- Update the Git revision and install the parser **and** queries together.
  For a direct install, check out the new revision and repeat the build/copy
  commands. For the plugin, change `revision` and run `:TSUpdate layup`.
- Restart Neovim after replacing a parser that is already loaded.
- If the filetype is missing, check `:set filetype?` and the `vim.filetype.add`
  configuration. The filetype and parser name should both be `layup`.
- If the parser is missing or incompatible, inspect
  `:lua =vim.api.nvim_get_runtime_file('parser/layup.*', true)` and check the
  supported ABI. An older parser earlier on `runtimepath` can take precedence.
- If parsing works but highlighting or folds are missing, inspect
  `:lua =vim.api.nvim_get_runtime_file('queries/layup/highlights.scm', true)`
  and the corresponding `folds.scm` path. Earlier queries can override these.
  Ensure the FileType configuration ran and inspect captures with `:Inspect`.

Neovim searches for parsers and queries on `runtimepath`; see its
[Tree-sitter documentation](https://neovim.io/doc/user/treesitter/) for query
precedence and highlighting configuration.

## Other editors and Markdown

Other Tree-sitter integrations can fetch the same Git revision and grammar
subdirectory, compile `src/parser.c` plus `src/scanner.c` with the headers in
`src/tree_sitter/`, and load the queries supported by that editor. Capture names
and folding behavior depend on the host. This repository provides a grammar,
queries and a Node binding; it does not include a VS Code or Zed extension.

`tree-sitter.json` declares `injection-regex` as `^layup$`. Markdown hosts still
need a fenced-code injection query and the `layup` parser registered to use it.
Neovim's Markdown injection queries can use the registered `layup` language once
both parsers and their queries are installed. Strings such as `code "..."` have
no declared language, so the grammar does not inject Rust or JavaScript there.

## Coverage

- Shared document syntax with optional `layup 1`, named diagrams, scopes,
  quoted IDs, dotted/root references, annotations, lists and records.
- Graph, sequence and state-machine examples, including explicit styles,
  weighted rows, four arrows, named connections, views and presentations.
- Direct properties such as `type`, `style`, `node-style`, `edge-style`,
  `palette`, paint channels and distinct flow/text directions.
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

## Grammar development

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

Tests share valid document syntax fixtures with the Rust parser and cover
corpus trees, highlight captures, every canonical example and
checkpoint, query compilation, incremental edits and malformed-input recovery.
CI also verifies that regenerating the parser produces the committed sources.
When editing the grammar, use `pnpm syntax:generate`, review corpus changes,
and rerun `pnpm syntax:test`.

## Optional Node binding

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

## License

MIT OR Apache-2.0; see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
