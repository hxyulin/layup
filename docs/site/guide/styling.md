# Styling and international text

Layup uses semantic tones and measured typography. The same scene can render
light, dark, or automatic themes without changing its geometry.

## Tones and custom kinds

The tones are `gray`, `blue`, `green`, `yellow`, `purple`, `orange`, and `red`.
Omitting a tone gives ordinary nodes automatic colors. Authored layout cycles
colors in declaration order; automatic graphs derive them from node IDs so
unrelated insertions do not change existing colors.

```layup source
diagram "Ports and adapters" layout=auto direction=right {
  style service base=node tone=blue
  style port shape=api hollow purple label="port"
  service api "HTTP service" { code "GET /orders" }
  port repository "Repository" { code "find(id)" }
  api -uses-> repository labeled
}
```

`base=` copies a kind; `shape=` sets its geometry. `hollow`/`filled` choose
fill, `mono`/`sans` choose header typography, and `align=` controls alignment.
`role=` adds a role label. `tag=` or a title starting with `[tag]` supplies
semantic context. Typed edge defaults and custom arrows are listed in the
[DSL reference](/reference/dsl#edges).

## Direction and Unicode

Graph flow and text direction are independent. Set
`text-direction=auto|ltr|rtl` on a diagram or node. `auto` follows the first
strong paragraph character. Node overrides apply to descendants, and code
runs stay isolated left-to-right inside RTL prose.

`align=start` follows text direction; `left`, `right`, and `center` are
physical alignments. CJK prose wraps at Unicode line-break opportunities,
including punctuation rules, without requiring spaces. Explicit newlines
create additional text lines. Arabic and Hebrew use shaped glyphs.

<Playground preset="international" />

## System fonts or supplied fonts

Latin, Mono, Arabic, and Hebrew faces are bundled. **No CJK font is bundled.**
By default, the viewer supplies CJK glyphs and the compiler estimates their
widths. Appearance and exact wrapping can vary between systems.

For reproducible fallback measurement, pass a standalone OpenType or TrueType
face. Supplied fonts are tried after bundled fonts, measured, and embedded
whole when used. Collections (`.ttc`) are not supported. Large CJK faces can
substantially increase SVG/HTML size.

```sh
layup render international.layup --font /path/to/CJK-Regular.ttf
layup check international.layup --font /path/to/CJK-Regular.ttf --strict
```

```js
const fontBytes = new Uint8Array(await (await fetch('/fonts/CJK-Regular.ttf')).arrayBuffer());
const result = engine.render(source, { fonts: [fontBytes] });
```

The live editor's **Use your own fallback fonts** section accepts local files;
they stay in the browser. Share links carry source and view selection, not font
bytes. Recipients can select their own matching font.

An inline host can choose a system family through CSS:

```css
.layup { --layup-font-fallback: "Noto Sans CJK SC", sans-serif; }
```

CSS changes viewer appearance; it does not give the engine exact metrics.
For font licensing and provenance, see
[the bundled-font documentation](https://github.com/hxyulin/layup/blob/main/crates/layup/fonts/README.md).
