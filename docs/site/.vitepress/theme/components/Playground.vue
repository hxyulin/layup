<script setup>
import { computed, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue';
import { withBase } from 'vitepress';
import { sampleById, samples } from '../samples.js';
import { decodeShare, encodeShare } from '../share.js';
import { safeLinks } from '../links.js';
import { EXPAND_ICON, ZOOM_IN_ICON, ZOOM_OUT_ICON } from '../../../../../packages/layup/icons.js';

const props = defineProps({ preset: { type: String, default: 'hello' }, full: Boolean });
const editorId = `layup-source-${useId()}`;
const helpId = `${editorId}-help`;
const sample = ref(sampleById(props.preset).id);
const source = ref(sampleById(props.preset).source);
const view = ref(sampleById(props.preset).view || '');
const svg = ref('');
const scene = ref(null);
const diagnostics = ref([]);
const status = ref('loading');
const notice = ref('');
const fontFiles = ref([]);
const fonts = ref([]);
const textarea = ref();
const panels = ref();
const sourcePanel = ref();
const sourceWidth = ref(50);
const panelHeight = ref(null);
const measuredHeight = ref(522);
const resizing = ref(false);
const workerReady = ref(false);
const renderedSource = ref('');
const renderedView = ref('');
const previewZoom = ref(1);
const current = computed(() => status.value === 'ready' && source.value === renderedSource.value && view.value === renderedView.value);
const statusText = computed(() => ({ loading: 'Loading renderer…', updating: 'Rendering…', ready: diagnostics.value.length ? `Rendered with ${diagnostics.value.length} diagnostic(s)` : 'Rendered · no diagnostics', error: 'Source needs attention' })[status.value]);
let worker;
let debounce;
let watchdog;
let generation = 0;
let mounted = false;
let destroyed = false;
let activeOperation;
let loadedLocation;
let resizeObserver;
let resizeDrag;

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

function startResize(event, axis) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  resizeDrag = {
    axis, pointerId: event.pointerId, x: event.clientX, y: event.clientY,
    width: panels.value.getBoundingClientRect().width,
    fraction: sourceWidth.value, height: sourcePanel.value.getBoundingClientRect().height,
    rows: window.matchMedia('(max-width: 760px)').matches ? 2 : 1,
  };
  resizing.value = true;
}

function moveResize(event) {
  if (event.pointerId !== resizeDrag?.pointerId) return;
  if (resizeDrag.axis === 'width') sourceWidth.value = clamp(resizeDrag.fraction + (event.clientX - resizeDrag.x) / resizeDrag.width * 100, 20, 80);
  else panelHeight.value = clamp(resizeDrag.height + (event.clientY - resizeDrag.y) / resizeDrag.rows, 240, 1200);
}

function endResize(event) {
  if (event.pointerId !== resizeDrag?.pointerId) return;
  resizeDrag = null;
  resizing.value = false;
  if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
}

function resizeKey(event, axis) {
  const keys = axis === 'width' ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
  if (![...keys, 'Home', 'End', 'Enter'].includes(event.key)) return;
  event.preventDefault();
  if (axis === 'width') {
    sourceWidth.value = event.key === 'Enter' ? 50 : event.key === 'Home' ? 20 : event.key === 'End' ? 80 : clamp(sourceWidth.value + (event.key === keys[0] ? -5 : 5), 20, 80);
  } else {
    panelHeight.value = event.key === 'Enter' ? null : event.key === 'Home' ? 240 : event.key === 'End' ? 1200 : clamp(measuredHeight.value + (event.key === keys[0] ? -40 : 40), 240, 1200);
  }
}

function resetPanes() { sourceWidth.value = 50; panelHeight.value = null; }

function download(content, extension, mime) {
  const url = URL.createObjectURL(new Blob([content], { type: mime }));
  const link = document.createElement('a');
  link.href = url;
  link.download = `layup-${sample.value}.${extension}`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function startWorker() {
  worker?.terminate();
  workerReady.value = false;
  status.value = 'loading';
  worker = new Worker(new URL('../workers/compiler.js', import.meta.url), { type: 'module' });
  clearTimeout(watchdog);
  watchdog = setTimeout(() => fail('The renderer could not finish loading. Restart it to retry.'), 30000);
  worker.onmessage = ({ data }) => {
    if (destroyed) return;
    if (data.type === 'ready') {
      clearTimeout(watchdog);
      workerReady.value = true;
      schedule();
      return;
    }
    if (data.type === 'load-error') return fail(data.message);
    if (data.id !== generation) return;
    clearTimeout(watchdog);
    if (data.type === 'error') {
      diagnostics.value = data.diagnostics;
      status.value = 'error';
    } else if (data.type === 'formatted') {
      source.value = data.source;
      schedule();
    } else if (data.type === 'exported') {
      const isSvg = data.format === 'svg';
      download(safeLinks(data.output, data.format), isSvg ? 'svg' : 'html', isSvg ? 'image/svg+xml;charset=utf-8' : 'text/html;charset=utf-8');
      status.value = 'ready';
    } else if (data.type === 'rendered') {
      svg.value = safeLinks(data.output);
      scene.value = data.scene;
      diagnostics.value = data.diagnostics;
      renderedSource.value = activeOperation.source;
      renderedView.value = data.view;
      view.value = data.view;
      if (data.viewReset) notice.value = `The selected view is no longer in this source. Showing ${data.scene.views.length ? 'the first available view' : 'the diagram'}.`;
      status.value = 'ready';
    }
  };
  worker.onerror = event => fail(event.message || 'The renderer could not start. Reload the page to retry.');
}

function fail(message) {
  clearTimeout(watchdog);
  worker?.terminate();
  worker = null;
  workerReady.value = false;
  status.value = 'error';
  diagnostics.value = [{ severity: 'error', code: 'editor', message }];
}

function request(operation = 'render', extra = {}) {
  if (!workerReady.value) return;
  clearTimeout(debounce);
  clearTimeout(watchdog);
  const id = ++generation;
  activeOperation = { source: source.value, view: view.value };
  status.value = 'updating';
  notice.value = '';
  worker.postMessage({ id, operation, ...activeOperation, fonts: Array.from(fonts.value), ...extra });
  watchdog = setTimeout(() => fail('Rendering took too long. Simplify the diagram or restart the renderer; your source is still here.'), 15000);
}

function schedule() {
  if (!mounted || destroyed) return;
  // Invalidate responses as soon as source changes, including during debounce.
  ++generation;
  clearTimeout(debounce);
  if (!worker) { startWorker(); return; }
  if (!workerReady.value) return;
  status.value = 'updating';
  debounce = setTimeout(() => request(), 220);
}

function locationKey(url) {
  return JSON.stringify([url.searchParams.get('example'), new URLSearchParams(url.hash.slice(1)).get('diagram')]);
}

function chooseSample(updateLocation = true) {
  const chosen = sampleById(sample.value);
  source.value = chosen.source;
  view.value = chosen.view || '';
  diagnostics.value = [];
  notice.value = '';
  previewZoom.value = 1;
  if (props.full && updateLocation) {
    const url = new URL(window.location.href);
    url.searchParams.set('example', sample.value);
    url.hash = '';
    loadedLocation = locationKey(url);
    window.history.replaceState(window.history.state, '', url);
  }
  schedule();
}

function syncLocation() {
  if (!props.full) return;
  const url = new URL(window.location.href);
  const key = locationKey(url);
  // Heading links must not discard edits to an example.
  if (key === loadedLocation) return;
  loadedLocation = key;
  try {
    const shared = decodeShare(url.hash);
    if (shared) {
      source.value = shared.source;
      view.value = shared.view || '';
      previewZoom.value = 1;
      notice.value = '';
      schedule();
    } else {
      sample.value = sampleById(url.searchParams.get('example') || props.preset).id;
      chooseSample(false);
    }
  } catch (error) { notice.value = error.message; }
}

function jumpTo(diagnostic) {
  const input = textarea.value;
  if (!input || !diagnostic.line) return;
  let start;
  let end;
  if (diagnostic.span) {
    const bytes = new TextEncoder().encode(source.value);
    const decoder = new TextDecoder();
    start = decoder.decode(bytes.subarray(0, diagnostic.span.start)).length;
    end = decoder.decode(bytes.subarray(0, diagnostic.span.end)).length;
  } else {
    const rows = source.value.split('\n');
    start = rows.slice(0, diagnostic.line - 1).reduce((offset, row) => offset + row.length + 1, 0);
    start += Array.from(rows[diagnostic.line - 1] || '').slice(0, Math.max(0, (diagnostic.column || 1) - 1)).join('').length;
    end = start;
  }
  input.focus();
  input.setSelectionRange(start, end);
}

async function addFonts(event) {
  const files = Array.from(event.target.files || []);
  if (files.some(file => file.size > 32 * 1024 * 1024)) {
    notice.value = 'Please choose font files smaller than 32 MB each.';
    return;
  }
  status.value = 'updating';
  try {
    const bytes = await Promise.all(files.map(async file => new Uint8Array(await file.arrayBuffer())));
    if (destroyed) return;
    fonts.value = bytes;
    fontFiles.value = files.map(file => file.name);
    schedule();
  } catch (error) {
    notice.value = `Could not read this font: ${error.message}`;
    schedule();
  }
}

async function share() {
  const url = new URL(withBase('/playground.html'), window.location.origin);
  url.hash = new URLSearchParams({ diagram: encodeShare(source.value, view.value || undefined) }).toString();
  if (url.href.length > 16000) { notice.value = 'This diagram is too large for a practical share link. Download the source instead.'; return; }
  try { await navigator.clipboard.writeText(url.href); notice.value = 'Copied a link to this source. Font files are not included.'; }
  catch { notice.value = url.href; }
}

watch([source, view], () => { if (!current.value) schedule(); });
watch(() => props.preset, value => { sample.value = sampleById(value).id; chooseSample(false); });
onMounted(() => {
  mounted = true;
  resizeObserver = new ResizeObserver(() => { measuredHeight.value = Math.round(sourcePanel.value.getBoundingClientRect().height); });
  resizeObserver.observe(sourcePanel.value);
  startWorker();
  if (props.full) {
    for (const event of ['layup:navigate', 'hashchange', 'popstate']) window.addEventListener(event, syncLocation);
    syncLocation();
  }
});
onBeforeUnmount(() => {
  destroyed = true;
  resizeObserver?.disconnect();
  for (const event of ['layup:navigate', 'hashchange', 'popstate']) window.removeEventListener(event, syncLocation);
  clearTimeout(debounce); clearTimeout(watchdog); worker?.terminate();
});
</script>

<template>
  <section class="live-editor" :class="{ 'is-resizing': resizing }" :style="{ '--source-fraction': sourceWidth / 100, '--panel-height': panelHeight == null ? undefined : `${panelHeight}px` }" aria-label="Live Layup editor">
    <div class="editor-controls">
      <label>Example <select v-model="sample" aria-label="Example" @change="chooseSample()"><option v-for="item in samples" :key="item.id" :value="item.id">{{ item.title }}</option></select></label>
      <label>View <select v-model="view" aria-label="View" :disabled="!scene?.views.length"><option value="">First view (default)</option><option v-for="item in scene?.views || []" :key="item.id" :value="item.id">{{ item.title }}</option></select></label>
      <button type="button" :disabled="!workerReady || status === 'updating'" @click="request('format')">Format</button>
      <button type="button" @click="chooseSample()">Reset example</button>
      <button type="button" @click="resetPanes">Reset pane sizes</button>
      <button v-if="!workerReady && status === 'error'" type="button" @click="startWorker">Restart renderer</button>
    </div>
    <div ref="panels" class="editor-panels">
      <div :id="`${editorId}-panel`" ref="sourcePanel" class="source-panel">
        <label :for="editorId" class="panel-label">Layup source <span>Updates as you type</span></label>
        <textarea :id="editorId" ref="textarea" v-model="source" maxlength="100000" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" :aria-describedby="helpId" @keydown.ctrl.enter.prevent="request()"></textarea>
      </div>
      <div class="pane-width-handle" role="separator" tabindex="0" aria-label="Resize source and diagram widths" aria-orientation="vertical" :aria-controls="`${editorId}-panel ${editorId}-preview`" aria-valuemin="20" aria-valuemax="80" :aria-valuenow="Math.round(sourceWidth)" :aria-valuetext="`Source ${Math.round(sourceWidth)}%, diagram ${Math.round(100 - sourceWidth)}%`" title="Drag to resize widths. Left/Right arrows adjust; Enter resets." @pointerdown="startResize($event, 'width')" @pointermove="moveResize" @pointerup="endResize" @pointercancel="endResize" @lostpointercapture="endResize" @keydown="resizeKey($event, 'width')"></div>
      <div :id="`${editorId}-preview`" class="preview-panel" :aria-busy="status === 'loading' || status === 'updating'">
        <div class="panel-label preview-label"><div>Diagram <span role="status" aria-live="polite" aria-atomic="true" :class="`status-${status}`">{{ statusText }}</span></div><div v-if="svg" class="preview-tools"><button type="button" aria-label="Zoom preview out" :disabled="previewZoom <= 1" @click="previewZoom = Math.max(1, previewZoom - .5)" v-html="ZOOM_OUT_ICON"></button><button type="button" @click="previewZoom = 1">Fit preview</button><button type="button" aria-label="Zoom preview in" :disabled="previewZoom >= 5" @click="previewZoom = Math.min(5, previewZoom + .5)" v-html="ZOOM_IN_ICON"></button></div></div>
        <div v-if="svg" class="preview-scroll">
          <p v-if="status === 'error'" class="last-valid">Showing the last successful render.</p>
          <div class="layup-diagram" :style="{ width: `${previewZoom * 100}%` }" tabindex="0" aria-label="Diagram preview and presentation controls">
            <div class="rendered-svg" v-html="svg"></div>
            <button type="button" class="layup-expand" aria-label="Expand diagram" title="Expand diagram" v-html="EXPAND_ICON"></button>
          </div>
        </div>
        <div v-else class="preview-empty">Your diagram appears here. The renderer loads once and runs in your browser.</div>
      </div>
    </div>
    <div class="pane-height-handle" role="separator" tabindex="0" aria-label="Resize source and diagram height" aria-orientation="horizontal" :aria-controls="`${editorId}-panel ${editorId}-preview`" aria-valuemin="240" aria-valuemax="1200" :aria-valuenow="measuredHeight" title="Drag to resize both panes. Up/Down arrows adjust; Enter resets." @pointerdown="startResize($event, 'height')" @pointermove="moveResize" @pointerup="endResize" @pointercancel="endResize" @lostpointercapture="endResize" @keydown="resizeKey($event, 'height')"></div>
    <p :id="helpId" class="editor-help">Ctrl+Enter renders now · Tab moves focus · Click a diagnostic to select its source · Expand for pan, zoom and presentation controls</p>
    <ul v-if="diagnostics.length" class="editor-diagnostics" aria-label="Source diagnostics">
      <li v-for="(item, index) in diagnostics" :key="index" :class="item.severity">
        <button type="button" :disabled="!item.line" @click="jumpTo(item)"><strong>{{ item.severity }}</strong><span v-if="item.line"> · {{ item.line }}:{{ item.column || 1 }}</span> · {{ item.message }}</button>
        <p v-if="item.help">{{ item.help }}</p>
      </li>
    </ul>
    <div class="editor-downloads">
      <button type="button" @click="download(source, 'layup', 'text/plain;charset=utf-8')">Download source</button>
      <button type="button" :disabled="!current" @click="request('export', { format: 'svg' })">Download SVG</button>
      <button type="button" :disabled="!current" @click="request('export', { format: 'html' })">Download HTML</button>
      <button type="button" :disabled="!current" @click="request('export', { format: 'embed' })">Download embed</button>
      <button type="button" :disabled="!current" @click="download(JSON.stringify(scene, null, 2), 'json', 'application/json')">Download scene JSON</button>
      <button type="button" @click="share">Copy share link</button>
    </div>
    <details class="editor-fonts">
      <summary>Use your own fallback fonts</summary>
      <p>CJK uses your system fonts by default. Choose standalone .ttf/.otf faces for exact measurement and embedding. Files stay in this browser; share links do not include fonts.</p>
      <input type="file" multiple accept=".ttf,.otf" aria-label="Fallback font files" @change="addFonts">
      <span v-if="fontFiles.length">{{ fontFiles.join(', ') }}</span>
      <button v-if="fontFiles.length" type="button" @click="fonts = []; fontFiles = []; schedule()">Clear fonts</button>
    </details>
    <p v-if="notice" class="editor-notice" role="status">{{ notice }}</p>
    <noscript>The live editor requires JavaScript. The guide's diagrams and source examples remain available.</noscript>
  </section>
</template>
