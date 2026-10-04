# layup in VitePress

Hover or click a node to highlight its edges. The button in the corner opens a
full-window view with pan and zoom. The API node links to another page.

A `layup` code block renders to an inline SVG when the site builds. It follows
the site's light and dark toggle.

```layup
diagram "Request path" {
  node api "API" href="./other" { code "GET /items" }
  node worker "Worker" { code "poll()" }
  node store "Store" { code "items.put()" }

  api -> worker "sends"
  worker -uses-> store labeled
}
```

A second diagram on the same page uses different characters, including
double braces in a code line, which Vue would otherwise interpolate:

```layup
diagram "Storage contracts — ünïcödé" {
  package "storage-api" {
    trait store "Store interface" { code "get(key) / put(key, {{ value }})" }
  }
  package "application" {
    module app "Catalog" { code "Catalog<DiskStore>" }
  }
  app -uses-> store labeled
}
```

Adding `source` to the fence shows the diagram's source below it:

```layup source
diagram "Cache" {
  node reader "Reader"
  node cache "Cache"
  reader -> cache "reads"
}
```

Decision trees use diamonds for questions and rounded terminals for outcomes.
The same hover, pin, and full-window viewer controls work with these shapes.

```layup
diagram "Review a request" layout=auto direction=right {
  decision ready "Is the request ready?" yellow
  terminal approve "Approve" green
  terminal followup "Request more information" orange
  ready -> approve "Yes"
  ready -> followup "No"
}
```

State machines add initial and final markers, labeled transitions, and
readable return paths. The renderer displays guards and actions as text.

```layup
diagram "Job lifecycle" mode=state-machine direction=right {
  initial begin
  state idle "Idle" blue
  state active "Active" green
  final done
  begin -> idle
  idle -> active "start"
  active -> active "tick"
  active -> idle "reset"
  active -> done "finish"
}
```
