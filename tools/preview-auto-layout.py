#!/usr/bin/env python3
"""Generate the first checkpoint's local before/after gallery. Run from repo root."""
from pathlib import Path
import html
import subprocess

root = Path(__file__).resolve().parent.parent
out = root / "out/checkpoint-01"
out.mkdir(parents=True, exist_ok=True)
branch = (root / "examples/auto-layout.layup").read_text()
nested = (root / "docs/checkpoints/01-auto-layout/nested.layup").read_text()
expanded = branch.replace('  node store', '  node audit "Audit" { code "record(event)" }\n  node store').replace('  auth -> store', '  gateway -> audit\n  audit -> store\n  auth -> store')
cases = [
    ("Branching", "Same declarations and edges. Only layout=auto changes.", branch.replace("layout=auto", "layout=manual"), branch),
    ("Nested", "Automatic placement also applies inside the authored service container.", nested.replace("layout=auto", "layout=manual"), nested),
    ("Add a branch", "Both use automatic layout. Adding Audit creates a third peer; the existing canvas policy widens the diagram for three columns.", branch, expanded),
]
sections = []
for i, (title, note, before, after) in enumerate(cases):
    cards = []
    for label, source in [("Before", before), ("After", after)]:
        stem = f"{i + 1}-{label.lower()}"
        path = out / f"{stem}.layup"
        path.write_text(source)
        result = subprocess.run([str(root / "target/debug/layup"), "render", str(path)], capture_output=True, text=True, check=True)
        warnings = "\n".join(line for line in result.stderr.splitlines() if "warning:" in line)
        warning_html = f"<pre>{html.escape(warnings)}</pre>" if warnings else "<p class='ok'>No layout warnings</p>"
        cards.append(f"<article><h3>{label}</h3><a href='{stem}.svg'><img src='{stem}.svg' alt='{html.escape(title)}: {label}'></a>{warning_html}<details><summary>Source</summary><pre>{html.escape(source)}</pre></details></article>")
    sections.append(f"<section><h2>{title}</h2><p>{note}</p><div class='pair'>{''.join(cards)}</div></section>")
(out / "index.html").write_text("""<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Layup checkpoint 1 — automatic placement</title>
<style>body{font:16px/1.5 system-ui,sans-serif;background:#f5f7fa;color:#17212f;max-width:1500px;margin:32px auto;padding:0 24px}h1{margin-bottom:8px}section{margin:36px 0}.pair{display:grid;grid-template-columns:1fr 1fr;gap:20px}article{background:white;border:1px solid #d9e1e9;border-radius:12px;padding:16px;min-width:0}img{width:100%;height:auto}pre{white-space:pre-wrap;font-size:12px;overflow-wrap:anywhere}.ok{color:#23683e;font-size:13px}summary{cursor:pointer}@media(max-width:850px){.pair{grid-template-columns:1fr}}</style>
<h1>Checkpoint 1: automatic placement</h1><p>Review the layout and source side by side. Click a diagram for full size. The first two comparisons preserve declaration order in Before and infer layers in After. Routing is unchanged in this checkpoint.</p>
""" + "".join(sections) + "</html>")
print(out / "index.html")
