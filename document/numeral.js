export const decode = state => {
    const contains = (world, label) => world.particle.some(value => value.display === label);
    const method = world => world.particle.find(value => value.display.startsWith('⟨[Read.Head.'));
    const head = state.world.filter(world => ['Return', 'Expression', 'Evaluate'].every(label => contains(world, label)));
    if (head.length !== 1) throw Error('No completed expression within the execution budget. Try a smaller expression.');
    let current = head[0];
    if (contains(current, 'Error')) {
        if (contains(current, 'Divisor')) throw Error('Division by zero.');
        throw Error('The Photonic evaluator rejected the expression syntax.');
    }
    if (!contains(current, 'Positive') && !contains(current, 'Negative')) throw Error('The evaluator did not return a number.');
    const negative = contains(current, 'Negative');
    const digit = [];
    const visited = new Set();
    while (!contains(current, 'Zero')) {
        const read = method(current);
        if (read?.capture === undefined || visited.has(read.capture)) throw Error('Invalid native numeral.');
        visited.add(read.capture);
        const cell = state.world.filter(world => contains(world, 'Cell') &&
            world.particle.some(value => value.display === '⟨[Seal] ()⟩' && value.capture === read.capture) &&
            method(world)?.capture !== read.capture);
        if (cell.length !== 1) throw Error('Invalid native numeral link.');
        const value = cell[0].particle.map(value => value.display.match(/^⟨\[Digit\] ([012])⟩$/)?.[1]).filter(Boolean);
        if (value.length !== 1) throw Error('Invalid native numeral.');
        digit.push(value[0]);
        current = cell[0];
    }
    const magnitude = digit.reverse().join('').replace(/^0+(?=.)/, '') || '0';
    const sign = negative && magnitude !== '0' ? '-' : '';
    const decimal = [...magnitude].reduce((value, digit) => value * 3n + BigInt(digit), 0n);
    return { ternary: sign + magnitude, decimal: sign + decimal };
};
