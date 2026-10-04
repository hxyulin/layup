import { readFileSync } from 'node:fs';

const changelog = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8');
const section = changelog.split(/^## /m).find(section => /^\d+\.\d+\.\d+/.test(section));
if (!section) throw new Error('No versioned release section in CHANGELOG.md');
const version = section.match(/^(\d+\.\d+\.\d+)/)[1];
const notes = section.slice(section.indexOf('\n') + 1).trim();
if (process.env.RELEASE_TAG && process.env.RELEASE_TAG !== `v${version}`) {
  throw new Error(`Release tag ${process.env.RELEASE_TAG} does not match changelog ${version}`);
}
console.log(`Install the CLI from crates.io:\n\n\`\`\`sh\ncargo install layup-cli --version ${version} --locked\n\`\`\`\n\nOr download a prebuilt binary with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):\n\n\`\`\`sh\ncargo binstall layup-cli --version ${version}\n\`\`\`\n\nThe command is named \`layup\`. Rust applications can use \`layup = "${version}"\`.\n\nInstall the JavaScript/WASM package from npm:\n\n\`\`\`sh\npnpm add @hxyulin/layup@${version}\n# or: npm install @hxyulin/layup@${version}\n\`\`\`\n\nThe npm package includes the compiled WASM engine; installing it does not require Rust.\n\nPrebuilt binaries: Linux x86_64/ARM64 (glibc 2.35+), macOS Intel/Apple Silicon, and Windows x86_64. Archives include code and font licenses; SHA256SUMS contains their checksums.\n\n[Documentation](https://hxyulin.github.io/layup/) · [Live playground](https://hxyulin.github.io/layup/playground.html)\n\n${notes}\n`);
