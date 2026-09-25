import assert from 'node:assert/strict';
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';

const [output, repository, ...pair] = process.argv.slice(2);
const file = new Map();
for (let index = 0; index < pair.length; index += 2) file.set(pair[index + 1], pair[index]);
for (const required of ['index.html', 'book/worker.js', 'toolchain/browser/module/runtime.js', 'toolchain/browser/module/runtime_bg.wasm']) {
    assert.ok(file.has(required), `the site needs ${required}`);
}

const page = (await readFile(file.get('index.html'), 'utf8')).replace(/\b(href|src)="([^"#:]+)((?:#[^"]*)?)"/g, (link, attribute, path, anchor) => file.has(path)
    ? link
    : `${attribute}="${repository}/${path.endsWith('/') ? 'tree' : 'blob'}/main/${path}${anchor}"`);

for (const [destination, source] of file) {
    await mkdir(dirname(join(output, destination)), { recursive: true });
    if (destination === 'index.html') await writeFile(join(output, destination), page);
    else await copyFile(source, join(output, destination));
}
