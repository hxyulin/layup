import './sync-crate-fixtures.mjs';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import MarkdownIt from 'markdown-it';
import { parseDocument } from 'yaml';

const root = fileURLToPath(new URL('../', import.meta.url));
const md = new MarkdownIt({ html: true });
let links = 0;
function checkLink(filename, target) {
  if (!target || /^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(target)) return;
  const local = decodeURIComponent(target.split(/[?#]/)[0]);
  assert.ok(existsSync(resolve(root, dirname(filename), local)), `${filename}: missing target ${target}`);
  links++;
}
function checkTokens(filename, tokens) {
  for (const token of tokens) {
    if (token.type === 'link_open') checkLink(filename, token.attrGet('href'));
    if (token.type === 'image') checkLink(filename, token.attrGet('src'));
    if (token.type === 'html_block' || token.type === 'html_inline') {
      for (const [, attribute, value] of token.content.matchAll(/\b(href|src|srcset)\s*=\s*["']([^"']*)["']/gi)) {
        const targets = attribute === 'srcset' ? value.split(',').map(item => item.trim().split(/\s+/)[0]) : [value];
        for (const target of targets) checkLink(filename, target);
      }
    }
    if (token.children) checkTokens(filename, token.children);
  }
}
for (const filename of ['README.md', 'CONTRIBUTING.md', 'AGENTS.md']) {
  checkTokens(filename, md.parse(readFileSync(join(root, filename), 'utf8'), {}));
}
const directory = join(root, '.github/ISSUE_TEMPLATE');
for (const filename of readdirSync(directory).filter(file => file.endsWith('.yml'))) {
  const document = parseDocument(readFileSync(join(directory, filename), 'utf8'), { uniqueKeys: true });
  assert.equal(document.errors.length, 0, `${filename}: ${document.errors.join('; ')}`);
  const form = document.toJS();
  if (filename === 'config.yml') {
    assert.equal(typeof form.blank_issues_enabled, 'boolean', filename);
    for (const contact of form.contact_links ?? []) {
      assert.ok(contact.name && contact.about, `${filename}: contact needs name and about`);
      assert.equal(new URL(contact.url).protocol, 'https:', filename);
    }
    continue;
  }
  assert.ok(form.name && form.description && form.body?.length, `${filename}: missing form metadata or fields`);
  const ids = new Set();
  for (const field of form.body) {
    assert.ok(['markdown', 'textarea', 'input', 'dropdown', 'checkboxes'].includes(field.type), `${filename}: unsupported field ${field.type}`);
    if (field.type === 'markdown') continue;
    assert.match(field.id ?? '', /^[a-zA-Z0-9_-]+$/, `${filename}: invalid field id`);
    assert.ok(!ids.has(field.id), `${filename}: duplicate field id ${field.id}`);
    ids.add(field.id);
    assert.ok(field.attributes?.label, `${filename}: missing label for ${field.id}`);
    if (field.type === 'dropdown' || field.type === 'checkboxes') assert.ok(field.attributes.options?.length, `${filename}: missing options`);
    if (field.validations?.required != null) assert.equal(typeof field.validations.required, 'boolean', filename);
  }
}
for (const filename of ['.github/CODEOWNERS', '.github/pull_request_template.md']) {
  assert.ok(readFileSync(join(root, filename), 'utf8').trim(), `${filename}: empty file`);
}
console.log(`Repository links (${links} local targets), assets and issue forms are valid.`);
