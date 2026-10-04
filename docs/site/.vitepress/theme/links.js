// Shared diagrams are editable input. Preserve ordinary hyperlinks without
// giving a source-provided URL permission to execute in the documentation app.
export function safeLinks(markup, format = 'svg') {
  const document = new DOMParser().parseFromString(markup, format === 'svg' ? 'image/svg+xml' : 'text/html');
  for (const link of document.querySelectorAll('a')) {
    for (const attribute of ['href', 'xlink:href']) {
      const href = link.getAttribute(attribute);
      if (!href) continue;
      try {
        if (!['http:', 'https:', 'mailto:', 'tel:'].includes(new URL(href, window.location.href).protocol)) link.removeAttribute(attribute);
      } catch { link.removeAttribute(attribute); }
    }
  }
  return format === 'svg'
    ? new XMLSerializer().serializeToString(document.documentElement)
    : `<!doctype html>\n${document.documentElement.outerHTML}`;
}
