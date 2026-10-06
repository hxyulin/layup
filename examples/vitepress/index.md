# layup in VitePress

Hover or click a node to highlight its edges. The button in the corner opens a
full-window view with pan and zoom. The API node links to another page.

A `layup` code block renders to an inline SVG when the site builds. It follows
the site's light and dark toggle.

```layup
diagram main "Request path" type=graph {
  node api "API" href="./other" {
    code "GET /items"
  }
  node worker "Worker" {
    code "poll()"
  }
  node store "Store" {
    code "items.put()"
  }
  ::api -> ::worker "sends"
  ::worker -> ::store label=style style=uses
}
```

A second diagram on the same page uses different characters, including
double braces in a code line, which Vue would otherwise interpolate:

```layup
diagram main "Storage contracts — ünïcödé" type=graph {
  package storage-api "storage-api" {
    node store "Store interface" style=trait {
      code "get(key) / put(key, {{ value }})"
    }
  }
  package application "application" {
    node app "Catalog" style=module {
      code "Catalog<DiskStore>"
    }
  }
  ::application.app -> ::storage-api.store label=style style=uses
}
```

Adding `source` to the fence shows the diagram's source below it:

```layup source
diagram main "Cache" type=graph {
  node reader "Reader"
  node cache "Cache"
  ::reader -> ::cache "reads"
}
```

Decision trees use diamonds for questions and rounded terminals for outcomes.
The same hover, pin, and full-window viewer controls work with these shapes.

```layup
diagram main "Review a request" type=graph layout=auto flow-direction=right {
  node ready "Is the request ready?" style=decision palette=yellow
  node approve "Approve" style=terminal palette=green
  node followup "Request more information" style=terminal palette=orange
  ::ready -> ::approve "Yes"
  ::ready -> ::followup "No"
}
```

State machines add initial and final markers, labeled transitions, and
readable return paths. The renderer displays guards and actions as text.

```layup
diagram main "Job lifecycle" type=state-machine flow-direction=right {
  initial begin
  state idle "Idle" palette=blue
  state active "Active" palette=green
  final done
  transition connection-1 ::begin -> ::idle
  transition connection-2 ::idle -> ::active "start"
  transition connection-3 ::active -> ::active "tick"
  transition connection-4 ::active -> ::idle "reset"
  transition connection-5 ::active -> ::done "finish"
}
```

Composite states have their own initial paths and preserve the same viewer
controls. Automatic width grows to fit the nested layout.

```layup
diagram main "Connection" type=state-machine flow-direction=right {
  initial start
  state offline "Offline"
  state connected "Connected" {
    initial enter
    state ready "Ready"
    state sending "Sending"
    transition connection-1 ::connected.enter -> ::connected.ready
    transition connection-2 ::connected.ready -> ::connected.sending "send"
    transition connection-3 ::connected.sending -> ::connected.ready "ack"
  }
  transition connection-4 ::start -> ::offline
  transition connection-5 ::offline -> ::connected "connect"
  transition connection-6 ::connected -> ::offline "disconnect"
}
```
