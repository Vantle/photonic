import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [webassembly, javascript, command, numeral] = process.argv.slice(2);
const engine = await import(pathToFileURL(javascript));
engine.initSync({ module: await readFile(webassembly) });
const explore = request => JSON.parse(engine.explore(JSON.stringify(request)));
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
    const native = spawnSync(command, ['run', path, '--steps', '20000', '--states', '128', '--cells', '128', '--frames', '16', '--coherences', '16', '--records', '100000', '--json'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    assert.equal(native.status, 0, native.error?.message || native.stderr);
    const response = explore({ version: 1, source, target: [] });
    assert.equal(response.error, undefined);
    const report = JSON.parse(native.stdout);
    const chain = index => {
        const path = [];
        for (let origin = report.view[index].origin; origin; origin = report.view[origin.view].origin) path.unshift(origin.event);
        return path;
    };
    const direct = value => value.evidence.some(index => report.view[index].source === report.view[index].target);
    const event = report.event.map(value => ({ ...value, deduction: direct(value) ? [] : chain(value.evidence[0]) }));
    assert.deepEqual(response.execution, { ...report, event, view: [] }, source);
}
assert.deepEqual(explore({ version: 1, source: 'A, [A] B', target: ['B, [A] B', 'C, [A] B'] }).verdict.map(value => value.outcome), ['reached', 'unreachable']);
assert.deepEqual(explore({ version: 1, source: 'A, [A] B', target: ['B', 'A'], preserve: true }).verdict.map(value => value.outcome), ['reached', 'reached']);
assert.deepEqual(explore({ version: 1, source: '[] A', target: ['A.A, [] A'] }).verdict.map(value => value.outcome), ['unknown']);
assert.equal(explore({ version: 2, source: 'A' }).error.code, 'version');
assert.equal(explore({ version: 1, source: '[', target: [] }).error.code, 'source');
assert.equal(explore({ version: 1, source: 'A', target: ['['] }).error.code, 'target');
assert.equal(explore({ version: 1, source: 'A', target: ['A, [A] B'] }).verdict[0].outcome, 'unreachable');
assert.ok(explore({ version: 1, source: 'A', target: Array(17).fill('A') }).error);
assert.equal(explore({ version: 1, source: 'A', extra: true }).error.code, 'request');
assert.equal(JSON.parse(engine.explore('[')).error.code, 'request');
assert.equal(explore({ version: 1, source: 'A'.repeat(131073) }).error.code, 'size');
assert.equal(explore({ version: 1, source: 'A', target: ['A'] }).verdict[0].outcome, 'reached');
const located = explore({ version: 1, source: 'A, [B' });
assert.equal(located.error.code, 'source');
assert.equal(located.error.span.offset, 5);
assert.equal(explore({ version: 1, source: '人, [B' }).error.span.offset, 5);
assert.equal(explore({ version: 1, source: '人 [B' }).error.span.offset, 4);
assert.equal(explore({ version: 1, source: 'A', target: ['人.人, [B'] }).error.span.offset, 7);
const deduced = explore({ version: 1, source: 'A, [A] B.C, [B] D' }).execution;
const shortcut = deduced.event.find(value => value.source === 0 && value.rule === '[B] D');
assert.deepEqual(shortcut.deduction.map(index => deduced.event[index].rule), ['[A] B.C']);
assert.ok(deduced.event.filter(value => value.rule === '[A] B.C').every(value => !value.deduction.length));
console.log('WebAssembly exploration matches native Rust reports, including suspended exploration, generated code and deductions.');

const library = [{ name: 'not.particle', source: '[Not.True] False,\n[Not.False] True' }];
assert.deepEqual(explore({ version: 1, source: 'Not.True', library, target: ['False'], preserve: true }).verdict.map(value => value.outcome), ['reached']);
assert.deepEqual(explore({ version: 1, source: 'Not.True', library, target: ['False'] }).verdict.map(value => value.outcome), ['unreachable']);
assert.equal(explore({ version: 1, source: 'A', library: [{ name: 'data.particle', source: 'B' }] }).error.code, 'library');
assert.equal(explore({ version: 1, source: 'A', library: [{ name: 'broken.particle', source: '[' }] }).error.code, 'library');
console.log('Libraries load as declarations, and preserved targets include their rules.');

const lowered = JSON.parse(engine.lower('A.(B, C), [A] (D, E)'));
assert.deepEqual(lowered.program.initial, [['A', 'B'], ['A', 'C']]);
assert.equal(lowered.program.rule[0].output.length, 2);
assert.equal(JSON.parse(engine.lower('[A] (B')).error.code, 'source');
assert.deepEqual(JSON.parse(engine.lower('⟨x⟩ ]')).error.span, { offset: 4, length: 1 });
assert.equal(JSON.parse(engine.lower('A'.repeat(131073))).error.code, 'size');
console.log('Lowering reports programs and located syntax errors.');

const theorem = new engine.Path(JSON.stringify({ version: 1, source: 'A, A, [A] B, [B, B] Theorem', target: ['Theorem'], preserve: true }));
const proved = JSON.parse(theorem.run());
assert.equal(proved.outcome, 'reached');
assert.ok(proved.event > 0);
assert.ok(proved.definition.length > 0);
const opening = JSON.parse(theorem.inspect(0));
assert.equal(opening.before.id, 0);
assert.equal(opening.after.id, opening.event.target);
assert.deepEqual(Object.keys(opening).sort(), ['after', 'before', 'event', 'version']);
assert.ok(JSON.parse(theorem.inspect(proved.event)).error);
theorem.free();
const open = new engine.Path(JSON.stringify({ version: 1, source: 'A, [A] B' }));
assert.equal(JSON.parse(open.run()).outcome, undefined);
open.free();
const refused = path => {
    const session = new engine.Path(JSON.stringify(path));
    const result = JSON.parse(session.run());
    assert.ok(JSON.parse(session.inspect(0)).error);
    session.free();
    return result.error;
};
assert.equal(refused({ version: 1, source: 'A', target: ['A', 'B'] }).code, 'target');
assert.deepEqual(refused({ version: 1, source: 'A, [B' }), refused({ version: 1, source: 'A, [B', target: ['A'] }));
assert.equal(refused({ version: 1, source: 'A, [B' }).span.offset, 5);
assert.equal(refused({ version: 2, source: 'A' }).code, 'version');
console.log('Direct paths reach preserved targets, inspect every transition and locate refusals.');

const { decode } = await import(pathToFileURL(numeral));
const evaluate = input => {
    const path = engine.Path.expression(input);
    const result = JSON.parse(path.run());
    path.free();
    return result;
};
for (const [input, expected] of [
    ['12 + 2', '21'], ['12 * 2', '101'], ['21 / 2 - 1', '2'],
    ['-(12 + 2) * 10', '-210'], ['-21 / 2', '-10'],
    ['1212 * 10 / 2 + 11 - 1', '2220'], ['0', '0'], ['00012', '12'],
    ['2 + 1 * 2', '11'], ['(2 + 1) * 2', '20'], ['--2', '2'], ['-(12+2)*2', '-112'], ['(-2)*(-2)', '11'], ['(-2)*0', '0'],
]) {
    const result = evaluate(input);
    assert.equal(result.error, undefined, input);
    assert.equal(decode(result.state).ternary, expected, input);
}
for (const input of ['', '1+', '(1', '1**2', '1/0']) {
    assert.throws(() => decode(evaluate(input).state), input === '1/0' ? /Division by zero/ : /syntax/, input);
}
for (const input of ['3+1', '1 2', 'A', '1'.repeat(257)]) {
    assert.ok(evaluate(input).error, input);
}
const session = engine.Path.expression('12+2');
const completed = JSON.parse(session.run());
assert.equal(decode(completed.state).ternary, '21');
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
const rejected = engine.Path.expression('1/0');
const error = JSON.parse(rejected.run());
assert.throws(() => decode(error.state), /Division by zero/);
assert.ok(JSON.parse(rejected.inspect(error.event - 1)).event);
rejected.free();
console.log('Native Photonic expressions execute through Wasm, including signed arithmetic and errors.');

const start = performance.now();
const repeated = evaluate('2*2*2*2*2*2*2*2*2*2');
console.log(JSON.stringify({ repeated: { elapsed: performance.now() - start, event: repeated.event, work: repeated.work, state: repeated.state?.id } }));
assert.equal(decode(repeated.state).ternary, '1101221');
