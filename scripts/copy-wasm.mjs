import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const profile = process.argv[2] === 'release' ? 'release' : 'debug';
const src = resolve(__dirname, `../target/wasm32-unknown-unknown/${profile}/wasm_audio.wasm`);
const dst = resolve(__dirname, '../web/src/lib/wasm/wasm_audio.wasm');
mkdirSync(dirname(dst), { recursive: true });

function resolveWasmOpt() {
  const isWindows = process.platform === 'win32';
  const candidates = isWindows
    ? [
        resolve(__dirname, '../node_modules/.bin/wasm-opt.cmd'),
        resolve(__dirname, '../node_modules/.bin/wasm-opt'),
      ]
    : [
        resolve(__dirname, '../node_modules/.bin/wasm-opt'),
        resolve(__dirname, '../node_modules/.bin/wasm-opt.cmd'),
      ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

const wasmOptBin = resolveWasmOpt();

if (profile === 'release' && wasmOptBin) {
  const beforeSize = statSync(src).size;
  const isCmd = wasmOptBin.toLowerCase().endsWith('.cmd');
  execFileSync(wasmOptBin, ['-O3', '--strip-debug', src, '-o', dst], {
    stdio: 'inherit',
    shell: isCmd,
  });
  const afterSize = statSync(dst).size;
  const reduction = ((1 - afterSize / beforeSize) * 100).toFixed(1);
  console.log(
    `[copy-wasm] wasm-opt -O3 applied: ${beforeSize} -> ${afterSize} bytes (${reduction}% reduction)`,
  );
  console.log(`[copy-wasm] ${src} -> ${dst}`);
} else {
  copyFileSync(src, dst);
  if (profile === 'release' && !wasmOptBin) {
    console.warn(
      '[copy-wasm] wasm-opt not found in node_modules/.bin, install binaryen as devDependency',
    );
  }
  console.log(`copied ${src} -> ${dst}`);
}
