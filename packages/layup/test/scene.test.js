import { authored, objectId, connectionId, xml } from './helpers.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { loadSync, LayupError } from '../node.js';
import MarkdownIt from 'markdown-it';
import plugin from '../markdown-it.js';

const root = new URL('../../../', import.meta.url);
const engine = loadSync();
const cli = (args, source) => spawnSync(new URL('target/debug/layup', root).pathname, args, { input: source, encoding: 'utf8' });
const slice = (source, span) => new TextDecoder().decode(new TextEncoder().encode(source).slice(span.start, span.end));
const model = `diagram main "Shared services" type=graph {
  node api "API"
  node worker "Worker"
  edge dispatch ::api -> ::worker "dispatch"
  view overview "Overview" {
    include ::api
  }
  view detail "Request path" {\n    slide size=wide
    include ::api ::worker
    step entry "Entry" {
      show objects=[::api]
      highlight objects=[::api]
    }
    step request "Dispatch" {
      show objects=[::worker]
      show connections=[dispatch]
      highlight connections=[dispatch]
      speaker-note "Plain text <b>note</b>"
    }
  }
}`;

function parity(source, options = {}, extra = []) {
  const result = cli(['compile', '-', ...(options.view ? ['--view', options.view] : []), ...extra], source);
  assert.equal(result.status, 0, result.stderr);
  const scene = engine.compile(source, options);
  assert.deepEqual(scene, JSON.parse(result.stdout));
  assert.equal(scene.version, 1);
  assert.equal(scene.units, 'svg-user-units');
  assert.equal(scene.coordinateSystem, 'scene');
  for (const node of scene.nodes) assert.ok(slice(source, node.span).includes(node.objectPath.at(-1)));
  const check = value => {
    if (typeof value === 'number') assert.ok(Number.isFinite(value));
    else if (value && typeof value === 'object') Object.values(value).forEach(check);
  };
  check(scene);
  assert.equal('output' in scene, false);
  return scene;
}

test('compile exposes complete scene geometry, hierarchy, text, source spans and slide metadata', () => {
  const source = '/* 注释 */\r\ndiagram main "API \\"契约\\"" type=graph {\r\n slide size=wide min-font-size=1\r\n group system "System" { node api "مرحبا" href="https://example.com/?a=1&b=2"; node worker "Worker" }\r\n edge request system.api -> system.worker "dispatch"\r\n step entry "Entry" { show objects=[system.api] }\r\n step dispatch "Dispatch" { show objects=[system.worker] connections=[request]; speaker-note "line one\\nline two \\"quoted\\"" }\r\n}';
  const scene = parity(source);
  assert.equal(scene.viewport.width, 1920);
  assert.equal(scene.viewport.height, 1080);
  assert.ok(scene.viewport.slide.scale > 0);
  assert.equal(scene.nodes.find(n => n.objectPath.at(-1) === 'api').parentId, objectId('system'));
  assert.equal(scene.nodes.find(n => n.objectPath.at(-1) === 'api').href, 'https://example.com/?a=1&b=2');
  assert.equal(scene.presentation.steps[1].note, 'line one\nline two "quoted"');
  assert.equal(scene.presentation.steps[1].visibleEdges[0], connectionId('request'));
  assert.ok(scene.items.some(p => p.drawing.type === 'text'));
  assert.ok(!JSON.stringify(scene).includes('data:font'));
});

test('views select the same graph for compile, render, lint, CLI and Markdown', () => {
  assert.equal(parity(model).selectedView, 'overview');
  const scene = parity(model, { view: 'detail' });
  assert.equal(scene.title, 'Request path');
  assert.deepEqual(scene.views.map(v => v.id), ['overview', 'detail']);
  assert.equal(scene.edges[0].id, connectionId('dispatch'));
  assert.equal(scene.presentation.steps.length, 2);
  const svg = engine.render(model, { view: 'overview' }).output;
  assert.ok(svg.includes('data-id="' + xml(objectId('api')) + '"'));
  assert.ok(!svg.includes('data-id="' + xml(objectId('worker')) + '"'));
  const diagnostics = engine.lint(model, { view: 'detail' });
  assert.deepEqual(diagnostics, JSON.parse(cli(['lint', '-', '--view', 'detail', '--json'], model).stdout).map(r => r.diagnostic));
  const md = new MarkdownIt().use(plugin, { view: 'overview', strict: true });
  const document = md.render('```layup view=detail\n' + model + '\n```');
  assert.ok(document.includes('data-id="' + xml(objectId('worker')) + '"'));
  assert.match(document, /data-presentation=/);
});

test('sequence scenes preserve message order, named messages and annotation drawing groups', () => {
  const scene = parity(`diagram main "Request" type=sequence {
  slide size=wide
  participant browser "Browser"
  participant api "API"
  message request ::browser -> ::api "GET /resource"
  message response ::api -> ::browser "200 OK" type=reply
  message validate ::api -> ::api "validate"
  step participants {
    show objects=[::browser, ::api]
  }
  step request {
    show connections=[request]
  }
  step response {
    show connections=[response, validate]
  }
}`);
  assert.equal(scene.mode, 'sequence');
  assert.deepEqual(scene.sequence.participants.map(authored), ['browser', 'api']);
  assert.deepEqual(scene.sequence.messages.map(m => authored(m.id)), ['request', 'response', 'validate']);
  assert.deepEqual(scene.presentation.steps[0].visibleEdges, []);
  assert.deepEqual(scene.presentation.steps[1].visibleEdges.map(authored), ['request']);
  assert.ok(scene.edges.some(e => e.from === e.to));
});

test('compile supports supplied font parity and validates host options', () => {
  const path = new URL('crates/layup/tests/fonts/Fallback.ttf', root);
  const bytes = readFileSync(path);
  const scene = parity('diagram main "CJK" type=graph {\n  node a "中中"\n}', { fonts: [bytes] }, ['--font', path.pathname]);
  assert.equal(scene.fonts.fallbacks.length, 1);
  assert.equal(scene.fonts.systemCjk, true);
  assert.throws(() => engine.compile('diagram main "T" type=graph {\n}', { fonts: ['bad'] }), TypeError);
  assert.throws(() => engine.compile(model, { view: 'detail\noperation=format' }), TypeError);
  assert.throws(() => engine.render(model, { view: 1 }), TypeError);
  assert.throws(() => engine.lint(model, { view: null }), TypeError);
  assert.throws(() => engine.compile(model, { view: 'missing' }), e => e instanceof LayupError && e.code === 'view/unknown');
  assert.throws(() => engine.compile('diagram main "T" type=graph { node a href= }'), e => e instanceof LayupError && e.span && e.code === 'document/syntax');
});

test('repeated sequence messages retain return and async semantics without duplicate-transition lint', () => {
  const source = `diagram main "Retry" type=sequence {
  participant client "Client"
  participant api "API"
  message first ::client -> ::api "request" delivery=async
  message retry ::api -> ::client "retry" type=reply
  message second ::client -> ::api "request" delivery=async
  message again ::api -> ::client "retry" type=reply
}`;
  const scene = parity(source);
  assert.deepEqual(scene.sequence.messages.map(message => message.asynchronous), [true, false, true, false]);
  assert.deepEqual(scene.edges.map(edge => edge.style.dashed), [false, true, false, true]);
  assert.deepEqual(engine.lint(source), []);
  const result = cli(['lint', '-', '--json', '--strict'], source);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), []);
});
