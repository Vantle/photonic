import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInThisContext } from 'node:vm';

for (const name of ['state', 'value', 'gate', 'binding', 'flow', 'model', 'support', 'example', 'dynamic', 'fixture', 'capture', 'test']) {
    const filename = new URL(`../document/kernel/${name}.js`, import.meta.url);
    runInThisContext(await readFile(filename, 'utf8'), { filename: filename.pathname });
}
const result = kernel.verify();
assert.ok(result.checked > 0);
assert.deepEqual(result.failure, []);
assert.deepEqual(kernel.fixture(), JSON.parse(await readFile(new URL('../example/reference.json', import.meta.url), 'utf8')));
console.log(`${result.checked} reference assertions passed; generated fixture matches the Rust oracle.`);
