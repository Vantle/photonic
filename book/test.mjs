import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInThisContext } from 'node:vm';

for (const file of process.argv.slice(2)) runInThisContext(await readFile(file, 'utf8'), { filename: file });
const { graph, pattern, record } = globalThis.book;

const parallel = graph.model(record.workbench.Parallel.result.execution);
const shown = (text, data = parallel) => [...pattern.state(pattern.read(text), data).state].sort((left, right) => left - right);
assert.deepEqual(shown('C'), [1, 3, 4]);
assert.deepEqual(shown('C, D'), [3, 4]);
assert.deepEqual(shown('C.D'), []);
assert.ok(shown('[C, D] E').length);
for (const text of ['[C,D] E', '[D, C]E', ' [ D ,C ] E ']) assert.deepEqual(shown(text), shown('[C, D] E'), text);
assert.deepEqual(shown('[C, D] F'), []);
const dynamic = graph.model(record.example.dynamic.result.execution);
assert.ok(shown('([A] B)', dynamic).length);
assert.deepEqual(shown('([A]B)', dynamic), shown('([A] B)', dynamic));
assert.equal(pattern.read('  '), undefined);
for (const [text, message] of [
    ['A B', /dot or a comma/],
    ['B.', /single dots/],
    ['A,', /is empty/],
    ['(Seed).A', /rule value/],
    ['[A', /balance/],
    ['A)', /balance/],
    ['A, [B] C', /Start a rule pattern/],
]) assert.throws(() => pattern.read(text), message, text);
console.log('Patterns match coherences, rule values and rules by structure, whatever the spacing or order.');

for (const [name, entry] of Object.entries(record.example)) {
    if (!entry.result.execution) continue;
    const data = graph.model(entry.result.execution);
    for (const state of data.state) {
        const path = graph.route(data, state.id);
        path.forEach((event, index) => assert.equal(event.source, index ? path[index - 1].target : 0, `${name} s${state.id}`));
        assert.equal(path.at(-1)?.target ?? 0, state.id, `${name} s${state.id}`);
        if (path.some(event => !event.direct)) assert.ok(!data.tree[0].has(state.id), `${name} s${state.id} has a direct route`);
    }
}
console.log('Every recorded configuration has a route from the start, direct whenever one exists.');
