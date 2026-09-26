import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const label = (definition, value) => value.atom ?? `⟨${definition[value.rule].name}⟩`;
const contains = (definition, world, name) => world.particle.some(value => label(definition, value) === name);
const method = (definition, world) => world.particle.find(value => label(definition, value).startsWith('⟨[Read.Head.'));
const symbol = (definition, value) => label(definition, value).match(/^⟨\[Digit\] ([012])⟩$/)?.[1] ??
    ['Add', 'Subtract', 'Multiply', 'Divide', 'Open', 'Close'].find(name => name === value.atom);
const walk = (definition, state, head) => {
    let current = head;
    const token = [];
    const visited = new Set();
    while (!contains(definition, current, 'Zero')) {
        const read = method(definition, current);
        assert.ok(read);
        assert.ok(!visited.has(read.capture));
        visited.add(read.capture);
        const cell = state.world.filter(world => contains(definition, world, 'Cell') &&
            world.particle.some(value => label(definition, value) === '⟨[Seal] ()⟩' && value.capture === read.capture) &&
            method(definition, world)?.capture !== read.capture);
        assert.equal(cell.length, 1);
        const value = cell[0].particle.map(value => symbol(definition, value)).filter(Boolean);
        assert.equal(value.length, 1);
        token.push(value[0]);
        current = cell[0];
    }
    return token;
};
const decode = (definition, state, head) => {
    const token = walk(definition, state, head);
    assert.ok(token.every(value => /^[012]$/.test(value)));
    const ternary = token.reverse().join('').replace(/^0+(?=.)/, '') || '0';
    const decimal = [...ternary].reduce((value, digit) => value * 3n + BigInt(digit), 0n);
    return { ternary, decimal: String(decimal) };
};

// A direct path through the expression engine prints more JSON than one JavaScript string can hold,
// so the report stays in bytes and a configuration is parsed only when a check reads it.
const split = (byte, begin, end) => {
    assert.ok(byte[begin] === 0x5b || byte[begin] === 0x7b, 'the report is compact JSON');
    const range = [];
    let depth = 0;
    let quoted = false;
    let start = begin + 1;
    for (let index = start; index < end - 1; index += 1) {
        const value = byte[index];
        if (quoted) {
            if (value === 0x5c) index += 1;
            else if (value === 0x22) quoted = false;
        } else if (value === 0x22) quoted = true;
        else if (value === 0x5b || value === 0x7b) depth += 1;
        else if (value === 0x5d || value === 0x7d) depth -= 1;
        else if (value === 0x2c && depth === 0) {
            range.push([start, index]);
            start = index + 1;
        }
    }
    if (start < end - 1) range.push([start, end - 1]);
    return range;
};
const read = byte => {
    const text = ([start, end]) => byte.toString('utf8', start, end);
    const field = Object.fromEntries(split(byte, byte.indexOf('{'), byte.lastIndexOf('}') + 1).map(([start, end]) => {
        const colon = byte.indexOf('":', start) + 1;
        return [JSON.parse(text([start, colon])), [colon + 1, end]];
    }));
    const configuration = split(byte, ...field.state);
    return {
        definition: JSON.parse(text(field.definition)),
        outcome: JSON.parse(text(field.outcome)),
        work: JSON.parse(text(field.work)),
        event: JSON.parse(text(field.event)),
        state: index => JSON.parse(text(configuration[index])),
    };
};

export const record = (command, program, target) => {
    const source = JSON.parse(readFileSync(program, 'utf8'));
    const value = JSON.parse(execFileSync(command, ['lower', target], { encoding: 'utf8' }));
    const directory = mkdtempSync(join(tmpdir(), 'photonic-target-'));
    const configuration = join(directory, 'target.json');
    writeFileSync(configuration, JSON.stringify({ initial: value.initial, rule: [...value.rule, ...source.rule] }));
    let report;
    try {
        report = read(execFileSync(command, [
        'prism', program, '--target', configuration, '--path', '--json', '--compact',
        '--work', '100000000', '--configuration', '65536', '--occurrence', '16384',
        '--scope', '2048', '--coherence', '1024', '--record', '100000000',
    ], { maxBuffer: 2 ** 31 }));
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
    assert.equal(report.outcome, 'reached');
    const { definition } = report;
    const rule = event => definition[event.rule].name;
    const begin = report.event.find(event => rule(event).startsWith('[Function.Expression.Evaluate]'));
    assert.ok(begin);
    const state = report.state(begin.source);
    const head = state.world.filter(world => ['Function', 'Expression', 'Evaluate'].every(name => contains(definition, world, name)));
    assert.equal(head.length, 1);
    const input = walk(definition, state, head[0]);
    const event = report.event.filter(event => /^\[Return\.Integer\.\w+\.Positive, Execute\.Pending\./.test(rule(event))).map(event => {
        const operation = rule(event).match(/Execute\.Pending\.(\w+)/)[1];
        const state = report.state(event.source);
        const head = state.world.filter(world => ['Return', 'Integer', operation, 'Positive'].every(name => contains(definition, world, name)));
        assert.equal(head.length, 1);
        return { operation, ...decode(definition, state, head[0]), source: event.source, target: event.target, rule: rule(event) };
    });
    return { outcome: report.outcome, work: report.work, count: report.event.length, input, event };
};
