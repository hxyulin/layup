<script setup>
import { computed, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue';
import { withBase } from 'vitepress';
import { sampleById, samples } from '../samples.js';
import { decodeShare, encodeShare } from '../share.js';
import { safeLinks } from '../links.js';

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
      renderedView.value = activeOperation.view;
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

function chooseSample() {
  const chosen = sampleById(sample.value);
  source.value = chosen.source;
  view.value = chosen.view || '';
  diagnostics.value = [];
  notice.value = '';
  previewZoom.value = 1;
  schedule();
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

watch([source, view], schedule);
watch(() => props.preset, value => { sample.value = sampleById(value).id; chooseSample(); });
onMounted(() => {
  mounted = true;
  if (props.full) {
    try {
      const shared = decodeShare(window.location.hash);
      if (shared) { source.value = shared.source; view.value = shared.view || ''; }
      else {
        const selected = new URLSearchParams(window.location.search).get('example');
        if (selected) { sample.value = sampleById(selected).id; chooseSample(); }
      }
    } catch (error) { notice.value = error.message; }
  }
  startWorker();
});
onBeforeUnmount(() => { destroyed = true; clearTimeout(debounce); clearTimeout(watchdog); worker?.terminate(); });
</script>

<template>
  <section class="live-editor" aria-label="Live Layup editor">
    <div class="editor-controls">
      <label>Example <select v-model="sample" aria-label="Example" @change="chooseSample"><option v-for="item in samples" :key="item.id" :value="item.id">{{ item.title }}</option></select></label>
      <label>View <select v-model="view" aria-label="View" :disabled="!scene?.views.length"><option value="">First view (default)</option><option v-for="item in scene?.views || []" :key="item.id" :value="item.id">{{ item.title }}</option></select></label>
      <button type="button" :disabled="!workerReady || status === 'updating'" @click="request('format')">Format</button>
      <button type="button" @click="chooseSample">Reset example</button>
      <button v-if="!workerReady && status === 'error'" type="button" @click="startWorker">Restart renderer</button>
    </div>
    <div class="editor-panels">
      <div class="source-panel">
        <label :for="editorId" class="panel-label">Layup source <span>Updates as you type</span></label>
        <textarea :id="editorId" ref="textarea" v-model="source" maxlength="100000" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" :aria-describedby="helpId" @keydown.ctrl.enter.prevent="request()"></textarea>
      </div>
      <div class="preview-panel" :aria-busy="status === 'loading' || status === 'updating'">
        <div class="panel-label preview-label"><div>Diagram <span role="status" aria-live="polite" aria-atomic="true" :class="`status-${status}`">{{ statusText }}</span></div><div v-if="svg" class="preview-tools"><button type="button" aria-label="Zoom preview out" :disabled="previewZoom <= 1" @click="previewZoom = Math.max(1, previewZoom - .5)">−</button><button type="button" @click="previewZoom = 1">Fit preview</button><button type="button" aria-label="Zoom preview in" :disabled="previewZoom >= 5" @click="previewZoom = Math.min(5, previewZoom + .5)">+</button></div></div>
        <div v-if="svg" class="preview-scroll">
          <p v-if="status === 'error'" class="last-valid">Showing the last successful render.</p>
          <div class="layup-diagram" :style="{ width: `${previewZoom * 100}%` }" tabindex="0" aria-label="Diagram preview and presentation controls">
            <div class="rendered-svg" v-html="svg"></div>
            <button type="button" class="layup-expand" aria-label="Expand diagram" title="Expand diagram">⤢</button>
          </div>
        </div>
        <div v-else class="preview-empty">Your diagram appears here. The renderer loads once and runs in your browser.</div>
      </div>
    </div>
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
