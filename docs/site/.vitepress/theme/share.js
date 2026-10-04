// UTF-8 source travels in a URL fragment, which is not sent to the web server.
export function encodeShare(source, view) {
  const bytes = new TextEncoder().encode(JSON.stringify({ version: 1, source, view }));
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
export function decodeShare(hash) {
  const encoded = new URLSearchParams(hash.replace(/^#/, '')).get('diagram');
  if (!encoded) return null;
  if (encoded.length > 150000) throw new Error('The shared diagram is too large for this editor.');
  const binary = atob(encoded.replace(/-/g, '+').replace(/_/g, '/'));
  const payload = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(binary, char => char.charCodeAt(0))));
  if (payload.version !== 1 || typeof payload.source !== 'string' || payload.source.length > 100000 || (payload.view !== undefined && typeof payload.view !== 'string')) throw new Error('This diagram link has an unsupported format.');
  return payload;
}
