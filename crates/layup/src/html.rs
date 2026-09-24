//! Interactive HTML wrapper around the SVG: pan and zoom, hover and click
//! highlighting of a node's edges, deep links (`?focus=id`), a theme
//! toggle, and a `postMessage` protocol so a host page can drive an
//! embedded iframe.

use crate::svg::esc;
use crate::{Compiled, Theme};

pub fn render(c: &Compiled, theme: Theme, embed: bool) -> String {
    let svg = crate::svg::render(c, theme);
    let theme_name = match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
        Theme::Auto => "auto",
    };
    TEMPLATE
        .replace("{{TITLE}}", &esc(&c.diagram.title))
        .replace("{{THEME}}", theme_name)
        .replace("{{EMBED}}", if embed { "embed" } else { "" })
        .replace("{{WIDTH}}", &crate::svg::num(c.scene.width))
        .replace("{{HEIGHT}}", &crate::svg::num(c.scene.height))
        .replace("{{SVG}}", &svg)
}

const TEMPLATE: &str = r##"<!doctype html>
<html lang="en" data-theme="{{THEME}}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{TITLE}}</title>
<style>
  html,body{margin:0;height:100%;overflow:hidden;background:#fdfdfd;color:#57606a;font:13px ui-sans-serif,-apple-system,"Segoe UI",Helvetica,Arial,sans-serif}
  html[data-theme=dark],html[data-theme=dark] body{background:#0d1117;color:#8b949e}
  @media (prefers-color-scheme: dark){html[data-theme=auto],html[data-theme=auto] body{background:#0d1117;color:#8b949e}}
  #stage{position:absolute;inset:0;cursor:grab;touch-action:none}
  #stage.dragging{cursor:grabbing}
  #stage svg{width:100%;height:100%;display:block}
  #stage svg .frame{display:none}
  #bar{position:absolute;top:10px;right:10px;display:flex;gap:6px;z-index:2}
  #bar button{border:1px solid #d0d7de;background:#fff;color:#57606a;border-radius:6px;padding:4px 9px;font:inherit;cursor:pointer}
  #bar button:hover{background:#f6f8fa}
  html[data-theme=dark] #bar button{background:#161b22;border-color:#30363d;color:#8b949e}
  @media (prefers-color-scheme: dark){html[data-theme=auto] #bar button{background:#161b22;border-color:#30363d;color:#8b949e}}
  body.embed #bar{display:none}
  #hint{position:absolute;left:12px;bottom:10px;font-size:11.5px;opacity:.7;pointer-events:none}
  body.embed #hint{display:none}
  svg.focus .node:not(.hl),svg.focus .edge:not(.hl){opacity:.22;transition:opacity .15s}
  svg .node,svg .edge{transition:opacity .15s}
  svg .node{cursor:pointer}
  svg .node.hl .box{stroke-width:2.2}
  svg .edge.hl .ln{stroke-width:2.6}
</style>
</head>
<body class="{{EMBED}}">
<div id="bar">
  <button data-act="fit" title="Fit to window (F)">Fit</button>
  <button data-act="one" title="Actual size (1)">1:1</button>
  <button data-act="theme" title="Toggle theme (T)">Theme</button>
  <button data-act="clear" title="Clear selection (Esc)">Clear</button>
</div>
<div id="stage">{{SVG}}</div>
<div id="hint">drag to pan · wheel to zoom · hover or click a node · ?focus=id</div>
<script>
(() => {
  const W = {{WIDTH}}, H = {{HEIGHT}};
  const stage = document.getElementById('stage');
  const svg = stage.querySelector('svg');
  const html = document.documentElement;
  const params = new URLSearchParams(location.search);
  let vb = { x: 0, y: 0, w: W, h: H };
  let pinned = null;

  const apply = () => svg.setAttribute('viewBox', `${vb.x} ${vb.y} ${vb.w} ${vb.h}`);
  const aspect = () => stage.clientWidth / Math.max(1, stage.clientHeight);
  const fit = () => {
    const a = aspect();
    if (W / H > a) { vb = { x: 0, y: -(W / a - H) / 2, w: W, h: W / a }; }
    else { vb = { x: -(H * a - W) / 2, y: 0, w: H * a, h: H }; }
    apply();
  };
  const actual = () => {
    const w = stage.clientWidth, h = stage.clientHeight;
    vb = { x: (W - w) / 2, y: Math.min(0, (H - h) / 2), w, h };
    apply();
  };
  const toSvg = (cx, cy) => {
    const r = stage.getBoundingClientRect();
    return { x: vb.x + (cx - r.left) / r.width * vb.w, y: vb.y + (cy - r.top) / r.height * vb.h };
  };
  const zoomAt = (cx, cy, k) => {
    const p = toSvg(cx, cy);
    const nw = Math.min(Math.max(vb.w * k, 120), W * 6);
    const s = nw / vb.w;
    vb = { x: p.x - (p.x - vb.x) * s, y: p.y - (p.y - vb.y) * s, w: nw, h: vb.h * s };
    apply();
  };

  stage.addEventListener('wheel', e => {
    e.preventDefault();
    const k = Math.exp((e.ctrlKey ? e.deltaY * 0.01 : e.deltaY * 0.0018));
    zoomAt(e.clientX, e.clientY, k);
  }, { passive: false });

  let drag = null;
  stage.addEventListener('pointerdown', e => {
    if (e.button !== 0) return;
    drag = { x: e.clientX, y: e.clientY, vb: { ...vb }, moved: false };
  });
  stage.addEventListener('pointermove', e => {
    if (!drag) return;
    const r = stage.getBoundingClientRect();
    const dx = (e.clientX - drag.x) / r.width * vb.w, dy = (e.clientY - drag.y) / r.height * vb.h;
    // Capture only once dragging: a capture retargets pointerup to the stage,
    // and a click without movement must reach the node it pins.
    if (!drag.moved && Math.abs(e.clientX - drag.x) + Math.abs(e.clientY - drag.y) > 3) { drag.moved = true; stage.classList.add('dragging'); stage.setPointerCapture(e.pointerId); }
    vb = { ...vb, x: drag.vb.x - dx, y: drag.vb.y - dy };
    apply();
  });
  stage.addEventListener('pointerup', e => {
    if (drag && !drag.moved) {
      const node = e.target.closest('.node');
      if (node) { pin(node.dataset.id); } else { pin(null); }
    }
    drag = null; stage.classList.remove('dragging');
  });
  stage.addEventListener('dblclick', e => { e.preventDefault(); fit(); });

  const nodes = () => [...svg.querySelectorAll('.node')];
  const edges = () => [...svg.querySelectorAll('.edge')];
  const highlight = id => {
    if (!id) { svg.classList.remove('focus'); nodes().forEach(n => n.classList.remove('hl')); edges().forEach(x => x.classList.remove('hl')); return; }
    svg.classList.add('focus');
    const linked = new Set([id]);
    edges().forEach(x => {
      const on = x.dataset.from === id || x.dataset.to === id;
      x.classList.toggle('hl', on);
      if (on) { linked.add(x.dataset.from); linked.add(x.dataset.to); }
    });
    nodes().forEach(n => n.classList.toggle('hl', linked.has(n.dataset.id)));
  };
  const pin = id => {
    pinned = id;
    highlight(id);
    if (window.parent !== window) window.parent.postMessage({ layup: 'select', id }, '*');
  };
  stage.addEventListener('pointerover', e => {
    if (pinned || drag) return;
    const node = e.target.closest('.node');
    highlight(node ? node.dataset.id : null);
  });
  stage.addEventListener('pointerout', e => {
    if (pinned) return;
    if (!e.relatedTarget || !e.relatedTarget.closest || !e.relatedTarget.closest('.node')) highlight(null);
  });
  const focusOn = (id, zoom) => {
    const node = svg.querySelector(`.node[data-id="${CSS.escape(id)}"]`);
    if (!node) return false;
    pin(id);
    if (zoom) {
      const b = node.querySelector('.box');
      const x = +b.getAttribute('x'), y = +b.getAttribute('y'), w = +b.getAttribute('width'), h = +b.getAttribute('height');
      const a = aspect();
      const pad = 1.8;
      let vw = Math.max(w * pad, 320), vh = vw / a;
      if (vh < h * pad) { vh = h * pad; vw = vh * a; }
      vb = { x: x + w / 2 - vw / 2, y: y + h / 2 - vh / 2, w: vw, h: vh };
      apply();
    }
    return true;
  };

  const setTheme = t => { html.dataset.theme = t; svg.classList.toggle('dark', t === 'dark'); svg.classList.toggle('auto', t === 'auto'); try { localStorage.setItem('layup-theme', t); } catch (_) {} };
  const cycleTheme = () => { const order = ['light', 'dark', 'auto']; setTheme(order[(order.indexOf(html.dataset.theme) + 1) % 3]); };
  document.getElementById('bar').addEventListener('click', e => {
    const act = e.target.dataset.act;
    if (act === 'fit') fit(); else if (act === 'one') actual(); else if (act === 'theme') cycleTheme(); else if (act === 'clear') pin(null);
  });
  window.addEventListener('keydown', e => {
    if (e.key === 'Escape') pin(null); else if (e.key === 'f' || e.key === 'F') fit(); else if (e.key === '1') actual(); else if (e.key === 't' || e.key === 'T') cycleTheme();
  });
  window.addEventListener('resize', () => { if (!pinned) fit(); });
  window.addEventListener('message', e => {
    const m = e.data || {};
    if (m.layup === 'focus') focusOn(m.id, m.zoom !== false);
    else if (m.layup === 'clear') { pin(null); fit(); }
    else if (m.layup === 'theme') setTheme(m.theme);
    else if (m.layup === 'fit') fit();
  });

  const qTheme = params.get('theme');
  let saved = null; try { saved = localStorage.getItem('layup-theme'); } catch (_) {}
  if (qTheme) setTheme(qTheme); else if (saved) setTheme(saved); else setTheme(html.dataset.theme);
  fit();
  const focus = params.get('focus');
  if (focus) focusOn(focus, true);
  if (window.parent !== window) window.parent.postMessage({ layup: 'ready', width: W, height: H }, '*');
})();
</script>
</body>
</html>
"##;
