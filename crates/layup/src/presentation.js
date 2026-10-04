// A small browser controller for presentation metadata embedded in a Layup SVG.
// No rendering or layout runs here. The SVG remains complete until Present is
// selected, so static exports and environments without JavaScript stay usable.
export function createPresentation(svg, host, { onChange } = {}) {
  let plan;
  try { plan = JSON.parse(svg.dataset.presentation || 'null'); } catch { return null; }
  if (plan?.version !== 1 || !Array.isArray(plan.steps) || !plan.steps.length) return null;
  const steps = plan.steps;
  let index = -1;
  const nodes = [...svg.querySelectorAll('.node[data-id]')];
  const edges = [...svg.querySelectorAll('.edge[data-edge-id]')];
  const bar = document.createElement('div');
  bar.className = 'layup-presentation';
  bar.setAttribute('role', 'group');
  bar.setAttribute('aria-label', 'Diagram presentation');
  bar.innerHTML = '<button type="button" data-step="present">Present</button><button type="button" data-step="previous" aria-label="Previous presentation step" hidden>Previous</button><button type="button" data-step="next" aria-label="Next presentation step" hidden>Next</button><button type="button" data-step="all" hidden>Show all</button><span class="layup-step-status" aria-live="polite"></span><span class="layup-step-note" hidden></span>';
  host.append(bar);
  const previous = bar.querySelector('[data-step="previous"]');
  const next = bar.querySelector('[data-step="next"]');
  const status = bar.querySelector('.layup-step-status');
  const note = bar.querySelector('.layup-step-note');
  const set = (target) => {
    let chosen;
    if (target === null || target === -1) chosen = -1;
    else if (typeof target === 'string') chosen = steps.findIndex(s => s.id === target);
    else if (Number.isInteger(target)) chosen = target;
    else return false;
    if (chosen < -1 || chosen >= steps.length || (typeof target === 'string' && chosen < 0)) return false;
    index = chosen;
    const step = steps[index];
    const visibleNodes = new Set(step?.visibleNodes || []);
    const visibleEdges = new Set(step?.visibleEdges || []);
    const highlights = new Set(step?.highlightNodes || []);
    const edgeHighlights = new Set(step?.highlightEdges || []);
    const active = index >= 0;
    const emphasized = active && (highlights.size > 0 || edgeHighlights.size > 0);
    svg.classList.toggle('presenting', active);
    svg.classList.toggle('presentation-focus', emphasized);
    for (const node of nodes) {
      const hidden = active && !visibleNodes.has(node.dataset.id);
      node.classList.toggle('presentation-hidden', hidden);
      node.classList.toggle('presentation-highlight', active && highlights.has(node.dataset.id));
      if (hidden) node.setAttribute('aria-hidden', 'true');
      else node.removeAttribute('aria-hidden');
      for (const link of node.querySelectorAll('a')) {
        if (hidden) {
          if (!link.hasAttribute('data-presentation-tabindex')) link.dataset.presentationTabindex = link.getAttribute('tabindex') ?? '';
          link.setAttribute('tabindex', '-1');
        } else if (link.hasAttribute('data-presentation-tabindex')) {
          const value = link.dataset.presentationTabindex;
          if (value === '') link.removeAttribute('tabindex'); else link.setAttribute('tabindex', value);
          delete link.dataset.presentationTabindex;
        }
      }
    }
    for (const edge of edges) {
      const hidden = active && !visibleEdges.has(edge.dataset.edgeId);
      edge.classList.toggle('presentation-hidden', hidden);
      edge.classList.toggle('presentation-highlight', active && edgeHighlights.has(edge.dataset.edgeId));
      if (hidden) edge.setAttribute('aria-hidden', 'true'); else edge.removeAttribute('aria-hidden');
    }
    bar.querySelector('[data-step="present"]').hidden = active;
    for (const button of [previous, next, bar.querySelector('[data-step="all"]')]) button.hidden = !active;
    previous.disabled = index <= 0;
    next.disabled = index === steps.length - 1;
    status.textContent = active ? `${index + 1} / ${steps.length}: ${step.title}` : '';
    note.textContent = step?.note || '';
    note.hidden = !active || !step.note;
    const state = { index, id: step?.id ?? null, count: steps.length, title: step?.title ?? null, note: step?.note ?? null };
    svg.dispatchEvent(new CustomEvent('layup:step', { bubbles: true, detail: state }));
    onChange?.(state);
    return true;
  };
  const handleClick = (event) => {
    const action = event.target.closest('button')?.dataset.step;
    if (action === 'present') set(0);
    else if (action === 'previous') set(Math.max(0, index - 1));
    else if (action === 'next') set(Math.min(steps.length - 1, index + 1));
    else if (action === 'all') set(-1);
  };
  bar.addEventListener('click', handleClick);
  const keydown = (event) => {
    if (index < 0 || event.ctrlKey || event.metaKey || event.altKey) return false;
    if (event.target.closest?.('input,textarea,select,[contenteditable="true"]')) return false;
    let target;
    if (['ArrowRight', 'PageDown', ' '].includes(event.key)) target = Math.min(steps.length - 1, index + 1);
    else if (['ArrowLeft', 'PageUp'].includes(event.key)) target = Math.max(0, index - 1);
    else if (event.key === 'Home') target = 0;
    else if (event.key === 'End') target = steps.length - 1;
    else return false;
    event.preventDefault();
    set(target);
    return true;
  };
  // Reset cloned presentation state to a complete diagram at initialization.
  set(-1);
  return {
    svg,
    set,
    next: () => set(index < 0 ? 0 : Math.min(steps.length - 1, index + 1)),
    previous: () => set(Math.max(0, index - 1)),
    all: () => set(-1),
    keydown,
    get index() { return index; },
    get count() { return steps.length; },
    destroy() { set(-1); bar.removeEventListener('click', handleClick); bar.remove(); },
  };
}

export const PRESENTATION_CSS = `
svg .presentation-hidden{visibility:hidden!important;pointer-events:none!important}
svg.presentation-focus .node:not(.presentation-highlight),svg.presentation-focus .edge:not(.presentation-highlight){opacity:.3}
svg .node.presentation-highlight .box{stroke-width:2.2}
svg .edge.presentation-highlight .ln{stroke-width:2.6}
.layup-presentation{display:flex;align-items:center;gap:6px;flex-wrap:wrap;font:13px ui-sans-serif,system-ui,sans-serif}
.layup-presentation button{font:inherit;color:inherit;border:1px solid rgba(127,127,127,.4);border-radius:6px;background:rgba(127,127,127,.14);padding:4px 9px;cursor:pointer}
.layup-presentation button:disabled{opacity:.4;cursor:default}
.layup-presentation [hidden]{display:none!important}
.layup-step-status{font-weight:600}
.layup-step-note{flex-basis:100%;white-space:pre-wrap}
.layup-viewer>.layup-presentation{position:absolute;left:12px;bottom:12px;right:12px;z-index:2;padding:8px;border-radius:6px;background:var(--layup-viewer-bg,#fdfdfd)}
.layup-diagram>.layup-presentation{margin-top:8px}
@media(prefers-reduced-motion:reduce){svg .node,svg .edge{transition:none!important}}
`;
