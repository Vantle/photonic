import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { runInThisContext } from 'node:vm';

const [javascript, webassembly, table, numeral, ...script] = process.argv.slice(2);
const runtime = await import(pathToFileURL(javascript));
runtime.initSync({ module: await readFile(webassembly) });
const { version } = JSON.parse(runtime.lower('{}'));
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
for (const file of script) runInThisContext(await readFile(file, 'utf8'), { filename: file });
const { editor, engine, graph, pattern, record, render, share, symmetry } = globalThis.book;

const select = (text, execution) => JSON.parse(runtime.select(JSON.stringify({ version, pattern: text, execution })));
const selected = (text, execution) => {
    const reply = select(text, execution);
    assert.equal(reply.error, undefined, `${text}: ${reply.error?.message}`);
    return reply;
};
const parallel = record.workbench.Parallel.result.execution;
const shown = (text, execution = parallel) => [...pattern.state(selected(text, execution), graph.model(execution)).state].sort((left, right) => left - right);
assert.deepEqual(shown('C'), [1, 3, 4]);
assert.deepEqual(shown('C, D'), [3, 4]);
assert.deepEqual(shown('C.D'), []);
assert.deepEqual(shown('C,'), shown('C'));
assert.ok(shown('[C, D] E').length);
for (const text of ['[C,D] E', '[D, C]E', ' [ D ,C ] E ', 'E [C, D]', 'E[D,C]', '[(C), D] (E)']) assert.deepEqual(shown(text), shown('[C, D] E'), text);
assert.deepEqual(shown('[C, D] F'), []);
assert.deepEqual(shown('[A] C, [B] D'), [0, 1, 2, 3, 4]);
const dynamic = record.example.dynamic.result.execution;
assert.ok(shown('().([A] B)', dynamic).length);
assert.deepEqual(shown('().([A]B)', dynamic), shown('().([A] B)', dynamic));
assert.deepEqual(shown('(Seed).A', dynamic), shown('Seed.A', dynamic));
const bracketed = { definition: [], closed: true, work: 0, state: [{ id: 0, world: [{ frame: 0, particle: [{ kind: 'atom', id: 0, label: '⟨x⟩' }] }], frame: [{ parent: null, particle: [], held: [] }] }], event: [] };
assert.deepEqual(shown('⟨x⟩', bracketed), [0]);
const brew = record.example.brew.result.execution;
for (const text of ['(Kettle, [Kettle.Tea] Cup)', '([Kettle.Tea] Cup, Kettle.Tea)']) assert.deepEqual(selected(text, brew).state.map(found => found.id), [1], text);
assert.deepEqual(selected('([A] B)', brew).state, []);
assert.deepEqual(selected('Tea', brew).lane.map(entry => entry.state).sort(), [0, 1]);
const refusal = text => {
    const reply = select(text, parallel);
    assert.ok(reply.error, `${text} must be refused`);
    return Object.assign(new Error(reply.error.message), { detail: reply.error });
};
for (const [text, message, span] of [
    ['A B', /put a dot between these to join them, or a comma to separate them$/, { offset: 2, length: 1 }],
    ['[A.] B', /a dot joins two things/, { offset: 3, length: 1 }],
    ['B.', /a dot joins two things/, { offset: 2, length: 0 }],
    ['A)', /closes nothing/, { offset: 1, length: 1 }],
    ['A,,B', /a comma separates two things/, { offset: 2, length: 1 }],
]) {
    const error = refusal(text);
    assert.match(error.message, message, text);
    assert.deepEqual(error.detail.span, span, text);
}
assert.match(refusal('A, [B] C').message, /not both/);
assert.match(refusal('(K, [K] L), [B] C').message, /not both/);
assert.match(editor.describe(refusal('[A..X] B')), /\(at character 4\)$/);
console.log('The engine reads patterns as programs and matches coherences, scopes, rule values and rules by containment, whatever the spacing or order.');

const listed = JSON.parse(await readFile(table, 'utf8'));
for (const entry of listed) {
    const { execution } = JSON.parse(runtime.explore(JSON.stringify({ version, source: entry.program })));
    const reply = select(entry.pattern, execution);
    if (entry.error) {
        assert.equal(reply.error?.code, entry.error, entry.pattern);
        continue;
    }
    assert.ok(execution.closed, entry.program);
    const total = reply.kind === 'configuration' ? reply.state.length : reply.event.length;
    assert.deepEqual({ kind: reply.kind, total }, { kind: entry.kind, total: entry.total }, `${entry.pattern} on ${entry.program}`);
}
console.log(`The book selects what Spectrum selects in all ${listed.length} shared pattern cases.`);

const { decode } = await import(pathToFileURL(numeral));
const evaluate = input => {
    const path = runtime.Path.expression(JSON.stringify({ version, source: input }));
    const result = JSON.parse(path.run());
    path.free();
    return result;
};
for (const [input, expected] of [
    ['12 + 2', '21'], ['12+2', '21'], ['12 * 2', '101'], ['21 / 2 - 1', '2'],
    ['-(12 + 2) * 10', '-210'], ['-21 / 2', '-10'],
    ['1212 * 10 / 2 + 11 - 1', '2220'], ['0', '0'], ['00012', '12'],
    ['2 + 1 * 2', '11'], ['(2 + 1) * 2', '20'], ['--2', '2'], ['-(12+2)*2', '-112'], ['(-2)*(-2)', '11'], ['(-2)*0', '0'],
]) {
    const result = evaluate(input);
    assert.equal(result.error, undefined, input);
    assert.equal(decode(result.state, result.definition).ternary, expected, input);
}
for (const input of ['', '1+', '(1', '1**2', '1/0']) {
    const result = evaluate(input);
    assert.throws(() => decode(result.state, result.definition), input === '1/0' ? /Division by zero/ : /syntax/, input);
}
console.log('Numerals decode what the engine’s expressions compute, including signed arithmetic, syntax errors and division by zero.');

const start = performance.now();
const repeated = evaluate('2*2*2*2*2*2*2*2*2*2');
console.log(JSON.stringify({ repeated: { elapsed: performance.now() - start, event: repeated.event, work: repeated.work, state: repeated.state?.id } }));
assert.equal(decode(repeated.state, repeated.definition).ternary, '1101221');

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

const located = (message, detail) => Object.assign(new Error(message), { detail });
assert.equal(editor.describe(located('The target has no particle.', { code: 'target', target: 3 })), 'Target 4: The target has no particle.');
assert.equal(editor.describe(located('expected a particle', { code: 'target', target: 1, span: { offset: 2, length: 1 } })), 'Target 2: expected a particle (at character 3)');
assert.equal(editor.describe(located('expected a particle', { code: 'source', span: { offset: 0, length: 0 } })), 'expected a particle (at character 1)');
assert.equal(editor.describe(new Error('Stopped.')), 'Stopped.');
console.log('Errors name their target and character whenever the engine reports them.');

for (const program of [
    { source: 'A.X,\n[A] B', target: ['B.X, [A] B', 'C,\nD', '人.⟨x⟩', 'P&Q=R+S #1 %2 ?3'], library: ['library/boolean/not', 'library/function/invoke'], preserve: true },
    { source: '[A] B', target: ['B', ''], library: ['theorem/case'], preserve: false },
    { source: '', target: [], library: [], preserve: false },
]) {
    const link = share.link(program);
    assert.match(link, /^lightbox\.html\?source=/);
    assert.deepEqual(share.read(new URL(link, 'https://photonic.vantle.org/').search), program);
}
assert.equal(share.read('?target=A'), undefined);
console.log('Links carry the source, every target, every library and preserve through commas, newlines, Unicode and URL syntax.');

const announced = [];
engine.watch(state => announced.push(state));
const settled = [];
const track = (name, promise) => promise.then(value => settled.push([name, value.source]), error => settled.push([name, error.message]));
const pause = time => new Promise(resolve => setTimeout(resolve, time));
const posted = fake => fake.message.map(value => value.request.source);
const ready = fake => fake.onmessage({ data: { ready: true } });
const answer = (fake, body) => {
    const { serial, request } = fake.message.at(-1);
    fake.onmessage({ data: { serial, reply: { version: request.version, source: request.source, ...body } } });
};
const crash = fake => fake.onmessage({ data: { serial: fake.message.at(-1).serial, failure: { code: 'crash', message: 'unreachable' } } });

probe();
assert.deepEqual(worker[0].message.map(value => [value.kind, value.request]), [['lower', { version, source: 'A' }]]);
ready(worker[0]);
crash(worker[0]);
await pause(0);
assert.equal(engine.state, 'failed');
assert.equal(worker[0].terminated, true);
console.log('The page and its engine speak one version, and a probe that fails after the engine loads reports a failed engine rather than a local file.');

track('loading', engine.send('explore', { source: 'L' }, { timeout: 5 }));
assert.deepEqual(posted(worker[1]), ['L']);
await pause(40);
assert.deepEqual(settled, []);
assert.equal(worker[1].terminated, false);
ready(worker[1]);
await pause(40);
assert.deepEqual(settled, [['loading', 'Stopped after 0.005 seconds without a result.']]);
assert.equal(worker[1].terminated, true);
console.log('A request’s timeout starts only once its engine has loaded.');

const first = new AbortController();
const second = new AbortController();
track('first', engine.send('explore', { source: 'A' }, { signal: first.signal }));
track('second', engine.send('explore', { source: 'B' }, { signal: second.signal }));
ready(worker[2]);
assert.deepEqual(posted(worker[2]), ['A']);
second.abort();
await pause(0);
assert.deepEqual(settled.slice(1), [['second', 'Stopped.']]);
assert.equal(worker[2].terminated, false);
answer(worker[2]);
await pause(0);
first.abort();
await pause(0);
assert.deepEqual(settled.slice(1), [['second', 'Stopped.'], ['first', 'A']]);
assert.deepEqual(posted(worker[2]), ['A']);
assert.equal(engine.state, 'live');
console.log('Stopping a queued request withdraws only that request, which never reaches the engine.');

const third = new AbortController();
track('third', engine.send('explore', { source: 'C' }, { signal: third.signal }));
track('fourth', engine.send('explore', { source: 'D' }));
third.abort();
await pause(0);
assert.equal(worker[2].terminated, true);
assert.deepEqual(posted(worker[3]), ['D']);
ready(worker[3]);
answer(worker[2]);
answer(worker[3]);
await pause(0);
assert.deepEqual(settled.slice(3), [['third', 'Stopped.'], ['fourth', 'D']]);
track('slow', engine.send('explore', { source: 'E' }, { timeout: 5 }));
track('patient', engine.send('explore', { source: 'F' }));
await pause(40);
assert.equal(worker[3].terminated, true);
assert.deepEqual(posted(worker[4]), ['F']);
ready(worker[4]);
crash(worker[4]);
await pause(0);
assert.deepEqual(settled.slice(5).map(([name, message]) => [name, message.split(',')[0]]), [['slow', 'Stopped after 0.005 seconds without a result.'], ['patient', 'The engine stopped with unreachable']]);
assert.equal(worker[4].terminated, true);
console.log('Stopping, timing out or crashing the running request restarts the engine and resends the rest.');

const follow = engine.path();
const walk = follow('path', { source: 'P' }, { timeout: 60000 });
ready(worker[5]);
answer(worker[5], { outcome: 'reached', event: 2 });
const run = await walk;
assert.deepEqual([run.source, run.outcome, run.event], ['P', 'reached', 2]);
const step = run.inspect(1);
assert.deepEqual(worker[5].message.at(-1).request, { version, index: 1 });
worker[5].onmessage({ data: { serial: worker[5].message.at(-1).serial, reply: { version, event: { rule: '[A] B' } } } });
assert.deepEqual((await step).event, { rule: '[A] B' });
track('shared', engine.send('lower', { source: 'S' }));
assert.deepEqual(posted(worker[6]), ['S']);
assert.deepEqual(worker[5].message.map(value => value.kind), ['path', 'inspect']);
ready(worker[6]);
answer(worker[6]);
await pause(0);
console.log('A direct path owns its engine, and inspecting the path asks that engine.');

track('unloaded', engine.send('lower', { source: 'U' }));
worker[6].onerror({ preventDefault: () => {} });
await pause(0);
assert.deepEqual(settled.at(-1), ['unloaded', 'The WebAssembly engine did not load. Reload the page.']);
assert.equal(worker[6].terminated, true);
assert.equal(engine.state, 'failed');
track('recovered', engine.send('lower', { source: 'R' }));
ready(worker[7]);
answer(worker[7]);
await pause(0);
assert.equal(engine.state, 'live');
track('broken', engine.send('lower', { source: 'G' }));
track('behind', engine.send('lower', { source: 'H' }));
worker[7].onmessage({ data: { failure: { code: 'engine', message: 'no memory' } } });
await pause(0);
assert.deepEqual(settled.slice(-2), [['broken', 'The WebAssembly engine did not start: no memory'], ['behind', 'The WebAssembly engine did not start: no memory']]);
assert.equal(worker[7].terminated, true);
assert.equal(engine.state, 'failed');
console.log('An engine that cannot load or start rejects every waiting request and asks for a reload.');

track('stale', engine.send('lower', { source: 'I' }));
track('waiting', engine.send('lower', { source: 'J' }));
ready(worker[8]);
worker[8].onmessage({ data: { serial: worker[8].message[0].serial, reply: { version: version - 1, program: {} } } });
await pause(0);
assert.deepEqual(settled.slice(-2).map(([name, message]) => [name, /Reload the page/.test(message)]), [['stale', true], ['waiting', true]]);
assert.equal(worker[8].terminated, true);
assert.equal(engine.state, 'stale');
track('legacy', engine.send('lower', { source: 'K' }));
worker[9].onmessage({ data: { serial: worker[9].message[0].serial, version, program: {} } });
await pause(0);
assert.match(settled.at(-1)[1], /Reload the page/);
assert.deepEqual(announced, ['unknown', 'failed', 'live', 'failed', 'live', 'failed', 'stale']);
assert.deepEqual(settled.map(([name]) => name).sort(), ['behind', 'broken', 'first', 'fourth', 'legacy', 'loading', 'patient', 'recovered', 'second', 'shared', 'slow', 'stale', 'third', 'unloaded', 'waiting']);
console.log('A reply from another engine version rejects every waiting request and asks for a reload, and each request settles once.');
