(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const depth = 128;
    const budget = 1000000;
    const start = new Set(['[', '(', 'concept']);
    const advice = {
        ',': 'A comma separates two things; put something on each side',
        '.': 'A dot joins two things; put something on each side',
        ')': 'This closes nothing that is open here',
        ']': 'This closes nothing that is open here',
        end: 'Close what is still open',
    };
    const beside = 'Put a dot between these to join them, or a comma to separate them';
    const joined = 'A rule joins a particle inside parentheses, as in X.([A] B)';

    const refuse = (message, piece) => {
        throw Object.assign(new Error(message), piece && { detail: { span: { offset: piece.offset, length: piece.text.length } } });
    };

    const tokenize = text => {
        const piece = [];
        let offset = 0;
        for (const value of book.syntax.scan(text)) {
            if (value.kind !== 'space') piece.push({ kind: value.kind, text: value.text, offset });
            offset += value.text.length;
        }
        piece.push({ kind: 'end', text: '', offset });
        return piece;
    };

    const nest = piece => {
        let level = 0;
        for (const value of piece) {
            if (value.kind === '(' || value.kind === '[') level++;
            if (value.kind === ')' || value.kind === ']') level = Math.max(0, level - 1);
            if (level > depth) refuse(`Nesting exceeds ${depth} levels`, value);
        }
    };

    const parse = text => {
        const piece = tokenize(text);
        nest(piece);
        let index = 0;
        const peek = () => piece[index];
        const list = close => {
            const result = [];
            while (peek().kind !== close) {
                if (!start.has(peek().kind)) refuse(advice[peek().kind], peek());
                result.push(term());
                const next = peek();
                if (next.kind === ',') index++;
                else if (next.kind === '.') refuse(joined, next);
                else if (next.kind !== close) refuse(advice[next.kind], next);
            }
            index++;
            return result;
        };
        const term = () => {
            const bracket = [];
            let join = [];
            for (let value = peek(); start.has(value.kind); value = peek()) {
                if (value.kind === '[') {
                    index++;
                    bracket.push(list(']'));
                    continue;
                }
                if (join.length) refuse(beside, value);
                join = chain();
            }
            return { bracket, join };
        };
        const chain = () => {
            const result = [factor()];
            while (peek().kind === '.') {
                index++;
                const value = peek();
                if (value.kind === '[') refuse(joined, value);
                if (value.kind !== 'concept' && value.kind !== '(') refuse(advice['.'], value);
                result.push(factor());
            }
            return result;
        };
        const factor = () => {
            const value = piece[index++];
            if (value.kind === 'concept') return { atom: value.text };
            return { group: list(')') };
        };
        return list('end');
    };

    const shorthand = tree => tree.map(term => {
        const [only] = term.join;
        if (term.bracket.length || term.join.length !== 1 || !only.group?.length) return term;
        if (!only.group.every(inner => inner.bracket.length)) return term;
        return { bracket: [], join: [{ group: [] }, only] };
    });

    const lower = tree => {
        let remaining = budget;
        const spend = amount => {
            remaining -= amount;
            if (remaining < 0) refuse(`The pattern expands past ${budget.toLocaleString()} units; write it with fewer groups.`);
        };
        const size = value => value.reduce((sum, particle) => sum + 1 + particle.length, 0);
        const member = list => list.flatMap(term => term.bracket.length ? [{ rule: partition(term) }] : body(term.join));
        const body = factor => {
            if (!factor.length) return [];
            if (factor.length === 1 && factor[0].group) return group(factor[0].group);
            return join(factor).map(particle => ({ particle }));
        };
        const group = list => {
            const value = member(list);
            if (!value.length) return [{ particle: [] }];
            if (!value.some(entry => entry.rule)) return value;
            return [scope(value)];
        };
        const scope = value => {
            if (value.some(entry => entry.scope)) refuse('A scope cannot hold another scope; to keep a rule in its coherence, join it, as in ().([A] B).');
            const particle = value.filter(entry => entry.particle).map(entry => entry.particle);
            if (particle.length > 1) refuse(`A scope holds at most one coherence; found ${particle.length}.`);
            return { scope: { particle: particle[0] ?? [], body: value.flatMap(entry => entry.rule ?? []) } };
        };
        const combine = (left, right) => {
            if (left.length === 1 && right.length === 1) return [[...left[0], ...right[0]]];
            spend(size(left) * right.length + size(right) * left.length);
            return left.flatMap(first => right.map(second => [...first, ...second]));
        };
        const join = factor => factor.reduce((result, node) => combine(result, expand(node)), [[]]);
        const expand = node => {
            if (node.atom !== undefined) return [[{ atom: node.atom }]];
            if (!node.group.length) return [[]];
            return node.group.flatMap(term => term.bracket.length ? [partition(term).map(rule => ({ rule }))] : join(term.join));
        };
        const input = list => member(list).map(entry => {
            if (entry.scope) refuse('An input cannot be a scope; match a rule without parentheses, as in [[A] B].');
            return entry.particle ?? entry.rule.map(rule => ({ rule }));
        });
        const partition = term => {
            const pattern = term.bracket.map(input);
            const output = body(term.join).map(entry => entry.scope ?? { particle: entry.particle });
            if (pattern.length === 1) return [{ input: pattern[0], output }];
            const rule = pattern.flatMap((source, index) => [
                ...pattern.filter((_, other) => other !== index).map(target => target.map(particle => ({ particle }))),
                ...(output.length ? [output] : []),
            ].map(target => ({ input: source, output: target })));
            spend(rule.length);
            return rule;
        };
        const listed = member(tree);
        if (listed.some(entry => entry.scope)) refuse('Only a rule’s output opens a scope; to keep a rule in a coherence, join it, as in ().([A] B).');
        return {
            initial: listed.filter(entry => entry.particle).map(entry => entry.particle),
            rule: listed.flatMap(entry => entry.rule ?? []),
        };
    };

    const key = {
        value: value => value.rule ? `rule:${key.rule(value.rule)}` : `atom:${value.atom}`,
        particle: value => JSON.stringify(value.map(key.value).sort()),
        output: value => JSON.stringify([key.particle(value.particle), value.body ? value.body.map(key.rule).sort() : null]),
        rule: value => JSON.stringify([value.input.map(key.particle).sort(), value.output.map(key.output).sort()]),
    };

    const canonical = text => {
        try {
            const program = lower(parse(text));
            return program.rule.length === 1 && !program.initial.length ? key.rule(program.rule[0]) : undefined;
        } catch {
            return undefined;
        }
    };

    const memory = new Map();
    const normal = text => {
        if (!memory.has(text)) memory.set(text, canonical(text));
        return memory.get(text);
    };

    const read = text => {
        if (!text.trim()) return undefined;
        const program = lower(shorthand(parse(text)));
        if (program.rule.length && program.initial.length) refuse('Search for coherences or for rules, such as B.X or [B, C] D, not both.');
        if (program.rule.length) return { rule: new Set(program.rule.map(key.rule)) };
        return { particle: program.initial.map(particle => particle.map(key.value)) };
    };

    const occurrence = (token, definition) => token.kind === 'rule' ? `rule:${normal(definition[token.rule])}` : `atom:${token.label}`;

    const covers = (particle, world, definition) => {
        const need = new Map();
        particle.forEach(value => need.set(value, (need.get(value) ?? 0) + 1));
        const used = new Set();
        for (const [value, count] of need) {
            const found = world.particle.filter(token => occurrence(token, definition) === value).slice(0, count);
            if (found.length < count) return undefined;
            found.forEach(token => used.add(token.id));
        }
        return used;
    };

    const assign = (pattern, node, definition) => {
        const found = new Map();
        const place = index => {
            if (index === pattern.particle.length) return true;
            for (let world = 0; world < node.world.length; world++) {
                if (found.has(world)) continue;
                const used = covers(pattern.particle[index], node.world[world], definition);
                if (!used) continue;
                found.set(world, used);
                if (place(index + 1)) return true;
                found.delete(world);
            }
            return false;
        };
        return place(0) ? found : undefined;
    };

    const select = (pattern, data) => pattern.rule
        ? { kind: 'event', match: new Set(data.event.filter(value => pattern.rule.has(normal(value.rule))).map(value => value.id)) }
        : { kind: 'configuration', match: new Map(data.state.map(node => [node.id, assign(pattern, node, data.definition)]).filter(([, found]) => found)) };

    const state = (pattern, data) => {
        const found = select(pattern, data);
        const match = found.kind === 'configuration' ? found.match : new Map();
        const rule = found.kind === 'event' ? found.match : new Set();
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

    const lane = (pattern, trace, definition) => {
        const match = new Map();
        if (!pattern.rule) {
            trace.lifeline.forEach(line => {
                const used = pattern.particle.map(particle => covers(particle, line.world, definition)).find(Boolean);
                if (used) match.set(line, used);
            });
        }
        const visible = new Set(match.keys());
        const shown = new Set();
        trace.hyperedge.forEach(edge => {
            const applies = pattern.rule?.has(normal(edge.event.rule));
            if (!applies && !edge.input.some(line => visible.has(line))) return;
            shown.add(edge);
            edge.input.forEach(line => visible.add(line));
            edge.output.forEach(line => visible.add(line));
        });
        return { lifeline: visible, hyperedge: shown, match };
    };

    book.pattern = { read, select, state, lane };
})();
