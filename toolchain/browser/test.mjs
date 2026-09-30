import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [webassembly, javascript, command] = process.argv.slice(2);
const engine = await import(pathToFileURL(javascript));
engine.initSync({ module: await readFile(webassembly) });
const version = 3;
const call = (entry, body) => JSON.parse(entry(JSON.stringify(body)));
const explore = body => call(engine.explore, { version, ...body });
const lower = source => call(engine.lower, { version, source });
const follow = body => new engine.Path(JSON.stringify({ version, ...body }));
const expression = source => engine.Path.expression(JSON.stringify({ version, source }));
const select = (pattern, execution) => call(engine.select, { version, pattern, execution });
const budget = ['--engine', 'interpreter', '--work', '20000', '--configuration', '128', '--occurrence', '256', '--scope', '16', '--coherence', '16', '--record', '100000'];
const ask = (path, verb, ...argument) => {
    const reply = spawnSync(command, [verb, path, ...argument, ...budget, '--json'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    assert.equal(reply.status, 0, reply.error?.message || reply.stderr);
    return JSON.parse(reply.stdout).answer;
};
const number = handle => Number(handle.slice(1));
const atom = text => text.split(/[\s.,()[\]]+/).filter(Boolean).sort().join(' ');
const occurrence = handle => Number(handle.split('.o')[1]);
const place = value => Object.values(value)[0][1];
const render = (execution, state) => {
    const particle = world => {
        const part = world.particle.map(token => token.kind === 'atom' ? token.label : `(${execution.definition[token.rule]})`);
        if (!part.length) return '()';
        return world.particle.length === 1 && world.particle[0].kind === 'rule' ? `().${part[0]}` : part.join('.');
    };
    const inside = frame => state.world.filter(world => world.frame === frame).map(particle).join(', ');
    const part = [inside(0), ...state.frame.slice(1).map((_, index) => inside(index + 1) && `in f${index + 1}: ${inside(index + 1)}`)].filter(Boolean);
    return part.length ? part.join(' · ') : 'nothing';
};
for (const [source, pattern] of [
    ['A, [A] B', ['A', '()', '[A] B']],
    ['A, [A] B.C, [B] D', ['C', 'D', '[A] B.C', '[B] D']],
    ['Seed.A, [Seed] ().([A] B)', ['A', 'B', '().([A] B)', '[A] B']],
    ['A.X, B.Y, [A, B] (C, D), [C, D] E', ['X.Y', 'C, D', '[A, B] (C, D)', '[C, D] E']],
    ['A, [A] A.A', ['A.A.A', '[A] A.A']],
    ['Brew.Tea, [Brew] (Kettle, [Kettle.Tea] Cup)', ['Tea', '(Kettle, [Kettle.Tea] Cup)', '[Kettle.Tea] Cup']],
    ['And.True.False.Extra, [True] Boolean, [False] Boolean, [And.Boolean.Boolean] ([True.True] True, [True.False] False, [False.False] False)', ['Extra', 'False.Extra', 'Boolean.Boolean', '[True] Boolean', '[True.False] False']],
]) {
    const path = join(process.env.TEST_TMPDIR, 'source.wave');
    await writeFile(path, source);
    const { execution } = explore({ source, target: [] });
    const summary = ask(path, 'explore', '--limit', '1000');
    assert.equal(execution.state.length, summary.configuration, source);
    assert.equal(execution.event.length, summary.event, source);
    assert.equal(execution.closed, summary.closed, source);
    assert.deepEqual(execution.state.map(state => state.id), execution.state.map((_, index) => index), source);
    const leaving = new Set(execution.event.map(event => event.source));
    assert.deepEqual(execution.state.filter(state => !leaving.has(state.id)).map(state => state.id), summary.end.map(entry => number(entry.handle)), source);
    for (const entry of summary.end) assert.equal(render(execution, execution.state[number(entry.handle)]), entry.text, source);
    summary.rule.forEach((rule, index) => {
        assert.equal(atom(execution.definition[index]), atom(rule.text), `${source} ${rule.handle}`);
        const first = execution.event.find(event => event.rule === execution.definition[index]);
        assert.equal(first?.id, rule.first === undefined ? undefined : number(rule.first), `${source} ${rule.handle}`);
    });
    for (const text of pattern) {
        const expected = ask(path, 'select', '--pattern', text, '--limit', '100000');
        const found = select(text, execution);
        assert.equal(found.kind, expected.kind, `${source} ${text}`);
        if (found.kind === 'event') {
            assert.deepEqual(found.event, expected.found.map(entry => number(entry.handle)), `${source} ${text}`);
            continue;
        }
        assert.deepEqual(found.state.map(state => state.id), expected.found.map(entry => number(entry.handle)), `${source} ${text}`);
        found.state.forEach((state, index) => {
            assert.deepEqual(state.world.flatMap(entry => entry.token).sort((left, right) => left - right), (expected.found[index].occurrence ?? []).map(occurrence).sort((left, right) => left - right), `${source} ${text} s${state.id}`);
            assert.equal(render(execution, execution.state[state.id]), expected.found[index].text, `${source} ${text} s${state.id}`);
        });
    }
    if (execution.event.length > 40) continue;
    for (const event of execution.event) {
        const expected = ask(path, 'inspect', `e${event.id}`);
        assert.deepEqual([event.source, event.target, event.deduction], [number(expected.source), number(expected.target), expected.deduction.map(number)], `${source} e${event.id}`);
        assert.deepEqual(event.exact.map(place).sort(), expected.exact.map(entry => occurrence(entry.handle)).sort(), `${source} e${event.id}`);
        assert.deepEqual(event.footprint.map(place).filter(id => !event.exact.map(place).includes(id)).sort(), expected.witness.map(entry => occurrence(entry.handle)).sort(), `${source} e${event.id}`);
    }
}
const conjunction = 'And.True.False.Extra, [True] Boolean, [False] Boolean, [And.Boolean.Boolean] ([True.True] True, [True.False] False, [False.False] False)';
const verdict = explore({ source: conjunction, target: ['False.Extra', 'True.Extra'], preserve: true }).verdict;
const sample = join(process.env.TEST_TMPDIR, 'conjunction.wave');
await writeFile(sample, conjunction);
const reach = ask(sample, 'check', '--reach', 'False.Extra', '--exact', '--preserve').claim[0];
assert.deepEqual(verdict, [{ outcome: 'reached', witness: number(reach.witness) }, { outcome: 'unreachable', witness: null }]);
assert.deepEqual(explore({ source: 'A, [A] B', target: ['B, [A] B', 'C, [A] B'] }).verdict.map(value => value.outcome), ['reached', 'unreachable']);
assert.deepEqual(explore({ source: 'A, [A] B', target: ['B', 'A'], preserve: true }).verdict.map(value => value.outcome), ['reached', 'reached']);
assert.deepEqual(explore({ source: '[] A', target: ['A.A, [] A'] }).verdict.map(value => value.outcome), ['unknown']);
assert.equal(explore({ version: version + 1, source: 'A' }).error.code, 'version');
assert.equal(explore({ version: version - 1, source: 'A', extra: true }).error.code, 'version');
const unversioned = call(engine.explore, { source: 'A' });
assert.equal(unversioned.error.code, 'version');
assert.match(unversioned.error.message, /Reload the page/);
assert.equal(unversioned.version, version);
assert.equal(explore({ source: '[', target: [] }).error.code, 'source');
assert.equal(explore({ source: 'A', target: ['['] }).error.code, 'target');
assert.equal(explore({ source: 'A', target: ['A, [A] B'] }).verdict[0].outcome, 'unreachable');
assert.ok(explore({ source: 'A', target: Array(17).fill('A') }).error);
assert.equal(explore({ source: 'A', extra: true }).error.code, 'request');
assert.equal(JSON.parse(engine.explore('[')).error.code, 'request');
assert.equal(explore({ source: 'A'.repeat(131073) }).error.code, 'size');
assert.equal(explore({ source: 'A', target: ['A'] }).verdict[0].outcome, 'reached');
const located = explore({ source: 'A, [B' });
assert.equal(located.error.code, 'source');
assert.equal(located.error.span.offset, 3);
assert.equal(explore({ source: '人, [B' }).error.span.offset, 3);
assert.equal(explore({ source: '人 [B' }).error.span.offset, 2);
assert.deepEqual(explore({ source: 'A', target: ['人.人, [B'] }).error.span, { offset: 5, length: 1 });
assert.equal(explore({ source: 'A', target: ['A', 'B, ['] }).error.target, 1);
assert.deepEqual(explore({ source: '⟨x⟩, [⟨x⟩] B' }).execution.state[0].world[0].particle, [{ kind: 'atom', id: 0, label: '⟨x⟩' }]);
const valued = explore({ source: 'Seed.A, [Seed] ().([A] B)' }).execution;
const holding = valued.state.find(state => state.world[0]?.particle.some(token => token.kind === 'rule'));
assert.deepEqual(holding.world[0].particle.map(token => token.kind), ['atom', 'rule']);
const [written] = holding.world[0].particle.filter(token => token.kind === 'rule');
assert.deepEqual(Object.keys(written).sort(), ['capture', 'id', 'kind', 'rule']);
assert.equal(valued.definition[written.rule], '[A] B');
for (const token of valued.state.flatMap(node => node.frame.flatMap(frame => frame.particle))) {
    assert.deepEqual(Object.keys(token).sort(), ['id', 'rule']);
    assert.equal(typeof valued.definition[token.rule], 'string');
}
assert.deepEqual(valued.state[0].frame[0].particle.map(token => valued.definition[token.rule]), ['[Seed] ().([A] B)']);
const scoped = explore({ source: 'X.([A] B), [X.([A] B)] (Y, [Y] Z)' }).execution;
assert.deepEqual(scoped.state[1].frame[1].held.map(token => [token.kind, token.kind === 'rule' ? scoped.definition[token.rule] : token.label]).sort(), [['atom', 'X'], ['rule', '[A] B']]);
const deduced = explore({ source: 'A, [A] B.C, [B] D' }).execution;
const shortcut = deduced.event.find(value => value.source === 0 && value.rule === '[B] D');
assert.deepEqual(shortcut.deduction.map(index => deduced.event[index].rule), ['[A] B.C']);
assert.ok(deduced.event.filter(value => value.rule === '[A] B.C').every(value => !value.deduction.length));
console.log('WebAssembly exploration gives every configuration, event and occurrence the handle the command line gives it, with the same ends, first events, matches, deductions and witnesses, including suspended exploration and generated code.');

const symmetric = explore({ source: 'Light, [Light] Red, [Light] Green, [Light] Blue' });
assert.equal(symmetric.symmetry.size, '6');
assert.deepEqual(symmetric.symmetry.class, [{ kind: 'global', part: [['Red', 'Green', 'Blue']], rule: ['[Light] Blue', '[Light] Green', '[Light] Red'] }]);
assert.ok(symmetric.execution.event.every(value => symmetric.symmetry.class[0].rule.includes(value.rule)));
const local = explore({ source: 'Start, [Start] Not.True, [Not.True] False, [Not.False] True' }).symmetry;
assert.equal(local.size, '1');
assert.deepEqual(local.class.map(value => [value.kind, value.part.map(part => [...part].sort()), value.rule]), [['local', [['False', 'True'], ['False', 'True']], ['[Not.False] True', '[Not.True] False']]]);
assert.deepEqual(explore({ source: 'Go.Fast, [Go.Fast] Stop' }).symmetry, { size: '2', class: [{ kind: 'block', part: [['Go', 'Fast']], rule: [] }] });
assert.deepEqual(explore({ source: 'A, [A] B' }).symmetry, { size: '1', class: [] });
assert.deepEqual(explore({ source: 'A, B, [A] C, [B] D, [X] Y, [X] Y' }).symmetry.class, [{ kind: 'global', part: [['A', 'B'], ['C', 'D']], rule: ['[A] C', '[B] D'] }]);
console.log('Exploration reports global symmetries, rules that repeat under other names and blocks, named as events name their rules.');

const library = [{ name: 'not.particle', source: '[Not.True] False,\n[Not.False] True' }];
assert.deepEqual(explore({ source: 'Not.True', library, target: ['False'], preserve: true }).verdict.map(value => value.outcome), ['reached']);
assert.deepEqual(explore({ source: 'Not.True', library, target: ['False'] }).verdict.map(value => value.outcome), ['unreachable']);
assert.equal(explore({ source: 'A', library: [{ name: 'data.particle', source: 'B' }] }).error.code, 'library');
assert.equal(explore({ source: 'A', library: [{ name: 'broken.particle', source: '[' }] }).error.code, 'library');
console.log('Libraries load as declarations, and preserved targets include their rules.');

const lowered = lower('A.(B, C), [A] (D, E)');
assert.equal(lowered.version, version);
assert.deepEqual(lowered.program.initial, [['A', 'B'], ['A', 'C']]);
assert.equal(lowered.program.rule[0].output.length, 2);
assert.equal(lowered.program.rule[0].name, '[A] (D, E)');
const nested = lower('[Seed]\n    ().([A]  (B,\n\t[B] C))').program.rule[0];
assert.equal(nested.name, '[Seed] ().([A] (B, [B] C))');
assert.equal(nested.output[0][0].rule.name, '[A] (B, [B] C)');
assert.equal(nested.output[0][0].rule.output[0].rule[0].name, '[B] C');
assert.equal(lower('[A] (B').error.code, 'source');
assert.deepEqual(lower('⟨x⟩ ]').error.span, { offset: 4, length: 1 });
assert.equal(lower('A'.repeat(131073)).error.code, 'size');
assert.equal(JSON.parse(engine.lower('A')).error.code, 'request');
assert.equal(call(engine.lower, { version: version + 1, source: 'A' }).error.code, 'version');
console.log('Lowering reports programs and located syntax errors.');

const theorem = follow({ source: 'A, A, [A] B, [B, B] Theorem', target: ['Theorem'], preserve: true });
const proved = JSON.parse(theorem.run());
assert.equal(proved.outcome, 'reached');
assert.ok(proved.event > 0);
assert.deepEqual(proved.definition, ['[A] B', '[B, B] Theorem']);
assert.deepEqual(Object.keys(proved).sort(), ['definition', 'event', 'outcome', 'version', 'work']);
const opening = JSON.parse(theorem.inspect(0));
assert.equal(opening.before.id, 0);
assert.equal(opening.after.id, opening.event.target);
assert.deepEqual(Object.keys(opening).sort(), ['after', 'before', 'event', 'version']);
assert.deepEqual(Object.keys(opening.event).sort(), ['exact', 'footprint', 'rule', 'source', 'target']);
assert.ok(JSON.parse(theorem.inspect(proved.event)).error);
theorem.free();
const open = follow({ source: 'A, [A] B' });
assert.deepEqual(Object.keys(JSON.parse(open.run())).sort(), ['definition', 'event', 'version', 'work']);
open.free();
const refused = path => {
    const session = new engine.Path(JSON.stringify(path));
    const result = JSON.parse(session.run());
    assert.ok(JSON.parse(session.inspect(0)).error);
    session.free();
    return result.error;
};
assert.equal(refused({ version, source: 'A', target: ['A', 'B'] }).code, 'target');
assert.deepEqual(refused({ version, source: 'A, [B' }), refused({ version, source: 'A, [B', target: ['A'] }));
assert.equal(refused({ version, source: 'A, [B' }).span.offset, 3);
assert.equal(refused({ version: version + 1, source: 'A' }).code, 'version');
console.log('Direct paths reach preserved targets, inspect every transition and locate refusals.');

const evaluate = input => {
    const path = expression(input);
    const result = JSON.parse(path.run());
    path.free();
    return result;
};
for (const input of ['3+1', '1 2', 'A', '1'.repeat(257)]) {
    assert.ok(evaluate(input).error, input);
}
assert.equal(evaluate('1'.repeat(257)).error.code, 'size');
const raw = engine.Path.expression('12+2');
assert.equal(JSON.parse(raw.run()).error.code, 'request');
raw.free();
const mismatched = engine.Path.expression(JSON.stringify({ version: version + 1, source: '12+2' }));
assert.equal(JSON.parse(mismatched.run()).error.code, 'version');
mismatched.free();
const session = expression('12+2');
const completed = JSON.parse(session.run());
assert.deepEqual(Object.keys(completed).sort(), ['definition', 'event', 'source', 'state', 'version', 'work']);
assert.match(completed.source, /Function\.Expression\.Evaluate/);
const first = JSON.parse(session.inspect(0));
assert.equal(first.before.id, 0);
assert.equal(first.after.id, first.event.target);
assert.match(first.event.rule, /Push/);
const last = JSON.parse(session.inspect(completed.event - 1));
assert.deepEqual(last.after, completed.state);
assert.ok(JSON.parse(session.inspect(completed.event)).error);
assert.deepEqual(JSON.parse(session.inspect(0)), first);
session.free();
const rejected = expression('1/0');
const error = JSON.parse(rejected.run());
assert.ok(JSON.parse(rejected.inspect(error.event - 1)).event);
rejected.free();
console.log('Native Photonic expressions execute through Wasm as direct paths that refuse malformed requests and inspect every transition, including division by zero.');

const shape = body => call(engine.shape, { version, ...body });
const parity = '[Add.0.0] 0,\n[Add.0.1] 1,\n[Add.1.1] 0';
const exclusive = '[Xor.False.False] False,\n[Xor.False.True] True,\n[Xor.True.True] False';
const card = '[Compose.Keep.Keep] Keep,\n[Compose.Keep.Flip] Flip,\n[Compose.Flip.Flip] Keep';
const connected = shape({ program: [parity, exclusive, card] });
assert.equal(connected.shape.length, 1);
assert.deepEqual(connected.shape[0].member, [0, 1, 2]);
assert.deepEqual(
    Object.fromEntries(connected.shape[0].atom.map(row => [row.name[0], row.name.slice(1)])),
    { Add: ['Xor', 'Compose'], 0: ['False', 'Keep'], 1: ['True', 'Flip'] },
);
assert.equal(connected.shape[0].rule.length, 3);
assert.equal(connected.shape[0].size, '1');
assert.deepEqual(connected.shape[0].symmetry, []);
const light = shape({ program: ['Light, [Light] Red, [Light] Green, [Light] Blue'] });
assert.equal(light.shape[0].size, '6');
assert.equal(light.shape[0].symmetry.length, 5);
assert.deepEqual(light.shape[0].initial, [light.shape[0].atom.find(row => row.name[0] === 'Light').letter]);
const apart = shape({ program: ['A, [A] B', 'A, [A] B, [B] C'] });
assert.deepEqual(apart.shape.map(value => value.member), [[0], [1]]);
const [blocked] = shape({ program: ['[Boolean.Not.True] False'] }).shape;
assert.deepEqual(blocked.block[0].map(letter => blocked.atom.find(row => row.letter === letter).name[0]).sort(), ['Boolean', 'Not', 'True']);
assert.equal(shape({ program: [] }).error.code, 'request');
assert.equal(shape({ program: Array(5).fill('A') }).error.code, 'request');
assert.equal(shape({ version: version + 1, program: ['A'] }).error.code, 'version');
const broken = shape({ program: ['A', 'B, [C'] });
assert.equal(broken.error.code, 'source');
assert.equal(broken.error.program, 1);
assert.equal(broken.error.span.offset, 3);
const file = [];
for (const [index, source] of [parity, exclusive, card].entries()) {
    const path = join(process.env.TEST_TMPDIR, `shape.${index}.wave`);
    await writeFile(path, source);
    file.push(path);
}
const native = spawnSync(command, ['shape', ...file, '--json'], { encoding: 'utf8' });
assert.equal(native.status, 0, native.stderr);
assert.deepEqual(JSON.parse(native.stdout).answer.class[0].atom, connected.shape[0].atom.map(row => row.name));
const deep = Array.from({ length: 3500 }, (_, index) => `[a${index}] b${index},[b${index}] a${index}.b${index}`).join(',\n');
const crowd = shape({ program: [deep] });
assert.equal(crowd.error, undefined, JSON.stringify(crowd.error));
assert.equal(crowd.shape.length, 1);
assert.equal(engine.compare, undefined);
console.log('The shape export groups programs by shape, names every atom in each program and matches the native command.');
