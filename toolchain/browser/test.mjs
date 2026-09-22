import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const engine = await import(pathToFileURL(process.argv[3]));
engine.initSync({ module: await readFile(process.argv[2]) });
const invoke = request => JSON.parse(engine.execute(JSON.stringify(request)));
for (const source of [
    'A [A] B',
    'A [A] B.C [B] D',
    'Seed.A [Seed] [A] B',
    'A.X, B.Y [A, B] (C, D) [C, D] E',
    'A [A] A.A',
]) {
    const path = join(process.env.TEST_TMPDIR, 'source.wave');
    await writeFile(path, source);
    const native = spawnSync(process.argv[4], ['run', path, '--steps', '20000', '--states', '128', '--cells', '128', '--frames', '16', '--coherences', '16', '--records', '100000', '--json'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    assert.equal(native.status, 0, native.error?.message || native.stderr);
    const response = invoke({ version: 1, source, targets: [] });
    assert.equal(response.error, undefined);
    assert.deepEqual(response.execution, JSON.parse(native.stdout), source);
}
assert.deepEqual(invoke({ version: 1, source: 'A [A] B', targets: ['B [A] B', 'C [A] B'] }).verdict.map(value => value.outcome), ['reached', 'unreachable']);
assert.equal(invoke({ version: 2, source: 'A' }).error.code, 'version');
assert.equal(invoke({ version: 1, source: '[', targets: [] }).error.code, 'source');
assert.equal(invoke({ version: 1, source: 'A', targets: ['A [A] B'] }).verdict[0].outcome, 'unreachable');
assert.ok(invoke({ version: 1, source: 'A', targets: Array(17).fill('A') }).error);
assert.equal(invoke({ version: 1, source: 'A', extra: true }).error.code, 'request');
assert.equal(JSON.parse(engine.execute('[')).error.code, 'request');
assert.ok(invoke({version: 1, source: "A".repeat(32769)}).error);
assert.equal(invoke({ version: 1, source: 'A', targets: ['A'] }).verdict[0].outcome, 'reached');
console.log('WebAssembly matches native Rust reports, including suspended exploration and generated code.');

const { decode } = await import(pathToFileURL(process.argv[5]));
for (const [input, expected] of [
    ['12 + 2', '21'], ['12 * 2', '101'], ['21 / 2 - 1', '2'],
    ['-(12 + 2) * 10', '-210'], ['-21 / 2', '-10'],
    ['1212 * 10 / 2 + 11 - 1', '2220'], ['0', '0'], ['00012', '12'],
    ['2 + 1 * 2', '11'], ['(2 + 1) * 2', '20'], ['--2', '2'], ['-(12+2)*2', '-112'], ['(-2)*(-2)', '11'], ['(-2)*0', '0'],
]) {
    const result = JSON.parse(engine.calculate(input));
    assert.equal(result.error, undefined, input);
    assert.equal(decode(result.state).ternary, expected, input);
}
for (const input of ['', '1+', '(1', '1**2', '1/0']) {
    const result = JSON.parse(engine.calculate(input));
    assert.throws(() => decode(result.state), input === '1/0' ? /Division by zero/ : /syntax/, input);
}
for (const input of ['3+1', '1 2', 'A', '1'.repeat(257)]) {
    assert.ok(JSON.parse(engine.calculate(input)).error, input);
}
console.log('Native Photonic expressions execute through Wasm, including signed arithmetic and errors.');

for (let left = 0; left < 9; left++) for (let right = 0; right < 9; right++) {
    const result = JSON.parse(engine.multiply(left, right));
    assert.equal(result.error, undefined);
    assert.equal(result.ternary, (left * right).toString(3));
    for (const pair of result.pair) assert.equal(pair.digit + 3 * pair.carry, pair.left * pair.right);
    for (const column of result.column) assert.equal(column.digit + 3 * column.carry, column.contribution + column.incoming);
}
assert.ok(JSON.parse(engine.multiply(9, 0)).error);

const session = new engine.Evaluation('12+2');
const completed = JSON.parse(session.run());
assert.equal(decode(completed.state).ternary, '21');
const first = JSON.parse(session.inspect(0));
assert.equal(first.before.id, 0);
assert.equal(first.after.id, first.event.target);
assert.match(first.event.rule, /Push/);
const last = JSON.parse(session.inspect(completed.event - 1));
assert.deepEqual(last.after, completed.state);
assert.ok(JSON.parse(session.inspect(completed.event)).error);
assert.deepEqual(JSON.parse(session.inspect(0)), first);
session.free();
const rejected = new engine.Evaluation('1/0');
const error = JSON.parse(rejected.run());
assert.throws(() => decode(error.state), /Division by zero/);
assert.ok(JSON.parse(rejected.inspect(error.event - 1)).event);
rejected.free();

const start = performance.now();
const repeated = JSON.parse(engine.calculate('2*2*2*2*2*2*2*2*2*2'));
console.log(JSON.stringify({ repeated: { elapsed: performance.now() - start, event: repeated.event, work: repeated.work, state: repeated.state?.id } }));
assert.equal(decode(repeated.state).ternary, '1101221');
