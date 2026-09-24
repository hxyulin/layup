// Wraps the layup-wasm C ABI: strings in through layup_alloc, one
// length-prefixed JSON result out, every buffer freed with layup_free.

export class LayupError extends Error {
  constructor(reason, line) {
    super(line == null ? reason : `line ${line}: ${reason}`);
    this.name = 'LayupError';
    this.line = line;
    this.reason = reason;
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

  function render(source, { theme = 'light', format = 'svg', darkSelector } = {}) {
    let options = `theme=${theme}\nformat=${format}`;
    if (darkSelector) options += `\ndarkSelector=${darkSelector.replace(/\n/g, ' ')}`;
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
    if (result.error) throw new LayupError(result.error.message, result.error.line);
    return result;
  }

  return { render };
}
