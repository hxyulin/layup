# Write your first diagram

Layup turns a `.layup` text file into a diagram. Start with a few named nodes
and relationships, then add layout and detail as your explanation grows.

## 1. Describe a small system

```layup source
diagram "Hello Layup" {
  node api "Requests" { code "GET /items" }
  node worker "Worker" { code "poll()" }
  node store "Store" { code "items.put()" }
  api -> worker "sends"
  worker -uses-> store labeled
}
```

`diagram` supplies the title. `node api "Requests"` separates the stable ID
`api` from the displayed title. `code` adds a monospace line, and `->` connects
two IDs. `-uses->` is a typed relationship; `labeled` adds its default caption.
Omitted colors receive automatic tones.

## 2. Change it live

Rename a title, add a node, or put `layout=auto direction=right` after the
diagram title. The renderer updates after a short pause in typing.

<Playground preset="hello" />

If a change is invalid, diagnostics point to its source while the last
successful diagram remains visible. **Format** normalizes indentation without
changing strings or comments. **Download source** saves your current edits.

## 3. Render a file

Install from the current checkout to use every feature shown in these docs:

```sh
git clone https://github.com/hxyulin/layup.git
cd layup
cargo install --path crates/layup-cli
```

Save your source as `hello.layup`, then run:

```sh
layup render hello.layup
layup render hello.layup --html --theme auto
layup check hello.layup --strict
```

The first command writes `hello.svg`; `--html` writes an interactive page.
`check` reports layout problems, and `--strict` makes warnings fail in CI.

## Choose what to explain next

| Your explanation | Start here |
| --- | --- |
| Services, components, interfaces, or package boundaries | [Architecture diagrams](/diagrams/architecture) |
| Validation, routing rules, or an algorithm with branches | [Decisions and flowcharts](/diagrams/decisions) |
| An object's lifecycle or nested operating states | [State machines](/diagrams/states) |
| Requests, responses, retries, and asynchronous calls | [Sequence diagrams](/diagrams/sequences) |
| A talk that introduces a system gradually | [Slides and reveal](/guide/presentations) |

Continue with [the source language](/guide/language) to learn IDs, blocks,
strings, and attributes.
