import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';

const [output, manifest, ...pair] = process.argv.slice(2);
const path = new Map();
for (let index = 0; index < pair.length; index += 2) path.set(pair[index], pair[index + 1]);

const library = Object.create(null);
for (const { name, load } of JSON.parse(await readFile(manifest, 'utf8'))) {
    assert.equal(load.at(-1), name, `${name} must hold one file named after it, loaded after its dependencies`);
    library[name] = { package: name.split('/').at(-2), load, source: await readFile(path.get(name), 'utf8') };
}
for (const [name, entry] of Object.entries(library)) {
    for (const part of entry.load) assert.ok(part in library, `${name} loads ${part}, which the Lightbox must offer as well`);
}

const text = Object.entries(library).map(([name, entry]) => `${JSON.stringify(name)}: ${JSON.stringify(entry)}`).join(',\n');
await writeFile(output, `globalThis.book ??= {};\nglobalThis.book.library = {\n__proto__: null,\n${text}\n};\n`);
