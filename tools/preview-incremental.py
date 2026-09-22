#!/usr/bin/env python3
"""Build the checkpoint-3 baseline, then compare the same edits under both layout policies."""
from pathlib import Path
import html
import io
import subprocess
import tarfile
import tempfile

root = Path(__file__).resolve().parent.parent
out = root / "out/checkpoint-04"
out.mkdir(parents=True, exist_ok=True)
# Fixed baseline: checkpoint 3, before disconnected-region isolation and stable tones.
baseline = "ee8f8ad"
with tempfile.TemporaryDirectory(prefix="layup-incremental-baseline-") as tmp:
    data = subprocess.check_output(["git", "archive", baseline], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive.getmembers():
            target = (Path(tmp) / member.name).resolve()
            if Path(tmp).resolve() not in target.parents:
                raise ValueError("unexpected archive path")
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(member).read())
            else:
                raise ValueError("unexpected archive member")
    subprocess.run(["cargo", "build", "--locked", "--manifest-path", str(Path(tmp)/"Cargo.toml"),
                    "-p", "layup-cli", "--target-dir", str(out/"baseline-target")], check=True)
    baseline_bin = out/"baseline-target/debug/layup"
    subprocess.run(["cargo", "build", "-p", "layup-cli"], cwd=root, check=True)
    sections = []
    base = root / "docs/checkpoints/04-incremental/base.layup"
    for name, title, note in [
        ("unrelated", "Insert an unrelated node", "Notes is inserted between existing declarations. The new policy puts unconnected nodes after the graph, without resizing or recoloring it."),
        ("peer", "Add a parallel peer", "Worker shares a layer with the new Audit peer, so it becomes narrower. Existing node colors stay tied to their IDs."),
        ("subsystem", "Append an independent subsystem", "Metrics and Archive form a separate region below the existing service instead of joining its layers."),
    ]:
        edited = root / f"docs/checkpoints/04-incremental/{name}.layup"
        cards = []
        for policy, binary in [("Checkpoint 3", baseline_bin), ("Checkpoint 4", root/"target/debug/layup")]:
            for stage, path in [("Before edit", base), ("After edit", edited)]:
                stem = f"{name}-{policy[-1]}-{'before' if path == base else 'after'}"
                result = subprocess.run([str(binary), "render", str(path), "-o", str(out/f"{stem}.svg")], capture_output=True, text=True, check=True)
                warnings = "\n".join(line for line in result.stderr.splitlines() if "warning:" in line)
                diagnostic = f"<pre>{html.escape(warnings)}</pre>" if warnings else "<p class='ok'>No layout warnings</p>"
                cards.append(f"<article><h3>{policy} — {stage}</h3><a href='{stem}.svg'><img src='{stem}.svg' alt='{title}: {policy}, {stage}'></a>{diagnostic}</article>")
        sections.append(f"<section><h2>{title}</h2><p>{note}</p><div class='pair'>{''.join(cards)}</div><details><summary>Edited source</summary><pre>{html.escape(edited.read_text())}</pre></details></section>")
(out/"index.html").write_text("""<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Layup checkpoint 4 — predictable edits</title>
<style>body{font:16px/1.5 system-ui,sans-serif;background:#f5f7fa;color:#17212f;max-width:1500px;margin:32px auto;padding:0 24px}section{margin:36px 0}.pair{display:grid;grid-template-columns:1fr 1fr;gap:20px}article{background:white;border:1px solid #d9e1e9;border-radius:12px;padding:16px;min-width:0}img{width:100%;height:auto}pre{white-space:pre-wrap;font-size:12px;overflow-wrap:anywhere}.ok{color:#23683e;font-size:13px}summary{cursor:pointer}@media(max-width:850px){.pair{grid-template-columns:1fr}}</style>
<h1>Checkpoint 4: predictable edits</h1><p>Each example shows the same edit under two policies: checkpoint 3 on the top row, checkpoint 4 on the bottom. Compare left to right to see what moves. Colors change once to the new ID-based scheme, then stay stable as peers are inserted. No saved layout is used.</p>
""" + "".join(sections) + "</html>")
print(out/"index.html")
