#!/usr/bin/env python3
"""Build the checkpoint-2 baseline, then compare identical sources with today's router."""
from pathlib import Path
import html
import io
import subprocess
import tarfile
import tempfile

root = Path(__file__).resolve().parent.parent
out = root / "out/checkpoint-03"
out.mkdir(parents=True, exist_ok=True)
# Fixed baseline: checkpoint 2, before the routing fallback was added.
baseline = "d04da27"
with tempfile.TemporaryDirectory(prefix="layup-routing-baseline-") as tmp:
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
    for name, title, note in [
        ("skip-node", "Skip a busy stage", "Same nodes and edge: find a path around the intervening work."),
        ("return-path", "A return path", "A backward edge gets an outside path without via hints."),
        ("pinned-ports", "Keep explicit ports", "The edge still leaves and enters on the right, with a path outside the node boundaries."),
    ]:
        path = root/f"docs/checkpoints/03-routing/{name}.layup"
        cards = []
        for label, binary in [("Before", baseline_bin), ("After", root/"target/debug/layup")]:
            stem = f"{name}-{label.lower()}"
            result = subprocess.run([str(binary), "render", str(path), "-o", str(out/f"{stem}.svg")],
                                    capture_output=True, text=True, check=True)
            warnings = "\n".join(line for line in result.stderr.splitlines() if "warning:" in line)
            diagnostic = f"<pre>{html.escape(warnings)}</pre>" if warnings else "<p class='ok'>No layout warnings</p>"
            cards.append(f"<article><h3>{label}</h3><a href='{stem}.svg'><img src='{stem}.svg' alt='{title}: {label}'></a>{diagnostic}</article>")
        sections.append(f"<section><h2>{title}</h2><p>{note}</p><div class='pair'>{''.join(cards)}</div><details><summary>Identical source on both sides</summary><pre>{html.escape(path.read_text())}</pre></details></section>")
(out/"index.html").write_text("""<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Layup checkpoint 3 — automatic routing</title>
<style>body{font:16px/1.5 system-ui,sans-serif;background:#f5f7fa;color:#17212f;max-width:1500px;margin:32px auto;padding:0 24px}section{margin:36px 0}.pair{display:grid;grid-template-columns:1fr 1fr;gap:20px}article{background:white;border:1px solid #d9e1e9;border-radius:12px;padding:16px;min-width:0}img{width:100%;height:auto}pre{white-space:pre-wrap;font-size:12px;overflow-wrap:anywhere}.ok{color:#23683e;font-size:13px}summary{cursor:pointer}@media(max-width:850px){.pair{grid-template-columns:1fr}}</style>
<h1>Checkpoint 3: automatic routing</h1><p>Before uses the checkpoint-2 router; After uses the new fallback. Source and node positions are identical. Click a diagram to view full size.</p>
""" + "".join(sections) + "</html>")
print(out/"index.html")
