import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decodeShare, encodeShare } from '../.vitepress/theme/share.js';

test('share payloads round-trip international source and view selection', () => {
  const source = 'model "中文 / العربية / 😀" { // comment\n node 客户端 "客户端"\n view 详情 { include 客户端 }\n}';
  const hash = `#${new URLSearchParams({ diagram: encodeShare(source, '详情') })}`;
  assert.deepEqual(decodeShare(hash), { version: 1, source, view: '详情' });
  assert.equal(decodeShare('#heading'), null);
});

test('malformed or incompatible shared data fails explicitly', () => {
  for (const hash of ['#diagram=!', '#diagram=' + btoa('{}'), '#diagram=' + btoa('{"version":2,"source":"node a"}'), '#diagram=' + 'a'.repeat(150001)]) {
    assert.throws(() => decodeShare(hash));
  }
});
