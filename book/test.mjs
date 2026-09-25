import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInThisContext } from 'node:vm';

const worker = [];
let probe;
globalThis.location = { protocol: 'http:' };
globalThis.requestIdleCallback = callback => { probe = callback; };
globalThis.Worker = class {
    constructor() {
        this.message = [];
        this.terminated = false;
        worker.push(this);
    }

    postMessage(data) {
        this.message.push(data);
    }

    terminate() {
        this.terminated = true;
    }
};
for (const file of process.argv.slice(2)) runInThisContext(await readFile(file, 'utf8'), { filename: file });
const { editor, engine, graph, pattern, record, render, symmetry } = globalThis.book;

const parallel = graph.model(record.workbench.Parallel.result.execution);
const shown = (text, data = parallel) => [...pattern.state(pattern.read(text), data).state].sort((left, right) => left - right);
assert.deepEqual(shown('C'), [1, 3, 4]);
assert.deepEqual(shown('C, D'), [3, 4]);
assert.deepEqual(shown('C.D'), []);
assert.ok(shown('[C, D] E').length);
for (const text of ['[C,D] E', '[D, C]E', ' [ D ,C ] E ', 'E [C, D]', 'E[D,C]']) assert.deepEqual(shown(text), shown('[C, D] E'), text);
assert.deepEqual(shown('[C, D] F'), []);
const dynamic = graph.model(record.example.dynamic.result.execution);
assert.ok(shown('([A] B)', dynamic).length);
assert.deepEqual(shown('([A]B)', dynamic), shown('([A] B)', dynamic));
assert.equal(pattern.read('  '), undefined);
const bracketed = graph.model({ definition: [], closed: true, work: 0, state: [{ id: 0, world: [{ frame: 0, particle: [{ kind: 'atom', id: 0, label: '⟨x⟩' }] }], frame: [{ parent: null, particle: [], held: [] }] }], event: [] });
assert.deepEqual(shown('⟨x⟩', bracketed), [0]);
for (const [text, message] of [
    ['A B', /dot or a comma/],
    ['B.', /single dots/],
    ['A,', /is empty/],
    ['(Seed).A', /rule value/],
    ['[A', /balance/],
    ['A)', /balance/],
    ['A, [B] C', /one rule/],
]) assert.throws(() => pattern.read(text), message, text);
console.log('Patterns match coherences, rule values and rules by structure, whatever the spacing or order.');

for (const [name, entry] of Object.entries(record.example)) {
    if (!entry.result.execution) continue;
    const data = graph.model(entry.result.execution);
    for (const state of data.state) {
        const path = graph.route(data, state.id);
        path.forEach((event, index) => assert.equal(event.source, index ? path[index - 1].target : 0, `${name} s${state.id}`));
        assert.equal(path.at(-1)?.target ?? 0, state.id, `${name} s${state.id}`);
        if (path.some(event => event.deduction.length)) assert.ok(!data.tree[0].has(state.id), `${name} s${state.id} has a direct route`);
    }
    for (const event of data.event) {
        event.deduction.forEach((index, position) => assert.equal(data.event[index].source, position ? data.event[event.deduction[position - 1]].target : event.source, `${name} event ${event.id}`));
    }
}
console.log('Every recorded configuration has a route from the start, direct whenever one exists.');

const conjunction = graph.model(record.example.conjunction.result.execution);
const abstraction = conjunction.event.find(event => event.source === 0 && event.rule.startsWith('[And.Boolean.Boolean]') && event.deduction.length);
assert.deepEqual(abstraction.deduction.map(index => conjunction.event[index].rule).sort(), ['[False] Boolean', '[True] Boolean']);
console.log('Every deduction is a path from the configuration where its event happens, and And deduces both operands as Boolean.');

for (const [part, text] of [
    [[['True', 'False'], ['False', 'True']], 'True ⇄ False'],
    [[['Up', 'Less'], ['Down', 'Greater']], 'Up → Down, Less → Greater'],
    [[['A', 'B'], ['B', 'C']], 'A → B → C'],
    [[['A', 'B', 'C'], ['B', 'C', 'A']], '(A B C)'],
    [[['Zero'], ['One'], ['Two']], 'Zero / One / Two'],
]) assert.equal(symmetry.describe({ kind: 'local', part }), text);
assert.equal(symmetry.describe({ kind: 'global', part: [['A', 'B'], ['X', 'Y']] }), 'A and B; X and Y');
assert.equal(symmetry.describe({ kind: 'block', part: [['Boolean', 'Not']] }), 'Boolean.Not');
assert.equal(symmetry.describe(record.example.light.result.symmetry.class[0]), 'Red, Green and Blue');
console.log('Symmetries read as exchanges, renamings, copies and blocks.');

assert.deepEqual([render.count(0, 'event'), render.count(1, 'rule'), render.count(2, 'configuration')], ['0 events', '1 rule', '2 configurations']);
console.log('Counts agree with their numbers.');

const refusal = (message, detail) => Object.assign(new Error(message), { detail });
assert.equal(editor.describe(refusal('The target has no particle.', { code: 'target', target: 3 })), 'Target 4: The target has no particle.');
assert.equal(editor.describe(refusal('expected a particle', { code: 'target', target: 1, span: { offset: 2, length: 1 } })), 'Target 2: expected a particle (at character 3)');
assert.equal(editor.describe(refusal('expected a particle', { code: 'source', span: { offset: 0, length: 0 } })), 'expected a particle (at character 1)');
assert.equal(editor.describe(new Error('Stopped.')), 'Stopped.');
console.log('Errors name their target and character whenever the engine reports them.');

const settled = [];
const track = (name, promise) => promise.then(value => settled.push([name, value.source]), error => settled.push([name, error.message]));
const flush = () => new Promise(resolve => setTimeout(resolve, 0));
const posted = fake => fake.message.map(value => value.request.source);
const answer = (fake, body) => {
    const { serial, request } = fake.message.at(-1);
    fake.onmessage({ data: { serial, reply: { version: request.version, source: request.source, ...body } } });
};
const channel = engine.open();
const first = new AbortController();
const second = new AbortController();
track('first', channel.send('explore', { source: 'A' }, { signal: first.signal }));
track('second', channel.send('explore', { source: 'B' }, { signal: second.signal }));
assert.equal(worker[0].message[0].request.version, 2);
assert.deepEqual(posted(worker[0]), ['A']);
second.abort();
await flush();
assert.deepEqual(settled, [['second', 'Stopped.']]);
assert.equal(worker[0].terminated, false);
answer(worker[0]);
await flush();
first.abort();
await flush();
assert.deepEqual(settled, [['second', 'Stopped.'], ['first', 'A']]);
assert.deepEqual(posted(worker[0]), ['A']);
assert.equal(engine.state, 'live');
console.log('Stopping a queued request withdraws only that request, which never reaches the engine.');

const third = new AbortController();
track('third', channel.send('explore', { source: 'C' }, { signal: third.signal }));
track('fourth', channel.send('explore', { source: 'D' }));
third.abort();
await flush();
assert.equal(worker[0].terminated, true);
assert.deepEqual(posted(worker[1]), ['D']);
answer(worker[0]);
answer(worker[1]);
await flush();
assert.deepEqual(settled.slice(2), [['third', 'Stopped.'], ['fourth', 'D']]);
track('slow', channel.send('explore', { source: 'E' }, { timeout: 5 }));
track('patient', channel.send('explore', { source: 'F' }));
await new Promise(resolve => setTimeout(resolve, 40));
assert.equal(worker[1].terminated, true);
assert.deepEqual(posted(worker[2]), ['F']);
worker[2].onmessage({ data: { serial: worker[2].message[0].serial, failure: { code: 'crash', message: 'unreachable' } } });
await flush();
assert.deepEqual(settled.slice(4).map(([name, message]) => [name, message.split(',')[0]]), [['slow', 'Stopped after 0.005 seconds without a result.'], ['patient', 'The engine stopped with unreachable']]);
assert.equal(worker[2].terminated, true);
console.log('Stopping, timing out or crashing the running request restarts the engine and resends the rest.');

track('stale', channel.send('lower', { source: 'G' }));
track('waiting', channel.send('lower', { source: 'H' }));
const [{ serial }] = worker[3].message;
worker[3].onmessage({ data: { serial, reply: { version: 1, program: {} } } });
await flush();
assert.deepEqual(settled.slice(6).map(([name, message]) => [name, /Reload the page/.test(message)]), [['stale', true], ['waiting', true]]);
assert.equal(worker[3].terminated, true);
assert.equal(engine.state, 'stale');
const legacy = engine.open();
track('legacy', legacy.send('lower', { source: 'I' }));
worker[4].onmessage({ data: { serial: worker[4].message[0].serial, version: 2, program: {} } });
await flush();
assert.match(settled.at(-1)[1], /Reload the page/);
probe();
worker[5].onmessage({ data: { serial: worker[5].message[0].serial, reply: { version: 1, program: {} } } });
await flush();
assert.equal(engine.state, 'stale');
assert.deepEqual(settled.map(([name]) => name).sort(), ['first', 'fourth', 'legacy', 'patient', 'second', 'slow', 'stale', 'third', 'waiting']);
console.log('A reply from another engine version rejects every waiting request and asks for a reload, and each request settles once.');

