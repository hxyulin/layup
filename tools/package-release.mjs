import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== '--target' || !/^[a-zA-Z0-9_-]+$/.test(args[1])) {
  throw new Error('Usage: node tools/package-release.mjs --target <Rust target triple>');
}
const target = args[1];
const metadata = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], { cwd: root, encoding: 'utf8' });
if (metadata.error) throw metadata.error;
if (metadata.status !== 0) throw new Error(metadata.stderr);
const workspace = JSON.parse(metadata.stdout);
const cli = workspace.packages.find(pkg => pkg.name === 'layup-cli');
if (process.env.GITHUB_REF_TYPE === 'tag' && process.env.GITHUB_REF_NAME !== `v${cli.version}`) {
  throw new Error(`Release tag ${process.env.GITHUB_REF_NAME} does not match CLI version ${cli.version}`);
}
const name = `${cli.name}-${cli.version}-${target}`;
const output = join(root, 'out/release');
const stage = join(output, 'stage', name);
const binary = target.includes('windows') ? 'layup.exe' : 'layup';
rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });
copyFileSync(join(workspace.target_directory, target, 'release', binary), join(stage, binary));
for (const [source, destination] of [
  ['crates/layup-cli/README.md', 'README.md'],
  ['LICENSE-MIT', 'LICENSE-MIT'],
  ['LICENSE-APACHE', 'LICENSE-APACHE'],
  ['crates/layup/fonts/OFL.txt', 'OFL.txt'],
  ['crates/layup/fonts/README.md', 'FONT-NOTES.md'],
]) copyFileSync(join(root, source), join(stage, destination));
const archive = join(output, `${name}.tar.gz`);
// GNU tar interprets Windows drive-letter paths as remote archive locations.
// Relative arguments work with both GNU tar and bsdtar on every runner.
const result = spawnSync('tar', ['-czf', `${name}.tar.gz`, '-C', 'stage', name], { cwd: output, stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) throw new Error(`Packaging ${name} failed`);
console.log(`Created ${archive}`);
