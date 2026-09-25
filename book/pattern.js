(() => {
    'use strict';
    const book = globalThis.book ??= {};

    const parse = text => {
        const root = { kind: 'root', inner: [] };
        const stack = [root];
        for (const piece of book.syntax.scan(text)) {
            const top = stack.at(-1);
            if (piece.kind === 'space') continue;
            if (piece.kind === '(' || piece.kind === '[') {
                const node = { kind: piece.kind, inner: [] };
                top.inner.push(node);
                stack.push(node);
                continue;
            }
            if (piece.kind === ')' || piece.kind === ']') {
                if (top.kind !== (piece.kind === ')' ? '(' : '[')) throw new Error('The brackets in the pattern do not balance.');
                stack.pop();
                continue;
            }
            top.inner.push({ kind: piece.kind, text: piece.text });
        }
        if (stack.length > 1) throw new Error('The brackets in the pattern do not balance.');
        return root.inner;
    };

    const show = item => {
        if (item.kind === '(') return `(${canonical(item.inner)})`;
        if (item.kind === '[') return `[${canonical(item.inner)}]`;
        return item.text;
    };

    const print = item => item.map((value, index) => {
        const previous = item[index - 1];
        const gap = previous && value.kind !== ',' && value.kind !== '.' && previous.kind !== '.' ? ' ' : '';
        return gap + show(value);
    }).join('');

    const part = (item, strict) => {
        const result = [];
        let last = 'separator';
        for (const value of item) {
            if (value.kind === '.') {
                if (strict && last !== 'factor') throw new Error('Join atoms with single dots.');
                last = 'dot';
            } else if (value.kind === ',') {
                if (strict && last !== 'factor') throw new Error('A coherence in the pattern is empty.');
                last = 'separator';
            } else {
                if (strict && last === 'factor') throw new Error('Put a dot or a comma between the parts of the pattern.');
                if (last === 'dot') result.at(-1).push(value);
                else result.push([value]);
                last = 'factor';
            }
        }
        if (strict && last === 'dot') throw new Error('Join atoms with single dots.');
        if (strict && last === 'separator') throw new Error('A coherence in the pattern is empty.');
        return result;
    };

    const plain = value => value.length === 1 && value[0].kind === '(' && value[0].inner.length
        && !value[0].inner.some(item => item.kind === '[');

    const coherence = item => part(item, false)
        .flatMap(value => plain(value) ? coherence(value[0].inner) : [value.map(show).sort().join('.')]);

    const configuration = item => {
        if (item.some(value => value.kind === '[')) return print(item);
        return coherence(item).sort().join(', ');
    };

    const canonical = item => {
        if (item[0]?.kind !== '[') return configuration(item);
        const [context, ...rest] = item;
        return rest.length ? `[${configuration(context.inner)}] ${configuration(rest)}` : `[${configuration(context.inner)}]`;
    };

    const memory = new Map();
    const normal = text => {
        if (!memory.has(text)) {
            let value;
            try {
                value = canonical(parse(text));
            } catch {
                value = text;
            }
            memory.set(text, value);
        }
        return memory.get(text);
    };

    const term = value => {
        if (value.kind === 'concept') return `atom:${value.text}`;
        if (value.kind === '(' && value.inner[0]?.kind === '[') return `rule:${canonical(value.inner)}`;
        throw new Error('Parentheses in a pattern hold a rule value, such as ([A] B).');
    };

    const read = text => {
        const query = text.trim();
        if (!query) return undefined;
        const item = parse(query);
        if (item[0]?.kind === '[') return { rule: canonical(item), particle: [] };
        if (item.some(value => value.kind === '[')) throw new Error('Start a rule pattern with its input, such as [B, C] D.');
        return { particle: part(item, true).map(value => value.map(term)) };
    };

    const key = token => token.display?.startsWith('⟨') ? `rule:${normal(book.render.unwrap(token.display))}` : `atom:${token.label}`;

    const covers = (particle, world) => {
        const need = new Map();
        particle.forEach(value => need.set(value, (need.get(value) ?? 0) + 1));
        const used = new Set();
        for (const [value, count] of need) {
            const found = world.particle.filter(token => key(token) === value).slice(0, count);
            if (found.length < count) return undefined;
            found.forEach(token => used.add(token.id));
        }
        return used;
    };

    const assign = (pattern, node) => {
        const found = new Map();
        const place = index => {
            if (index === pattern.particle.length) return true;
            for (let world = 0; world < node.world.length; world++) {
                if (found.has(world)) continue;
                const used = covers(pattern.particle[index], node.world[world]);
                if (!used) continue;
                found.set(world, used);
                if (place(index + 1)) return true;
                found.delete(world);
            }
            return false;
        };
        return place(0) ? found : undefined;
    };

    const state = (pattern, data) => {
        const match = new Map();
        const rule = new Set();
        if (pattern.rule) {
            data.event.forEach(value => {
                if (normal(value.rule) === pattern.rule) rule.add(value.id);
            });
        } else {
            data.state.forEach(node => {
                const found = assign(pattern, node);
                if (found) match.set(node.id, found);
            });
        }
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

    const lane = (pattern, trace) => {
        const match = new Map();
        if (!pattern.rule) {
            trace.lifeline.forEach(line => {
                const used = pattern.particle.map(particle => covers(particle, line.world)).find(Boolean);
                if (used) match.set(line, used);
            });
        }
        const visible = new Set(match.keys());
        const shown = new Set();
        trace.hyperedge.forEach(edge => {
            const applies = pattern.rule && normal(edge.event.rule) === pattern.rule;
            if (!applies && !edge.input.some(line => visible.has(line))) return;
            shown.add(edge);
            edge.input.forEach(line => visible.add(line));
            edge.output.forEach(line => visible.add(line));
        });
        return { lifeline: visible, hyperedge: shown, match };
    };

    book.pattern = { read, state, lane };
})();
