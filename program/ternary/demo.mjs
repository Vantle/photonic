import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { record } from './trace.mjs';

const [command, ...input] = process.argv.slice(2);
const reference = input.length === 10 ? input.pop() : undefined;
const definition = [
    { name: 'Addition', expression: '12 + 2', input: ['1', '2', 'Add', '2'], expected: ['21'], source: 'addition' },
    { name: 'Multiplication', expression: '12 × 2', input: ['1', '2', 'Multiply', '2'], expected: ['101'], source: 'multiplication' },
    { name: 'Division and subtraction', expression: '21 ÷ 2 − 1', input: ['2', '1', 'Divide', '2', 'Subtract', '1'], expected: ['10', '2'], source: 'division' },
];
assert.equal(input.length, definition.length * 3);
const demo = definition.map((value, index) => {
    const [program, target, source] = input.slice(index * 3, index * 3 + 3);
    const result = record(command, program, target);
    assert.deepEqual(result.input, value.input);
    assert.deepEqual(result.event.map(event => event.ternary), value.expected);
    return { name: value.name, expression: value.expression, path: `program/ternary/demo/${value.source}.wave`, command: `bazel test -c opt //program/ternary:demo.${value.source}.check --test_output=all`, source: readFileSync(source, 'utf8'), ...result };
});
const output = `globalThis.demo = ${JSON.stringify(demo)};\n`;
if (reference) assert.equal(output, readFileSync(reference, 'utf8'));
process.stdout.write(output);
