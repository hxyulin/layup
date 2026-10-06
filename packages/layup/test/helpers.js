export const objectId = (...path) => 'object:' + JSON.stringify(path);
export const connectionId = id => 'relationship:' + JSON.stringify(id);
export const authored = id => id.startsWith('object:') ? JSON.parse(id.slice(7)).at(-1)
  : id.startsWith('relationship:') ? JSON.parse(id.slice(13)) : id;
export const xml = value => value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
export const objectSelector = (...path) => '.node[data-id=' + JSON.stringify(objectId(...path)) + ']';
