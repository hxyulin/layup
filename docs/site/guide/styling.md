# Styling and international text

Layup uses named palettes and measured typography. The same scene can render
light, dark, or automatic themes without changing its geometry.

## Palettes and styles

The palettes are `gray`, `blue`, `green`, `yellow`, `purple`, `orange`, and `red`.
Omitting a palette gives ordinary nodes automatic colors. Authored layout cycles
colors in declaration order; automatic graphs derive them from node IDs so
unrelated insertions do not change existing colors.

```layup source
diagram main "Ports and adapters" type=graph layout=auto flow-direction=right {
  node-style service base=node palette=blue
  node-style port base=interface fill-color=none palette=purple legend-label="port"
  node api "HTTP service" style=service {
    code "GET /orders"
  }
  node repository "Repository" style=port {
    code "find(id)"
  }
  ::api -> ::repository label=style style=uses
}
```

`base=` inherits a style; `shape=` selects graph geometry. `fill-color=none`
removes fill, `font-family=mono|sans` selects header typography, and
`text-align=` controls alignment. `role=` adds a visible role label and
`tag "..."` supplies explicit context. Direct fill/stroke/text colors can use
`{light: COLOR, dark: COLOR}` values. Connection styles are listed in the
[DSL reference](/reference/dsl#edges).

## Independent paint channels

Paint properties override the coordinated palette without changing layout.
Literal colors stay literal in light and dark exports; theme pairs adapt.
Setting `auto` resets an inherited override and `none` removes fill or stroke.
A container's paint does not implicitly propagate to its children.

```layup source
diagram paint "Direct paint" type=graph layout=auto flow-direction=right background-color={light: "#ffffff", dark: "#101827"} {
  node-style service base=process palette=blue fill-color={light: "#eff6ff", dark: "#172554"} stroke-color={light: "#1d4ed8", dark: "#93c5fd"} text-color={light: "#1e3a8a", dark: "#bfdbfe"}
  node api "API" style=service
  node worker "Worker" style=service palette=green
  edge request api -> worker "request" stroke-color={light: "#1d4ed8", dark: "#93c5fd"} stroke-style=dotted stroke-width=2
}
```

[The paint example](https://github.com/hxyulin/layup/blob/main/examples/paint.layup)
also separates `default-label` from `legend-label` and combines inherited styles
with a rectangle outline and no fill. Legend samples retain declared style paint;
instance overrides remain local to their objects. Colors can be literals, theme tokens
(`background`, `text`, `muted`, `code`, `frame`), or light/dark pairs.

Try [the paint example in the playground](/playground?example=paint), including
both themes and inherited overrides.

## Direction and Unicode

Graph flow and text direction are independent. Set
`text-direction=auto|ltr|rtl` on a diagram or node. `auto` follows the first
strong paragraph character. Node overrides apply to descendants, and code
runs stay isolated left-to-right inside RTL prose.

`text-align=start|end` follows text direction; `left`, `right`, and `center` are
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
