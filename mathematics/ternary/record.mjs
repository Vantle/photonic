import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';

const [command, program, target] = process.argv.slice(2);
const report = JSON.parse(execFileSync(command, [
    'obsidian', program, '--target', target, '--path', '--json',
    '--steps', '100000000', '--states', '65536', '--cells', '16384',
    '--frames', '2048', '--coherences', '1024', '--records', '100000000',
], { encoding: 'utf8', maxBuffer: 536870912 }));
assert.equal(report.outcome, 'reached');
assert.ok(report.work <= 3000000, `Expression work regressed: ${report.work}`);
assert.ok(report.event.length <= 10200, `Expression event count regressed: ${report.event.length}`);

const contains = (world, label) => world.particle.some(value => value.display === label);
const method = world => world.particle.find(value => value.display.startsWith('⟨[Read.Head.'));
const decode = (state, head) => {
    let current = head;
    let digit = '';
    const visited = new Set();
    while (!contains(current, 'Zero')) {
        const read = method(current);
        assert.ok(read);
        assert.ok(!visited.has(read.capture));
        visited.add(read.capture);
        const value = read.display.match(/Emit\.([012])\)/);
        assert.ok(value);
        digit = value[1] + digit;
        const cell = state.world.filter(world => contains(world, 'Cell') &&
            world.particle.some(value => value.display === '⟨[Seal] ()⟩' && value.capture === read.capture) &&
            method(world)?.capture !== read.capture);
        assert.equal(cell.length, 1);
        current = cell[0];
    }
    const ternary = digit.replace(/^0+(?=.)/, '') || '0';
    const decimal = [...ternary].reduce((value, digit) => value * 3n + BigInt(digit), 0n);
    return { ternary, decimal: String(decimal) };
};

const event = report.event.filter(event => event.rule.startsWith('[Return.Integer.Number.Positive,Evaluate.Pending.')).map(event => {
    const state = report.state[event.source];
    const head = state.world.filter(world => ['Return', 'Integer', 'Number', 'Positive'].every(label => contains(world, label)));
    assert.equal(head.length, 1);
    return { operation: event.rule.match(/Evaluate\.Pending\.(\w+)/)[1], ...decode(state, head[0]), source: event.source, target: event.target, rule: event.rule };
});
assert.deepEqual(event.map(value => value.ternary), ['12120', '2210', '2221', '2220']);
process.stdout.write(`globalThis.expression = ${JSON.stringify({ outcome: report.outcome, work: report.work, count: report.event.length, event })};\n`);
