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
for (const source of [
    'A, [A] B',
    'A, [A] B.C, [B] D',
    'Seed.A, [Seed] ().([A] B)',
    'A.X, B.Y, [A, B] (C, D), [C, D] E',
    'A, [A] A.A',
    'And.True.False, [True] Boolean, [False] Boolean, [And.Boolean.Boolean] ([True.False] False)',
]) {
    const path = join(process.env.TEST_TMPDIR, 'source.wave');
    await writeFile(path, source);
    const native = spawnSync(command, ['run', path, '--work', '20000', '--configuration', '128', '--occurrence', '128', '--scope', '16', '--coherence', '16', '--record', '100000', '--json'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    assert.equal(native.status, 0, native.error?.message || native.stderr);
    const response = explore({ source, target: [] });
    assert.equal(response.error, undefined);
    const report = JSON.parse(native.stdout);
    const occurrence = token => token.atom === undefined
        ? { kind: 'rule', id: token.id, rule: token.rule, ...(token.capture === undefined ? {} : { capture: token.capture }) }
        : { kind: 'atom', id: token.id, label: token.atom };
    const configuration = node => ({
        id: node.id,
        world: node.world.map(world => ({ frame: world.frame, particle: world.particle.map(occurrence) })),
        frame: node.frame.map(frame => ({ parent: frame.parent, particle: frame.particle.map(token => ({ id: token.id, rule: token.rule })), held: frame.held.map(occurrence) })),
    });
    const definition = report.definition.map(value => value.name);
    const chain = index => {
        const path = [];
        for (let origin = report.view[index].origin; origin; origin = report.view[origin.view].origin) path.unshift(origin.event);
        return path;
    };
    const direct = value => value.evidence.some(index => report.view[index].source === report.view[index].target);
    const event = report.event.map(value => ({
        id: value.id,
        source: value.source,
        target: value.target,
        rule: report.definition[value.rule].name,
        footprint: value.footprint,
        exact: value.exact,
        world: value.world,
        context: value.context,
        deduction: direct(value) ? [] : chain(value.evidence[0]),
    }));
    assert.deepEqual(response.execution, { definition, closed: report.closed, work: report.work, state: report.state.map(configuration), event }, source);
}
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
assert.equal(located.error.span.offset, 5);
assert.equal(explore({ source: '人, [B' }).error.span.offset, 5);
assert.equal(explore({ source: '人 [B' }).error.span.offset, 4);
assert.deepEqual(explore({ source: 'A', target: ['人.人, [B'] }).error.span, { offset: 7, length: 0 });
assert.equal(explore({ source: 'A', target: ['A', 'B, ['] }).error.target, 1);
assert.deepEqual(explore({ source: '⟨x⟩, [⟨x⟩] B' }).execution.state[0].world[0].particle, [{ kind: 'atom', id: 0, label: '⟨x⟩' }]);
const valued = explore({ source: 'Seed.A, [Seed] ().([A] B)' }).execution;
assert.deepEqual(valued.state[1].world[0].particle.map(token => token.kind), ['atom', 'rule']);
const [written] = valued.state[1].world[0].particle.filter(token => token.kind === 'rule');
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
console.log('WebAssembly exploration matches native Rust reports, including suspended exploration, generated code and deductions.');

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
assert.equal(refused({ version, source: 'A, [B' }).span.offset, 5);
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
assert.equal(broken.error.span.offset, 5);
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
assert.equal(shape({ program: [deep] }).error.code, 'budget');
assert.equal(engine.compare, undefined);
console.log('The shape export groups programs by shape, names every atom in each program and matches the native command.');
