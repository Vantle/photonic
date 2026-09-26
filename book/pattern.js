(() => {
    'use strict';
    const book = globalThis.book ??= {};

    const read = selection => ({
        match: new Map(selection.kind === 'configuration'
            ? selection.state.map(found => [found.id, new Map(found.world.map(entry => [entry.world, new Set(entry.token)]))])
            : []),
        rule: new Set(selection.kind === 'event' ? selection.event : []),
        lane: new Map(selection.kind === 'configuration' ? selection.lane.map(entry => [`${entry.state} ${entry.world}`, new Set(entry.token)]) : []),
    });

    const state = (selection, data) => {
        const { match, rule } = read(selection);
        const forward = new Set(match.keys());
        data.event.forEach(value => {
            if (rule.has(value.id)) forward.add(value.target);
        });
        const queue = [...forward];
        for (let index = 0; index < queue.length; index++) {
            for (const value of data.outgoing.get(queue[index]) ?? []) {
                if (forward.has(value.target)) continue;
                forward.add(value.target);
                queue.push(value.target);
            }
        }
        const visible = new Set(forward);
        data.event.forEach(value => {
            if (rule.has(value.id)) visible.add(value.source);
        });
        const shown = new Set(data.event.filter(value => rule.has(value.id) || forward.has(value.source)).map(value => value.id));
        return { state: visible, event: shown, match };
    };

    const lane = (selection, trace) => {
        const { rule, lane: covered } = read(selection);
        const match = new Map(trace.lifeline.filter(line => covered.has(`${line.state} ${line.index}`)).map(line => [line, covered.get(`${line.state} ${line.index}`)]));
        const visible = new Set(match.keys());
        const shown = new Set();
        trace.hyperedge.forEach(edge => {
            if (!rule.has(edge.event.id) && !edge.input.some(line => visible.has(line))) return;
            shown.add(edge);
            edge.input.forEach(line => visible.add(line));
            edge.output.forEach(line => visible.add(line));
        });
        return { lifeline: visible, hyperedge: shown, match };
    };

    book.pattern = { state, lane };
})();
