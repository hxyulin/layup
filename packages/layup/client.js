// Interaction for diagrams rendered by layup/markdown-it: hovering or
// clicking a node highlights it and its edges, and the expand button opens
// a full-window view with pan and zoom. Handlers are delegated from the
// document, so diagrams added by client-side navigation need no setup.
// Importing this module during server rendering does nothing.

const CSS = `
.layup-diagram{position:relative}
.layup-diagram .layup-expand[hidden]{display:flex}
.layup-expand{position:absolute;right:6px;bottom:6px;align-items:center;justify-content:center;width:26px;height:26px;padding:0;cursor:pointer;opacity:0;transition:opacity .15s}
.layup-diagram:hover .layup-expand,.layup-expand:focus-visible{opacity:1}
@media (hover:none){.layup-expand{opacity:1}}
svg.layup .node{cursor:pointer}
svg.layup .node,svg.layup .edge{transition:opacity .15s}
svg.layup.focus .node:not(.hl),svg.layup.focus .edge:not(.hl){opacity:.22}
svg.layup .node.hl .box{stroke-width:2.2}
svg.layup .edge.hl .ln{stroke-width:2.6}
.layup-viewer{width:100vw;height:100vh;max-width:none;max-height:none;margin:0;padding:0;border:0;background:var(--layup-viewer-bg,#fdfdfd);color:var(--layup-viewer-ink,#57606a)}
.layup-viewer::backdrop{background:rgba(0,0,0,.5)}
.layup-viewer .layup-stage{position:absolute;inset:0;cursor:grab;touch-action:none}
.layup-viewer .layup-stage.dragging{cursor:grabbing}
.layup-viewer .layup-stage svg{width:100%;height:100%;display:block}
.layup-viewer .layup-stage svg .frame{display:none}
.layup-viewer .layup-bar{position:absolute;top:10px;right:10px;display:flex;gap:6px;z-index:1}
.layup-viewer .layup-bar button,.layup-expand{font:13px ui-sans-serif,system-ui,sans-serif;color:inherit;border:1px solid rgba(127,127,127,.4);border-radius:6px;background:rgba(127,127,127,.14)}
.layup-viewer .layup-bar button{padding:4px 10px;cursor:pointer}
`;

if (typeof document !== 'undefined' && !document.getElementById('layup-client')) install();

function install() {
  // VitePress's link prefetch reads `pathname` from every in-site <a>,
  // which SVG links lack (vitepress 1.6 and 2.0-alpha); without it, each
  // node link throws once when scrolled into view.
  if (typeof SVGAElement !== 'undefined' && !('pathname' in SVGAElement.prototype)) {
    Object.defineProperty(SVGAElement.prototype, 'pathname', {
      configurable: true,
      get() {
        return new URL(this.href.animVal, this.baseURI).pathname;
      },
    });
  }

  const style = document.createElement('style');
  style.id = 'layup-client';
  style.textContent = CSS;
  document.head.append(style);

  document.addEventListener('pointerover', (e) => {
    const svg = diagram(e.target);
    if (svg && !svg.dataset.pinned) highlight(svg, nodeId(e.target));
  });
  document.addEventListener('pointerout', (e) => {
    const svg = diagram(e.target);
    if (!svg || svg.dataset.pinned) return;
    const next = e.relatedTarget instanceof Element ? e.relatedTarget.closest('.node') : null;
    if (!next || !svg.contains(next)) highlight(svg, null);
  });
  document.addEventListener('click', (e) => {
    if (!(e.target instanceof Element)) return;
    const expand = e.target.closest('.layup-expand');
    if (expand) return openViewer(expand.closest('.layup-diagram').querySelector('svg.layup'));
    const svg = diagram(e.target);
    if (!svg || e.target.closest('a')) return;
    if (svg.dataset.dragged) return void delete svg.dataset.dragged;
    const id = nodeId(e.target);
    pin(svg, id && id !== svg.dataset.pinned ? id : null);
  });
}

function diagram(target) {
  return target instanceof Element ? target.closest('.layup-diagram svg.layup, .layup-viewer svg.layup') : null;
}

function nodeId(target) {
  return target.closest('.node')?.dataset.id ?? null;
}

function pin(svg, id) {
  if (id) svg.dataset.pinned = id;
  else delete svg.dataset.pinned;
  highlight(svg, id);
}

function highlight(svg, id) {
  svg.classList.toggle('focus', id != null);
  const linked = new Set([id]);
  for (const edge of svg.querySelectorAll('.edge')) {
    const on = id != null && (edge.dataset.from === id || edge.dataset.to === id);
    edge.classList.toggle('hl', on);
    if (on) linked.add(edge.dataset.from).add(edge.dataset.to);
  }
  for (const node of svg.querySelectorAll('.node')) node.classList.toggle('hl', id != null && linked.has(node.dataset.id));
}

function openViewer(source) {
  const dialog = document.createElement('dialog');
  dialog.className = 'layup-viewer';
  dialog.setAttribute('aria-label', source.querySelector('title')?.textContent ?? 'Diagram');
  dialog.innerHTML = '<div class="layup-bar"><button type="button" data-act="fit">Fit</button><button type="button" data-act="close">Close</button></div><div class="layup-stage"></div>';
  const svg = source.cloneNode(true);
  svg.removeAttribute('style');
  pin(svg, source.dataset.pinned ?? null);
  dialog.querySelector('.layup-stage').append(svg);
  document.body.append(dialog);
  const colors = getComputedStyle(source);
  dialog.style.setProperty('--layup-viewer-bg', colors.getPropertyValue('--bg'));
  dialog.style.setProperty('--layup-viewer-ink', colors.getPropertyValue('--muted'));
  dialog.addEventListener('close', () => dialog.remove());
  dialog.showModal();
  const view = panZoom(dialog.querySelector('.layup-stage'), svg);
  dialog.querySelector('.layup-bar').addEventListener('click', (e) => {
    const act = e.target.closest('button')?.dataset.act;
    if (act === 'fit') view.fit();
    if (act === 'close') dialog.close();
  });
}

// Pan and zoom by rewriting the viewBox: drag or one finger pans, the wheel,
// a trackpad pinch or two fingers zoom about the pointer, double-click fits.
function panZoom(stage, svg) {
  const [, , W, H] = svg.getAttribute('viewBox').split(/\s+/).map(Number);
  let vb = { x: 0, y: 0, w: W, h: H };
  const apply = () => svg.setAttribute('viewBox', `${vb.x} ${vb.y} ${vb.w} ${vb.h}`);
  const fit = () => {
    const a = stage.clientWidth / Math.max(1, stage.clientHeight);
    vb = W / H > a ? { x: 0, y: -(W / a - H) / 2, w: W, h: W / a } : { x: -(H * a - W) / 2, y: 0, w: H * a, h: H };
    apply();
  };
  const toSvg = (cx, cy) => {
    const r = stage.getBoundingClientRect();
    return { x: vb.x + ((cx - r.left) / r.width) * vb.w, y: vb.y + ((cy - r.top) / r.height) * vb.h };
  };
  const zoomAt = (cx, cy, k) => {
    const p = toSvg(cx, cy);
    const s = Math.min(Math.max(vb.w * k, 120), W * 6) / vb.w;
    vb = { x: p.x - (p.x - vb.x) * s, y: p.y - (p.y - vb.y) * s, w: vb.w * s, h: vb.h * s };
    apply();
  };

  stage.addEventListener('wheel', (e) => {
    e.preventDefault();
    zoomAt(e.clientX, e.clientY, Math.exp(e.deltaY * (e.ctrlKey ? 0.01 : 0.0018)));
  }, { passive: false });

  const pointers = new Map();
  let drag = null;
  let pinch = null;
  stage.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.size === 1) drag = { x: e.clientX, y: e.clientY, vb: { ...vb } };
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a.x - b.x, a.y - b.y) };
      drag = null;
    }
  });
  stage.addEventListener('pointermove', (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pinch && pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      zoomAt((a.x + b.x) / 2, (a.y + b.y) / 2, pinch.d / Math.max(d, 1));
      pinch.d = d;
      svg.dataset.dragged = '1';
      stage.setPointerCapture(e.pointerId);
    } else if (drag) {
      const r = stage.getBoundingClientRect();
      // Capture only once dragging: a capture retargets the click that
      // follows, and a click without movement must reach the node.
      if (!svg.dataset.dragged && Math.abs(e.clientX - drag.x) + Math.abs(e.clientY - drag.y) > 3) {
        svg.dataset.dragged = '1';
        stage.classList.add('dragging');
        stage.setPointerCapture(e.pointerId);
      }
      vb = { ...vb, x: drag.vb.x - ((e.clientX - drag.x) / r.width) * vb.w, y: drag.vb.y - ((e.clientY - drag.y) / r.height) * vb.h };
      apply();
    }
  });
  const release = (e) => {
    pointers.delete(e.pointerId);
    if (pointers.size < 2) pinch = null;
    if (pointers.size === 0) drag = null;
    stage.classList.remove('dragging');
  };
  stage.addEventListener('pointerup', release);
  stage.addEventListener('pointercancel', release);
  stage.addEventListener('dblclick', (e) => {
    e.preventDefault();
    fit();
  });
  // Nothing was dragged if the click that follows pointerdown never moved.
  stage.addEventListener('pointerdown', () => delete svg.dataset.dragged, { capture: true });

  fit();
  return { fit };
}
