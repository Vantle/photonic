import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { record } from './trace.mjs';

const result = record(...process.argv.slice(2));
assert.deepEqual(result.input, ['1', '2', '1', '2', 'Multiply', '1', '0', 'Divide', '2', 'Add', '1', '1', 'Subtract', '1']);
assert.ok(result.work <= 250000, `Expression work regressed: ${result.work}`);
assert.ok(result.count <= 10200, `Expression event count regressed: ${result.count}`);
assert.deepEqual(result.event.map(value => value.ternary), ['12120', '2210', '2221', '2220']);
const output = `globalThis.expression = ${JSON.stringify(result)};\n`;
if (process.argv[5]) assert.equal(output, readFileSync(process.argv[5], 'utf8'));
process.stdout.write(output);
