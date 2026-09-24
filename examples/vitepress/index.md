# layup in VitePress

A `layup` code block renders to an inline SVG when the site builds. It follows
the site's light and dark toggle.

```layup
diagram "Request path" {
  node api "API" { code "GET /items" }
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
