import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const profile = process.argv[2] === 'release' ? 'release' : 'debug';
const src = resolve(__dirname, `../target/wasm32-unknown-unknown/${profile}/wasm_audio.wasm`);
const dst = resolve(__dirname, '../web/src/lib/wasm/wasm_audio.wasm');
mkdirSync(dirname(dst), { recursive: true });
copyFileSync(src, dst);
console.log(`copied ${src} -> ${dst}`);
