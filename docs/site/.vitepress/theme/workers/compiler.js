import { load } from '@hxyulin/layup';
import wasmUrl from '../../../../../packages/layup/layup.wasm?url';

const engine = load(wasmUrl);
engine.then(() => self.postMessage({ type: 'ready' })).catch(error => self.postMessage({ type: 'load-error', message: error.message }));
self.onmessage = async ({ data }) => {
  const { id, source, operation, view, fonts = [] } = data;
  try {
    const layup = await engine;
    if (source.length > 100000) throw new Error('This editor supports up to 100,000 characters. Use the CLI for larger files.');
    let options = { fonts, ...(view ? { view } : {}) };
    if (operation === 'format') {
      self.postMessage({ type: 'formatted', id, source: layup.format(source) });
    } else if (operation === 'export') {
      self.postMessage({ type: 'exported', id, output: layup.render(source, { ...options, format: data.format, theme: 'auto' }).output, format: data.format });
    } else {
      let scene;
      let selectedView = view || '';
      try { scene = layup.compile(source, options); }
      catch (error) {
        if (!view) throw error;
        // Editing a model can remove or rename the selected view. Only fall
        // back if the source compiles and that view is actually absent.
        let fallback;
        try { fallback = layup.compile(source, { fonts }); } catch { throw error; }
        if (fallback.views.some(item => item.id === view)) throw error;
        scene = fallback;
        selectedView = '';
        options = { fonts };
      }
      const { output } = layup.render(source, { ...options, theme: 'auto', darkSelector: '.dark' });
      self.postMessage({ type: 'rendered', id, output, scene, view: selectedView, viewReset: selectedView !== (view || ''), diagnostics: layup.lint(source, options) });
    }
  } catch (error) {
    let diagnostics;
    try { diagnostics = (await engine).lint(source, { fonts, ...(view ? { view } : {}) }); } catch { /* retain the actual request error */ }
    self.postMessage({ type: 'error', id, message: error.message, diagnostics: diagnostics?.length ? diagnostics : [{ severity: 'error', code: error.code || 'editor', line: error.line ?? null, column: error.column ?? null, message: error.reason || error.message, help: error.help ?? null }] });
  }
};
