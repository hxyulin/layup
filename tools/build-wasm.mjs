import { spawnSync } from 'node:child_process';
import { copyFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const result = spawnSync('cargo', ['build', '-p', 'layup-wasm', '--profile', 'wasm', '--target', 'wasm32-unknown-unknown'], { cwd: root, stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
copyFileSync(new URL('../target/wasm32-unknown-unknown/wasm/layup_wasm.wasm', import.meta.url), new URL('../packages/layup/layup.wasm', import.meta.url));
copyFileSync(new URL('../crates/layup/src/presentation.js', import.meta.url), new URL('../packages/layup/presentation.js', import.meta.url));
