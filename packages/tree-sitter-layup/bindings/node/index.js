import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import nodeGypBuild from 'node-gyp-build';

const root = fileURLToPath(new URL('../..', import.meta.url));
const binding = nodeGypBuild(root);
binding.nodeTypeInfo = JSON.parse(readFileSync(`${root}/src/node-types.json`, 'utf8'));
binding.HIGHLIGHTS_QUERY = readFileSync(`${root}/queries/highlights.scm`, 'utf8');
binding.FOLDS_QUERY = readFileSync(`${root}/queries/folds.scm`, 'utf8');

export default binding;
