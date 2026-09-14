import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';

const contains = (world, label) => world.particle.some(value => value.display === label);
const method = world => world.particle.find(value => value.display.startsWith('⟨[Read.Head.'));
const walk = (state, head) => {
    let current = head;
    const token = [];
    const visited = new Set();
    while (!contains(current, 'Zero')) {
        const read = method(current);
        assert.ok(read);
        assert.ok(!visited.has(read.capture));
        visited.add(read.capture);
        const value = read.display.match(/(?:Emit\.([012])|Yield\.(Add|Subtract|Multiply|Divide|Open|Close))\)/);
        assert.ok(value);
        token.push(value[1] ?? value[2]);
        const cell = state.world.filter(world => contains(world, 'Cell') &&
            world.particle.some(value => value.display === '⟨[Seal] ()⟩' && value.capture === read.capture) &&
            method(world)?.capture !== read.capture);
        assert.equal(cell.length, 1);
        current = cell[0];
    }
    return token;
};
const decode = (state, head) => {
    const token = walk(state, head);
    assert.ok(token.every(value => /^[012]$/.test(value)));
    const ternary = token.reverse().join('').replace(/^0+(?=.)/, '') || '0';
    const decimal = [...ternary].reduce((value, digit) => value * 3n + BigInt(digit), 0n);
    return { ternary, decimal: String(decimal) };
};

export const record = (command, program, target) => {
    const report = JSON.parse(execFileSync(command, [
        'prism', program, '--target', target, '--path', '--json',
        '--steps', '100000000', '--states', '65536', '--cells', '16384',
        '--frames', '2048', '--coherences', '1024', '--records', '100000000',
    ], { encoding: 'utf8', maxBuffer: 536870912 }));
    assert.equal(report.outcome, 'reached');
    const begin = report.event.find(event => event.rule.startsWith('[Function.Expression]'));
    assert.ok(begin);
    const state = report.state[begin.source];
    const head = state.world.filter(world => ['Function', 'Expression'].every(label => contains(world, label)));
    assert.equal(head.length, 1);
    const input = walk(state, head[0]);
    const event = report.event.filter(event => event.rule.startsWith('[Return.Integer.Number.Positive,Evaluate.Pending.')).map(event => {
        const state = report.state[event.source];
        const head = state.world.filter(world => ['Return', 'Integer', 'Number', 'Positive'].every(label => contains(world, label)));
        assert.equal(head.length, 1);
        return { operation: event.rule.match(/Evaluate\.Pending\.(\w+)/)[1], ...decode(state, head[0]), source: event.source, target: event.target, rule: event.rule };
    });
    return { outcome: report.outcome, work: report.work, count: report.event.length, input, event };
};
