#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { writeFile } from 'node:fs/promises';
import { analyzeCargo } from './cargo.js';

try {
  const { values } = parseArgs({ options: {
    'manifest-path': { type: 'string', default: 'Cargo.toml' },
    output: { type: 'string', short: 'o' },
    'include-external': { type: 'boolean' },
    'include-dev': { type: 'boolean' },
    'source-base-url': { type: 'string' },
    online: { type: 'boolean' },
    features: { type: 'string', multiple: true },
    'all-features': { type: 'boolean' },
    'no-default-features': { type: 'boolean' },
    help: { type: 'boolean', short: 'h' },
  } });
  if (values.help) {
    console.log(`Usage: layup-cargo [--manifest-path Cargo.toml] [-o graph.json]
  --include-external       Include direct external workspace dependencies
  --include-dev            Include development dependencies
  --source-base-url URL    Source root URL ending in /
  --features FEATURES      Enable Cargo features (repeatable)
  --all-features           Enable all Cargo features
  --no-default-features    Disable default Cargo features
  --online                 Allow Cargo network access (default: offline)

Runs cargo metadata --locked; emits structured graph JSON with overview
and per-package detail views. Does not build or execute the project.`);
  } else {
    const graph = await analyzeCargo({ manifestPath: values['manifest-path'], includeExternal: values['include-external'], includeDev: values['include-dev'], sourceBaseUrl: values['source-base-url'], offline: !values.online, features: values.features, allFeatures: values['all-features'], noDefaultFeatures: values['no-default-features'] });
    const json = JSON.stringify(graph, null, 2) + '\n';
    if (values.output && values.output !== '-') await writeFile(values.output, json);
    else process.stdout.write(json);
  }
} catch (error) {
  console.error(`layup-cargo: ${error.message}`);
  process.exitCode = 1;
}
