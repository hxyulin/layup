// Renders ```layup fences to inline SVG when the Markdown is built.
import { loadSync, LayupError } from './node.js';

export default function layup(md, options = {}) {
  const { theme = 'auto', darkSelector, vue = false, strict = false } = options;
  const engine = loadSync();
  const fallback = md.renderer.rules.fence;

  md.renderer.rules.fence = (tokens, idx, opts, env, self) => {
    const token = tokens[idx];
    const [lang, ...flags] = token.info.trim().split(/\s+/);
    if (lang !== 'layup') return fallback(tokens, idx, opts, env, self);
    // ```layup source: the diagram, then its source as an ordinary code block.
    const source = () => {
      if (!flags.includes('source')) return '';
      const copy = Object.assign(Object.create(Object.getPrototypeOf(token)), token, { info: 'text' });
      return fallback([copy], 0, opts, env, self);
    };
    // Diagnostics name the Markdown line: the fence opens on map[0] (0-based).
    const where = (line) => `${env?.relativePath ?? env?.path ?? 'markdown'}:${(token.map?.[0] ?? 0) + 1 + (line ?? 0)}`;
    let svg;
    try {
      const { output, warnings } = engine.render(token.content, { theme, darkSelector });
      for (const w of warnings) {
        const message = `${where(w.line)}: layup warning: ${w.message}`;
        if (strict) throw new Error(message);
        console.warn(message);
      }
      svg = output;
    } catch (e) {
      if (!(e instanceof LayupError)) throw e;
      const message = `${where(e.line)}: layup error: ${e.reason}`;
      if (strict) throw new Error(message);
      console.error(message);
      return `<pre class="layup-error">${md.utils.escapeHtml(message)}</pre>\n`;
    }
    svg = svg.replace('<svg ', '<svg style="max-width:100%;height:auto" ');
    return `<div class="layup-diagram">${vue ? forVue(svg) : svg}${EXPAND}</div>\n${source()}`;
  };
}

// Shown by @hxyulin/layup/client, which provides the expanded view.
const EXPAND = '<button type="button" class="layup-expand" hidden aria-label="Expand diagram" title="Expand"><svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M10 2h4v4M6 14H2v-4M14 2 9.5 6.5M2 14l4.5-4.5"/></svg></button>';

/** For VitePress: follows its `.dark` class and survives Vue compilation. */
export function vitepress(md, options = {}) {
  layup(md, { darkSelector: '.dark', vue: true, ...options });
}

// Vue templates drop <style> elements and interpolate {{ }}. The stylesheet
// goes through <component is="style">, which renders a real <style>, and
// braces in the markup become entities.
function forVue(svg) {
  const [, before, css, after] = svg.match(/^([\s\S]*?)<style>([\s\S]*?)<\/style>([\s\S]*)$/);
  const braces = (s) => s.replace(/\{/g, '&#123;').replace(/\}/g, '&#125;');
  return `${braces(before)}<component is="style">${css}</component>${braces(after)}`;
}
