import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { test } from 'node:test';
import Parser from 'tree-sitter';
import Layup from './index.js';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
function parser() {
  const value = new Parser();
  value.setLanguage(Layup);
  return value;
}
function fixtures(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const filename = path.join(directory, entry.name);
    return entry.name === 'node_modules' ? [] : entry.isDirectory() ? fixtures(filename) : filename.endsWith('.layup') ? [filename] : [];
  });
}

for (const filename of [
  ...fixtures(path.join(root, 'examples')).filter(name => !name.includes('/node_modules/')),
  ...fixtures(path.join(root, 'docs/diagrams')),
  ...fixtures(path.join(root, 'docs/checkpoints')),
]) {
  test(`parses ${path.relative(root, filename)}`, () => {
    const tree = parser().parse(readFileSync(filename, 'utf8'));
    assert.equal(tree.rootNode.hasError, false, tree.rootNode.toString());
  });
}

test('highlight and folding queries compile and distinguish contextual names', () => {
  const text = '@company.analysis(value={public: true})\ndiagram graph type=graph {\n  node node "Literal // /* */" fill-color="#fff"\n  node -> api\n  node store "Storage" style=service\n}';
  const tree = parser().parse(text);
  assert.equal(tree.rootNode.hasError, false);
  const captures = new Parser.Query(Layup, Layup.HIGHLIGHTS_QUERY).captures(tree.rootNode);
  const names = (type, text) => captures.filter(c => c.node.type === type && c.node.text === text).map(c => c.name);
  assert.equal(names('declaration_keyword', 'node').at(-1), 'keyword');
  assert.equal(names('identifier', 'node').at(-1), 'variable');
  assert.equal(names('identifier', 'company').at(-1), 'attribute');
  assert.equal(names('identifier', 'fill-color').at(-1), 'property');
  assert.equal(names('identifier', 'public').at(-1), 'property');
  assert.equal(names('identifier', 'service').at(-1), 'constant');
  assert.ok(names('string', '"Literal // /* */"').includes('string'));
  assert.equal(tree.rootNode.descendantsOfType(['line_comment', 'block_comment']).length, 0);
  const folds = new Parser.Query(Layup, Layup.FOLDS_QUERY).captures(tree.rootNode);
  assert.ok(folds.some(c => c.node.type === 'block'));
  assert.ok(folds.some(c => c.node.type === 'record'));
});

test('incremental edits agree with a fresh parse, including nested comments', () => {
  const original = 'diagram g type=graph {\n  /* outer /* nested */ end */\n  node a palette=blue\n  a -> b\n}';
  const value = parser();
  const old = value.parse(original);
  const startIndex = original.indexOf('nested');
  const insertion = '/* deeper */ ';
  const rowStart = original.lastIndexOf('\n', startIndex) + 1;
  old.edit({ startIndex, oldEndIndex: startIndex, newEndIndex: startIndex + insertion.length,
    startPosition: { row: 1, column: startIndex - rowStart },
    oldEndPosition: { row: 1, column: startIndex - rowStart },
    newEndPosition: { row: 1, column: startIndex - rowStart + insertion.length } });
  const updated = original.slice(0, startIndex) + insertion + original.slice(startIndex);
  const incremental = value.parse(updated, old);
  const fresh = parser().parse(updated);
  assert.equal(incremental.rootNode.hasError, false);
  assert.equal(incremental.rootNode.toString(), fresh.rootNode.toString());
  assert.equal(incremental.rootNode.descendantsOfType('block_comment')[0].text, '/* outer /* /* deeper */ nested */ end */');
});

test('unfinished attributes recover at the next declaration', () => {
  const tree = parser().parse('diagram g type=graph {\n  node a fill-color=\n  node b "B"\n  b -> c\n}');
  assert.equal(tree.rootNode.hasError, true);
  assert.ok(tree.rootNode.descendantsOfType('declaration').some(node => node.text === 'node b "B"'));
  assert.ok(tree.rootNode.descendantsOfType('connection').some(node => node.text === 'b -> c'));
});

test('unknown diagrams and annotations retain later known diagrams', () => {
  const tree = parser().parse('@vendor.custom(data={x: [1, false, null]})\ndiagram future type=vendor-format {\n  widget a custom-property=enabled\n}\ndiagram known type=graph { node a "A" }');
  assert.equal(tree.rootNode.hasError, false);
  assert.equal(tree.rootNode.namedChildren.filter(node => node.type === 'declaration').length, 2);
  assert.equal(tree.rootNode.descendantsOfType('annotation_name')[0].text, 'vendor.custom');
});

test('unterminated strings and block comments report errors', () => {
  for (const text of ['node a "unfinished', '/* unfinished /* nested */']) {
    assert.equal(parser().parse(text).rootNode.hasError, true, text);
  }
});


test('Unicode spans use Node UTF-16 positions and CRLF remains a separator', () => {
  const text = 'diagram 图 type=graph {\r\n  node 𠮷 "مرحبا /* literal */"\r\n  𠮷 -> 图\r\n}';
  const tree = parser().parse(text);
  assert.equal(tree.rootNode.hasError, false);
  const id = tree.rootNode.descendantsOfType('identifier').find(node => node.text === '𠮷');
  assert.equal(id.startIndex, text.indexOf('𠮷'));
  assert.equal(id.endIndex, id.startIndex + '𠮷'.length);
  assert.deepEqual(id.startPosition, { row: 1, column: 7 });
});

test('leading comment markers in code strings stay content across edits', () => {
  const value = parser();
  const original = 'node a { code "/* literal */\n// literal" }';
  const old = value.parse(original);
  const startIndex = original.indexOf('/*');
  old.edit({ startIndex, oldEndIndex: startIndex + 2, newEndIndex: startIndex + 2,
    startPosition: { row: 0, column: startIndex },
    oldEndPosition: { row: 0, column: startIndex + 2 },
    newEndPosition: { row: 0, column: startIndex + 2 } });
  const updated = original.slice(0, startIndex) + '//' + original.slice(startIndex + 2);
  const tree = value.parse(updated, old);
  assert.equal(tree.rootNode.hasError, false);
  assert.equal(tree.rootNode.toString(), parser().parse(updated).rootNode.toString());
  assert.equal(tree.rootNode.descendantsOfType(['line_comment', 'block_comment']).length, 0);
});

const conformance = JSON.parse(readFileSync(path.join(root, 'crates/layup/tests/fixtures/language/conformance.json'), 'utf8'));
for (const fixture of conformance.filter(fixture => fixture.valid && !fixture.opaque)) {
  test(`shared language: ${fixture.name}`, () => {
    const tree = parser().parse(fixture.source);
    assert.equal(tree.rootNode.hasError, false, tree.rootNode.toString());
  });
}
