#!/usr/bin/env python3
"""Generate the second checkpoint's local before/after gallery. Run from repo root."""
from pathlib import Path
import html
import re
import subprocess

root = Path(__file__).resolve().parent.parent
out = root / "out/checkpoint-02"
out.mkdir(parents=True, exist_ok=True)
cases = []
for name, title, note in [
    ("below", "Below", "Only below hints are added: no artificial edges or explicit rows."),
    ("same-layer", "Same layer", "One same-layer hint moves Cache alongside Worker."),
    ("beside", "Beside", "Two beside hints keep Input, Transform, Output adjacent and in order. Notes stays outside that chain."),
]:
    after = (root / f"docs/checkpoints/02-layout-hints/{name}.layup").read_text()
    before = re.sub(r" (?:below|same-layer|beside)=[\w-]+", "", after)
    cases.append((title, note, before, after))
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
(out / "index.html").write_text("""<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Layup checkpoint 2 — layout hints</title>
<style>body{font:16px/1.5 system-ui,sans-serif;background:#f5f7fa;color:#17212f;max-width:1500px;margin:32px auto;padding:0 24px}h1{margin-bottom:8px}section{margin:36px 0}.pair{display:grid;grid-template-columns:1fr 1fr;gap:20px}article{background:white;border:1px solid #d9e1e9;border-radius:12px;padding:16px;min-width:0}img{width:100%;height:auto}pre{white-space:pre-wrap;font-size:12px;overflow-wrap:anywhere}.ok{color:#23683e;font-size:13px}summary{cursor:pointer}@media(max-width:850px){.pair{grid-template-columns:1fr}}</style>
<h1>Checkpoint 2: layout hints</h1><p>Review the layout and source side by side. Click a diagram for full size. Both sides use automatic placement. Only the hints change. Routing is unchanged in this checkpoint.</p>
""" + "".join(sections) + "</html>")
print(out / "index.html")
