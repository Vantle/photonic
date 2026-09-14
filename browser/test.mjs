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
assert.deepEqual(invoke({ version: 1, source: 'A [A] B', targets: ['B', 'C'] }).verdict.map(value => value.outcome), ['reached', 'unreachable']);
assert.equal(invoke({ version: 2, source: 'A' }).error.code, 'version');
assert.equal(invoke({ version: 1, source: '[', targets: [] }).error.code, 'source');
assert.ok(invoke({ version: 1, source: 'A', targets: ['A [A] B'] }).error);
assert.ok(invoke({ version: 1, source: 'A', targets: Array(17).fill('A') }).error);
assert.equal(invoke({ version: 1, source: 'A', extra: true }).error.code, 'request');
assert.equal(JSON.parse(engine.execute('[')).error.code, 'request');
assert.ok(invoke({version: 1, source: "A".repeat(32769)}).error);
assert.equal(invoke({ version: 1, source: 'A', targets: ['A'] }).verdict[0].outcome, 'reached');
console.log('WebAssembly matches native Rust reports, including suspended exploration and generated code.');
