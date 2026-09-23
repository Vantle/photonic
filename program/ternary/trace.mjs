import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const contains = (world, label) => world.particle.some(value => value.display === label);
const method = world => world.particle.find(value => value.display.startsWith('⟨[Read.Head.'));
const symbol = value => value.display.match(/^⟨\[Digit\] ([012])⟩$/)?.[1] ??
    ['Add', 'Subtract', 'Multiply', 'Divide', 'Open', 'Close'].find(name => name === value.display);
const walk = (state, head) => {
    let current = head;
    const token = [];
    const visited = new Set();
    while (!contains(current, 'Zero')) {
        const read = method(current);
        assert.ok(read);
        assert.ok(!visited.has(read.capture));
        visited.add(read.capture);
        const cell = state.world.filter(world => contains(world, 'Cell') &&
            world.particle.some(value => value.display === '⟨[Seal] ()⟩' && value.capture === read.capture) &&
            method(world)?.capture !== read.capture);
        assert.equal(cell.length, 1);
        const value = cell[0].particle.map(symbol).filter(Boolean);
        assert.equal(value.length, 1);
        token.push(value[0]);
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
    const source = JSON.parse(readFileSync(program, 'utf8'));
    const value = JSON.parse(execFileSync(command, ['lower', target], { encoding: 'utf8' }));
    const directory = mkdtempSync(join(tmpdir(), 'photonic-target-'));
    const configuration = join(directory, 'target.json');
    writeFileSync(configuration, JSON.stringify({ initial: value.initial, rule: [...value.rule, ...source.rule] }));
    let report;
    try {
        report = JSON.parse(execFileSync(command, [
        'prism', program, '--target', configuration, '--path', '--json', '--compact',
        '--steps', '100000000', '--states', '65536', '--cells', '16384',
        '--frames', '2048', '--coherences', '1024', '--records', '100000000',
    ], { encoding: 'utf8', maxBuffer: 536870912 }));
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
    assert.equal(report.outcome, 'reached');
    const begin = report.event.find(event => event.rule.startsWith('[Function.Expression.Evaluate]'));
    assert.ok(begin);
    const state = report.state[begin.source];
    const head = state.world.filter(world => ['Function', 'Expression', 'Evaluate'].every(label => contains(world, label)));
    assert.equal(head.length, 1);
    const input = walk(state, head[0]);
    const event = report.event.filter(event => /^\[Return\.Integer\.\w+\.Positive, Execute\.Pending\./.test(event.rule)).map(event => {
        const operation = event.rule.match(/Execute\.Pending\.(\w+)/)[1];
        const state = report.state[event.source];
        const head = state.world.filter(world => ['Return', 'Integer', operation, 'Positive'].every(label => contains(world, label)));
        assert.equal(head.length, 1);
        return { operation, ...decode(state, head[0]), source: event.source, target: event.target, rule: event.rule };
    });
    return { outcome: report.outcome, work: report.work, count: report.event.length, input, event };
};
