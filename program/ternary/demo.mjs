import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { record } from './trace.mjs';

const [command, ...input] = process.argv.slice(2);
const definition = [
    { name: 'Addition', expression: '12 + 2', input: ['1', '2', 'Add', '2'], expected: ['21'] },
    { name: 'Multiplication', expression: '12 × 2', input: ['1', '2', 'Multiply', '2'], expected: ['101'] },
    { name: 'Division and subtraction', expression: '21 ÷ 2 − 1', input: ['2', '1', 'Divide', '2', 'Subtract', '1'], expected: ['10', '2'] },
];
const reference = input.length === definition.length * 2 + 1 ? input.pop() : undefined;
assert.equal(input.length, definition.length * 2);
const demo = definition.map((value, index) => {
    const [program, target] = input.slice(index * 2, index * 2 + 2);
    const result = record(command, program, target);
    assert.deepEqual(result.input, value.input);
    assert.deepEqual(result.event.map(event => event.ternary), value.expected);
    return { name: value.name, expression: value.expression, ...result };
});
const output = `${JSON.stringify(demo, null, 2)}\n`;
if (reference) assert.equal(output, readFileSync(reference, 'utf8'));
process.stdout.write(output);
