const atom = (world, label) => world.particle.some(token => token.kind === 'atom' && token.label === label);

const rule = (world, definition) => world.particle
    .filter(token => token.kind === 'rule')
    .map(token => ({ text: definition[token.rule], capture: token.capture }));

const method = (world, definition) => rule(world, definition).find(value => value.text.startsWith('[Read.Head.'));

export const decode = (state, definition) => {
    const head = state.world.filter(world => ['Return', 'Expression', 'Evaluate'].every(label => atom(world, label)));
    if (head.length !== 1) throw Error('No completed expression within the execution budget. Try a smaller expression.');
    let current = head[0];
    if (atom(current, 'Error')) {
        if (atom(current, 'Divisor')) throw Error('Division by zero.');
        throw Error('The Photonic evaluator rejected the expression syntax.');
    }
    if (!atom(current, 'Positive') && !atom(current, 'Negative')) throw Error('The evaluator did not return a number.');
    const negative = atom(current, 'Negative');
    const digit = [];
    const visited = new Set();
    while (!atom(current, 'Zero')) {
        const read = method(current, definition);
        if (read?.capture === undefined || visited.has(read.capture)) throw Error('Invalid native numeral.');
        visited.add(read.capture);
        const cell = state.world.filter(world => atom(world, 'Cell') &&
            rule(world, definition).some(value => value.text === '[Seal] ()' && value.capture === read.capture) &&
            method(world, definition)?.capture !== read.capture);
        if (cell.length !== 1) throw Error('Invalid native numeral link.');
        const value = rule(cell[0], definition).map(entry => entry.text.match(/^\[Digit\] ([012])$/)?.[1]).filter(Boolean);
        if (value.length !== 1) throw Error('Invalid native numeral.');
        digit.push(value[0]);
        current = cell[0];
    }
    const magnitude = digit.reverse().join('').replace(/^0+(?=.)/, '') || '0';
    const sign = negative && magnitude !== '0' ? '-' : '';
    const decimal = [...magnitude].reduce((total, figure) => total * 3n + BigInt(figure), 0n);
    return { ternary: sign + magnitude, decimal: sign + decimal };
};

export const answer = (state, definition) => {
    try {
        return decode(state, definition);
    } catch (error) {
        return { error: error.message };
    }
};
