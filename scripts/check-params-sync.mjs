import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { generateRustSource, generateTsSource } from './gen-params.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const paramsJsonPath = resolve(root, 'params.json');
const rustPath = resolve(root, 'crates/dsp-core/src/params.rs');
const tsPath = resolve(root, 'web/src/lib/audio/generated/params.ts');

const paramsJson = JSON.parse(readFileSync(paramsJsonPath, 'utf8'));
const expectedRust = generateRustSource(paramsJson);
const expectedTs = generateTsSource(paramsJson);

const actualRust = readFileSync(rustPath, 'utf8');
const actualTs = readFileSync(tsPath, 'utf8');

let drift = false;
if (actualRust !== expectedRust) {
	console.error(`params.rs is out of sync with params.json. Run \`pnpm gen:params\`.`);
	console.error(`  expected: ${rustPath}`);
	drift = true;
}
if (actualTs !== expectedTs) {
	console.error(`generated/params.ts is out of sync with params.json. Run \`pnpm gen:params\`.`);
	console.error(`  expected: ${tsPath}`);
	drift = true;
}
if (drift) {
	process.exit(1);
}
console.log('ParamDescriptor sync OK.');
