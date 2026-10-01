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
const { editor, engine, euclid, figure, graph, library, notation, pattern, record, render, share, symmetry } = globalThis.book;

const select = (text, execution) => JSON.parse(runtime.select(JSON.stringify({ version, pattern: text, execution })));
const selected = (text, execution) => {
    const reply = select(text, execution);
    assert.equal(reply.error, undefined, `${text}: ${reply.error?.message}`);
    return reply;
};
const parallel = record.workbench.Parallel.result.execution;
const shown = (text, execution = parallel) => [...pattern.state(selected(text, execution), graph.model(execution)).state].sort((left, right) => left - right);
assert.deepEqual(shown('C'), [2, 3, 4]);
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
    ['A)', /this \) closes nothing that is open$/, { offset: 1, length: 1 }],
    ['[A', /this \[ is never closed$/, { offset: 0, length: 1 }],
    ['(A]', /this \] does not close the \( at 1:1$/, { offset: 2, length: 1 }],
    ['A\u200BB', /U\+200B cannot appear in an atom$/, { offset: 1, length: 1 }],
]) {
    const error = refusal(text);
    assert.match(error.message, message, text);
    assert.deepEqual(error.detail.span, span, text);
}
assert.match(refusal('A, [B] C').message, /not both/);
assert.match(refusal('(K, [K] L), [B] C').message, /not both/);
assert.match(editor.describe(refusal('[AX)] B')), /\(at character 4\)$/);
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
    assert.match(link, /^lightbox\.html#1&source=/);
    const address = new URL(link, 'https://photonic.vantle.org/');
    assert.equal(address.search, '');
    assert.deepEqual(share.read(address.hash), program);
    assert.deepEqual(share.legacy(`?${address.hash.slice('#1&'.length)}`), program);
}
for (const hash of ['', '#', '#1', '#1&target=A', '#2&source=A', '#source=A', '#10&source=A']) assert.equal(share.read(hash), undefined, hash);
assert.equal(share.legacy('?target=A'), undefined);
console.log('Links carry the source, every target, every library and preserve in a versioned fragment, through commas, newlines, Unicode and URL syntax, and the query form of older links still reads.');

const standard = Object.keys(library);
assert.equal(standard.length, 68);
for (const [name, entry] of Object.entries(library)) {
    assert.equal(entry.load.at(-1), name, name);
    assert.equal(entry.package, name.split('/').at(-2), name);
    assert.equal(entry.load.length, new Set(entry.load).size, name);
}
const addition = { source: 'Operand.Left.Zero, Operand.Right.Zero, Function.Natural.Add', target: ['Return.Natural.Add.Zero'], library: ['library/natural/add'], preserve: true };
assert.deepEqual(engine.expand(addition.library), library['library/natural/add'].load);
assert.deepEqual(engine.expand(['library/function/invoke', 'library/boolean/not']), ['library/function/invoke', 'library/boolean/not']);
assert.deepEqual(engine.request(addition).library.map(entry => entry.name), library['library/natural/add'].load.map(name => `${name}.particle`));
const whole = engine.size({ source: '', target: [], library: standard, preserve: false });
assert.ok(whole < engine.capacity, `the whole library takes ${whole} bytes`);
assert.deepEqual(JSON.parse(runtime.explore(JSON.stringify({ version, ...engine.request(addition) }))).verdict.map(value => value.outcome), ['reached']);
console.log(`The standard library loads each library after the libraries it needs, all ${standard.length} files fit in one request of ${whole} bytes, and natural addition runs in the engine.`);

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

assert.equal(engine.advice, 'The engine is still loading. Try again in a moment.');
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
assert.deepEqual(settled.slice(5), [['slow', 'Stopped after 0.005 seconds without a result.'], ['patient', 'The engine ran out of memory or failed; the next run starts a fresh engine.']]);
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
assert.deepEqual(settled.at(-1), ['unloaded', 'This browser could not start the WebAssembly engine.']);
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
assert.deepEqual(settled.slice(-2), [['broken', 'This browser could not start the WebAssembly engine: no memory'], ['behind', 'This browser could not start the WebAssembly engine: no memory']]);
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
console.log('A reply from another engine version rejects every waiting request and asks for a reload.');

const clock = globalThis.setTimeout;
let expire;
globalThis.setTimeout = (callback, delay) => {
    if (delay !== 60000) return clock(callback, delay);
    expire = callback;
    return 0;
};
track('stalled', engine.send('lower', { source: 'M' }));
globalThis.setTimeout = clock;
assert.deepEqual(posted(worker[10]), ['M']);
await pause(40);
assert.deepEqual(settled.at(-1)[0], 'legacy');
expire();
await pause(0);
assert.deepEqual(settled.at(-1), ['stalled', 'This browser could not start the WebAssembly engine.']);
assert.equal(worker[10].terminated, true);
assert.equal(engine.state, 'failed');
assert.equal(engine.advice, 'This browser could not start the WebAssembly engine. Reload the page to try again.');
assert.deepEqual(announced, ['unknown', 'failed', 'live', 'failed', 'live', 'failed', 'stale', 'failed']);
assert.deepEqual(settled.map(([name]) => name).sort(), ['behind', 'broken', 'first', 'fourth', 'legacy', 'loading', 'patient', 'recovered', 'second', 'shared', 'slow', 'stale', 'stalled', 'third', 'unloaded', 'waiting']);
console.log('An engine that has not loaded after a minute fails every waiting request, and each request settles once.');

const tolerance = 1e-3;
const measure = (part, point) => {
    if (part.atom === 'Right') return [Math.PI / 2, false];
    if (part.atom === 'Excess') return [0, true];
    const [kind, vertex] = part.head.map(value => value.atom);
    if (kind === 'Sum') return part.body.map(value => measure(value, point)).reduce(([sum, more], [value, excess]) => [sum + value, more || excess], [0, false]);
    const letter = part.body.map(value => value.atom);
    const [start, end] = letter.map(name => point[name]);
    if (kind === 'Line') return [Math.hypot(end[0] - start[0], end[1] - start[1]), false];
    if (kind === 'Square') return [Math.hypot(end[0] - start[0], end[1] - start[1]) ** 2, false];
    if (kind === 'Angle') {
        const at = point[vertex];
        const [outward, inward] = [[start[0] - at[0], start[1] - at[1]], [end[0] - at[0], end[1] - at[1]]];
        return [Math.acos(Math.max(-1, Math.min(1, (outward[0] * inward[0] + outward[1] * inward[1]) / Math.hypot(...outward) / Math.hypot(...inward)))), false];
    }
    assert.equal(kind, 'Area');
    const ring = figure.ring(letter, point).map(name => point[name]);
    return [Math.abs(ring.reduce((sum, value, index) => sum + value[0] * ring[(index + 1) % ring.length][1] - ring[(index + 1) % ring.length][0] * value[1], 0)) / 2, false];
};
const side = (line, name, point) => {
    const [start, end] = line.map(value => point[value]);
    const place = point[name];
    return Math.sign((end[0] - start[0]) * (place[1] - start[1]) - (end[1] - start[1]) * (place[0] - start[0]));
};
const letter = part => part.body.map(value => value.atom);
const holds = (fact, point) => {
    const [head, first, second] = notation.term(fact);
    if (head.atom === 'Parallel') {
        const [one, other] = [first, second].map(line => {
            const [start, end] = letter(line).map(name => point[name]);
            return [end[0] - start[0], end[1] - start[1]];
        });
        return Math.abs(one[0] * other[1] - one[1] * other[0]) < tolerance * Math.hypot(...one) * Math.hypot(...other);
    }
    if (head.atom === 'Across' || head.atom === 'Same') {
        const product = letter(second).map(name => side(letter(first), name, point)).reduce((sum, value) => sum * value, 1);
        return head.atom === 'Across' ? product < 0 : product > 0;
    }
    if (head.atom === 'Parallelogram') {
        const [[one, three], [two, four]] = [letter(first), letter(second)];
        const crossing = side([two, four], one, point) * side([two, four], three, point) < 0 && side([one, three], two, point) * side([one, three], four, point) < 0;
        return crossing && figure.parallelogram([one, two, three, four].map(name => point[name]));
    }
    if (head.atom !== 'Equal') return undefined;
    const [[left, over], [right, under]] = [measure(first, point), measure(second, point)];
    assert.ok(!(over && under), `${fact} has an excess on both sides`);
    if (over) return right > left + tolerance;
    if (under) return left > right + tolerance;
    return Math.abs(left - right) < tolerance * Math.max(1, Math.abs(left));
};
const proposition = euclid.map(entry => entry.name);
const reading = new Map(euclid.map(entry => {
    const { source, event } = record.euclid[entry.name];
    const line = source.split('\n');
    const first = line.findIndex(text => text.startsWith('['));
    const head = line.slice(0, first).flatMap(text => notation.split(text.replace(/,$/, '')));
    const shown = new Set(head.filter(text => text.startsWith('(Suppose.') && text.includes('] Proved.')).map(text => /^\(Suppose\.(\w+),/.exec(text)[1]));
    const step = event.map(([at, world]) => {
        const text = line[at].replace(/,$/, '');
        const closing = text.startsWith('(');
        return { at, world, closing, ...notation.rule(closing ? text.slice(text.indexOf('['), text.lastIndexOf(')')) : text) };
    });
    const rule = line.map((text, at) => [text, at]).filter(([text]) => text.startsWith('[')).map(([text, at]) => ({ at, ...notation.rule(text.replace(/,$/, '')) }));
    return [entry.name, { entry, head, shown, step, rule }];
}));

let checked = 0;
for (const { entry, head, shown, step, rule } of reading.values()) {
    const world = new Map(rule.map(value => [value.at, new Set()]));
    step.filter(value => !value.closing).forEach(value => world.get(value.at).add(value.world));
    const fact = head.filter(text => !text.startsWith('(')).map(text => [text, undefined]);
    for (const value of rule) {
        for (const text of [...value.input, ...value.output]) assert.ok(notation.gloss(text, entry.point), `${entry.name}: ${text} reads as nothing`);
        for (const place of world.get(value.at)) {
            if (place === undefined || shown.has(place)) fact.push(...[...value.input, ...value.output].map(text => [text, place]));
        }
    }
    for (const [text, place] of fact) {
        const verdict = holds(text, { ...entry.point, ...entry.arrangement?.[place] });
        if (verdict === undefined) continue;
        assert.ok(verdict, `proposition ${entry.name}: ${text} is false in its figure${place ? ` in case ${place}` : ''}`);
        checked++;
    }
}
assert.ok(checked > 1000);
console.log(`Every one of the ${checked} facts that Euclid's proofs state outside a refuted case holds in the coordinates of its figure, and every fact has a reading.`);

for (const [position, { entry, rule }] of [...reading.values()].entries()) {
    assert.equal(entry.rule.length, rule.length, `book/euclid.js must describe each rule of proposition ${entry.name}`);
    assert.equal(entry.rule.at(-1).why, 'theorem', `proposition ${entry.name} must end with its conclusion`);
    entry.rule.forEach((detail, index) => {
        const { input, output } = rule[index];
        const where = `proposition ${entry.name}, rule ${index + 1}`;
        const cited = notation.cite(detail.why);
        if (cited) assert.ok(proposition.includes(cited) && (cited === entry.name || proposition.indexOf(cited) < position), `${where} cites ${cited}, which is not a proposition before it`);
        else assert.ok(detail.why === 'theorem' || Object.hasOwn(notation.basis, detail.why), `${where} is justified by ${detail.why}, which the book does not name`);
        assert.equal(detail.why === 'theorem', output.length === 1 && output[0] === 'Theorem', `${where} must conclude Theorem exactly when it is the conclusion`);
        assert.equal(detail.why === 'suppose', input.length === 1 && input[0].startsWith('Suppose.'), `${where} must open a case exactly when it is a supposition`);
        if (detail.why === 'circle' || detail.why === 'produce') assert.ok(input.some(figure.token), `${where} draws a circle, so it must construct a point`);
        for (const at of detail.figure ?? []) assert.ok(at < output.length && !['Shown', 'Absurd'].includes(output[at]), `${where} reads output ${at + 1} from its figure, which it does not give`);
    });
}
console.log('Every rule names the postulate, common notion, definition or earlier proposition that justifies it, and its shape fits that justification.');

const key = chain => chain.map(part => part.atom ?? `(${key(part.head)}|${key(part.body)})`).sort().join('.');
const canonical = text => key(notation.term(text));
const reflexive = text => {
    const [head, first, second] = notation.term(text);
    return head.atom === 'Equal' && key([first]) === key([second]);
};
function* unify(pattern, target, binding) {
    if (pattern.length !== target.length) return;
    if (!pattern.length) {
        yield binding;
        return;
    }
    const [first, ...rest] = pattern;
    for (const [index, other] of target.entries()) {
        for (const extended of match(first, other, binding)) yield* unify(rest, target.toSpliced(index, 1), extended);
    }
}
function* match(pattern, target, binding) {
    if (pattern.atom !== undefined) {
        if (target.atom === undefined) return;
        if (!figure.token(pattern.atom)) {
            if (pattern.atom === target.atom) yield binding;
            return;
        }
        if (!figure.token(target.atom)) return;
        const bound = binding.get(pattern.atom);
        if (bound === undefined) yield new Map(binding).set(pattern.atom, target.atom);
        else if (bound === target.atom) yield binding;
        return;
    }
    if (target.atom !== undefined) return;
    for (const extended of unify(pattern.head, target.head, binding)) yield* unify(pattern.body, target.body, extended);
}
const mention = text => new Set(text.match(/\b[A-Z]\b/g));
// A hypothesis that only puts a point the conclusion does not name on a straight line produced holds
// for a point that Postulate 2 constructs, so a citation need not supply it.
const production = /^Equal\.\(\[Line\] ([A-Z])\.([A-Z])\)\.\(\[Sum\] \(\[Line\] ([A-Z])\.([A-Z])\)\.\(\[Line\] ([A-Z])\.([A-Z])\)\)$/;
const discharge = (hypothesis, named) => hypothesis.filter(fact => {
    const [, first, second, ...part] = production.exec(fact) ?? [];
    if (!first) return true;
    const free = [first, second].find(name => !named.has(name) && part.filter(value => value === name).length === 1);
    return !free || hypothesis.some(other => other !== fact && mention(other).has(free));
});
const statement = new Map([...reading.values()].map(({ entry, head, step, rule }) => {
    const given = new Set(head.filter(text => !text.startsWith('(') && !figure.token(text)).map(canonical));
    const producer = new Map();
    const place = (world, text) => `${world ?? ''}|${canonical(text)}`;
    step.forEach((value, index) => value.output.forEach(text => {
        const name = place(value.closing ? undefined : value.world, text);
        producer.set(name, [...producer.get(name) ?? [], index]);
    }));
    // The hypotheses a conclusion rests on are the given facts its derivation consumes at the root;
    // a case draws its facts from its supposition, never from the root.
    const slice = conclusion => {
        const hypothesis = new Set();
        const construction = new Set();
        const seen = new Set();
        const visit = (fact, world) => {
            if (figure.token(fact)) {
                construction.add(fact);
                return;
            }
            const name = place(world, fact);
            if (fact.startsWith('Suppose.') || seen.has(name)) return;
            seen.add(name);
            if (world === undefined && given.has(canonical(fact)) && !reflexive(fact)) hypothesis.add(fact);
            for (const index of producer.get(name) ?? []) step[index].input.forEach(input => visit(input, step[index].world));
        };
        visit(conclusion, undefined);
        return { hypothesis: [...hypothesis], construction: [...construction] };
    };
    const conclusion = rule.at(-1).input;
    const suppose = new Map(rule.filter(value => value.input.length === 1 && value.input[0].startsWith('Suppose.')).map(value => [value.input[0].slice('Suppose.'.length), value.output.filter(fact => !figure.token(fact))]));
    return [entry.name, { conclusion, slice, suppose }];
}));
function* satisfy(hypothesis, input, binding) {
    if (!hypothesis.length) {
        yield binding;
        return;
    }
    const [first, ...rest] = hypothesis;
    for (const text of input) {
        for (const extended of unify(notation.term(first), notation.term(text), binding)) yield* satisfy(rest, input, extended);
    }
}
const instance = (cited, detail, { input, output }) => {
    const theorem = statement.get(cited);
    const read = new Set(input.map(canonical));
    const figured = new Set(detail.figure ?? []);
    const claim = output.filter((value, at) => !figured.has(at) && !figure.token(value) && !read.has(canonical(value)));
    const constructed = new Set(input.filter(figure.token));
    function* search(remaining, binding, need) {
        if (!remaining.length) {
            yield { binding, need };
            return;
        }
        const [first, ...rest] = remaining;
        // A citation reports Absurd when its facts describe an arrangement the cited proposition refutes,
        // and Shown when the conclusion it reaches is the claim of the case it runs in.
        if (first === 'Absurd') {
            for (const [name, fact] of theorem.suppose) {
                if (theorem.conclusion.includes(`Refuted.${name}`)) yield* search(rest, binding, [...need, { hypothesis: fact, construction: [], conclusion: [] }]);
            }
            return;
        }
        for (const conclusion of theorem.conclusion.filter(value => !/^(Refuted|Proved)\./.test(value))) {
            if (first === 'Shown') {
                yield* search(rest, binding, [...need, { ...theorem.slice(conclusion), conclusion: [conclusion] }]);
                continue;
            }
            for (const extended of unify(notation.term(conclusion), notation.term(first), binding)) yield* search(rest, extended, [...need, { ...theorem.slice(conclusion), conclusion: [conclusion] }]);
        }
    }
    for (const { binding, need } of search(claim, new Map(), [])) {
        const named = new Set(need.flatMap(value => value.conclusion.flatMap(conclusion => [...mention(conclusion)])));
        const hypothesis = discharge([...new Set(need.flatMap(value => value.hypothesis))], named);
        const construction = [...new Set(need.flatMap(value => value.construction))].filter(name => named.has(name));
        for (const complete of satisfy(hypothesis, input, binding)) {
            if (construction.every(name => constructed.has(complete.get(name)))) return true;
        }
    }
    return false;
};
let citation = 0;
for (const { entry, rule } of reading.values()) {
    entry.rule.forEach((detail, index) => {
        const cited = notation.cite(detail.why);
        if (!cited || cited === entry.name) return;
        assert.ok(instance(cited, detail, rule[index]), `proposition ${entry.name}, rule ${index + 1}, is not an instance of proposition ${cited}`);
        citation++;
    });
}
assert.ok(citation > 150);
console.log(`Every one of the ${citation} citations between propositions is an instance of the proposition it cites.`);
