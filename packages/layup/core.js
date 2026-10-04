// Wraps the layup-wasm C ABI: strings in through layup_alloc, one
// length-prefixed JSON result out, every buffer freed with layup_free.

export class LayupError extends Error {
  constructor(reason, line, diagnostic = {}) {
    super(line == null ? reason : `line ${line}: ${reason}`);
    this.name = 'LayupError';
    this.line = line;
    this.reason = reason;
    this.column = diagnostic.column ?? null;
    this.span = diagnostic.span ?? null;
    this.code = diagnostic.code ?? null;
    this.help = diagnostic.help ?? null;
    this.related = diagnostic.related ?? [];
  }
}

export function wrap(instance) {
  const { memory, layup_alloc, layup_free, layup_render } = instance.exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const put = (text) => {
    const bytes = encoder.encode(text);
    const ptr = layup_alloc(bytes.length);
    new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length];
  };

  function fontOption(font) {
    if (!(font instanceof Uint8Array)) throw new TypeError('fonts must contain Uint8Array font bytes');
    // Bound spreads so large supplied fonts do not exhaust the stack.
    let binary = '';
    for (let i = 0; i < font.length; i += 8192) binary += String.fromCharCode(...font.subarray(i, i + 8192));
    return `\nfont=${btoa(binary)}`;
  }

  function compileOptions(options, fonts, view) {
    if (view !== undefined) {
      if (typeof view !== 'string' || /[\r\n]/.test(view)) throw new TypeError('view must be a single-line string');
      options += `\nview=${view}`;
    }
    for (const font of fonts) options += fontOption(font);
    return options;
  }

  function render(source, { theme = 'light', format = 'svg', darkSelector, fonts = [], view } = {}) {
    let options = `theme=${theme}\nformat=${format}`;
    if (darkSelector) options += `\ndarkSelector=${darkSelector.replace(/\n/g, ' ')}`;
    return invoke(source, compileOptions(options, fonts, view));
  }

  function invoke(source, options) {
    const [src, srcLen] = put(source);
    const [opt, optLen] = put(options);
    let out;
    try {
      out = layup_render(src, srcLen, opt, optLen);
    } finally {
      layup_free(src, srcLen);
      layup_free(opt, optLen);
    }
    const len = new DataView(memory.buffer).getUint32(out, true);
    const json = decoder.decode(new Uint8Array(memory.buffer, out + 4, len));
    layup_free(out, len + 4);
    const result = JSON.parse(json);
    if (result.error) throw new LayupError(result.error.message, result.error.line, result.error);
    return result;
  }

  return {
    render,
    format(source) { return invoke(source, 'operation=format').output; },
    lint(source, { fonts = [], view } = {}) {
      return invoke(source, compileOptions('operation=lint', fonts, view)).diagnostics;
    },
    compile(source, { fonts = [], view } = {}) {
      return invoke(source, compileOptions('operation=compile', fonts, view)).scene;
    },
  };
}
