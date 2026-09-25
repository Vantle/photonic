(() => {
    'use strict';
    const book = globalThis.book ??= {};

    const element = (tag, style, text) => {
        const node = document.createElement(tag);
        if (style) node.className = style;
        if (text !== undefined) node.textContent = text;
        return node;
    };

    const vector = (tag, attribute = {}) => {
        const node = document.createElementNS('http://www.w3.org/2000/svg', tag);
        for (const [name, value] of Object.entries(attribute)) node.setAttribute(name, value);
        return node;
    };

    const message = () => {
        const node = element('p', 'message');
        node.setAttribute('role', 'status');
        node.hidden = true;
        let timer;
        const say = (text, tone) => {
            clearTimeout(timer);
            node.hidden = !text;
            node.textContent = text ?? '';
            if (tone) node.dataset.tone = tone;
            else delete node.dataset.tone;
        };
        const wait = text => {
            clearTimeout(timer);
            timer = setTimeout(() => say(text), 400);
        };
        return { element: node, say, wait };
    };

    const tally = (number, word) => {
        const node = element('span');
        node.append(element('b', undefined, number.toLocaleString()), ` ${word}${number === 1 ? '' : 's'}`);
        return node;
    };

    const legend = entry => {
        const line = element('p', 'legend');
        for (const [style, text] of entry) {
            const node = element('span');
            node.append(element('i', style), text);
            line.append(node);
        }
        return line;
    };

    const unwrap = display => display.startsWith('⟨') && display.endsWith('⟩') ? display.slice(1, -1) : display;

    const catalog = definition => new Map((definition ?? []).map(value => [value.label, unwrap(value.display)]));

    const key = place => {
        const [kind, value] = Object.entries(place)[0];
        return `${kind}:${value[0]}:${value[1]}`;
    };

    const touch = event => new Map([
        ...event.footprint.map(place => [key(place), 'projected']),
        ...event.exact.map(place => [key(place), 'exact']),
    ]);

    const token = (occurrence, rule, consumed, place) => {
        const code = rule ?? (occurrence.display?.startsWith('⟨') ? unwrap(occurrence.display) : undefined);
        const node = element('span', code === undefined ? 'token' : 'token rule');
        if (code === undefined) node.textContent = occurrence.label;
        else node.append(book.syntax.fragment(code));
        const mark = consumed?.get(key(place));
        if (mark) node.dataset.touch = mark;
        return node;
    };

    const brief = rule => {
        const code = element('code');
        const [head, ...rest] = rule.split('\n');
        const text = !rest.length ? rule : head.trimEnd().endsWith('(') ? `${head.trimEnd()}…)` : `${head.trimEnd()} …`;
        code.append(book.syntax.fragment(text));
        if (rest.length) code.title = rule;
        return code;
    };

    const deduction = (event, data) => {
        const chain = event.deduction.map(index => data.event[index]);
        const line = element('span', 'deduction');
        line.append('matches ', element('b', undefined, `s${chain.at(-1).target}`), ' after ');
        chain.slice(0, 4).forEach((value, index) => {
            if (index) line.append(', ');
            line.append(brief(value.rule));
        });
        if (chain.length > 4) line.append(` and ${chain.length - 4} more`);
        return line;
    };

    const signature = (frame, definition) => frame.particle.map(value => definition.get(value.label) ?? value.label).sort().join('\n');

    const state = (node, option = {}) => {
        const definition = option.definition ?? new Map();
        const world = new Map();
        node.world.forEach((value, index) => {
            if (!world.has(value.frame)) world.set(value.frame, []);
            world.get(value.frame).push([index, value]);
        });
        const child = new Map();
        node.frame.forEach((value, index) => {
            if (value.parent === null || value.parent === undefined) return;
            if (!child.has(value.parent)) child.set(value.parent, []);
            child.get(value.parent).push(index);
        });
        const occupied = new Map();
        const filled = index => {
            if (occupied.has(index)) return occupied.get(index);
            occupied.set(index, false);
            const result = world.has(index) || (child.get(index) ?? []).some(filled);
            occupied.set(index, result);
            return result;
        };
        const touched = index => [...(option.touch?.keys() ?? [])].some(value => value.startsWith(`context:${index}:`));
        const environment = (index, label) => {
            const frame = node.frame[index];
            const box = element('div', 'environment');
            if (label) box.append(element('span', 'label', label));
            frame.particle.forEach(value => box.append(token(value, definition.get(value.label) ?? value.label, option.touch, { context: [index, value.id] })));
            return box;
        };
        const draw = index => {
            const result = element('div', 'world');
            for (const [position, value] of world.get(index) ?? []) {
                const coherence = element('div', value.particle.length ? 'coherence' : 'coherence empty');
                value.particle.forEach(occurrence => {
                    const item = token(occurrence, undefined, option.touch, { world: [position, occurrence.id] });
                    if (option.match?.get(position)?.has(occurrence.id)) item.dataset.match = '';
                    coherence.append(item);
                });
                result.append(coherence);
            }
            for (const inner of child.get(index) ?? []) {
                if (!filled(inner)) continue;
                const frame = node.frame[inner];
                const scope = element('div', 'scope');
                const held = frame.held?.length ? ` · holds ${frame.held.map(value => value.label).join('.')}` : '';
                const text = frame.particle.map(value => definition.get(value.label) ?? value.label);
                const brief = text.join(' ').length <= 48 || touched(inner);
                const count = frame.particle.length && !brief ? ` · ${frame.particle.length} rules` : '';
                const label = element('span', 'label', `scope${held}${count}`);
                if (count) label.title = text.join('\n');
                scope.append(label, draw(inner));
                if (frame.particle.length && brief) scope.append(environment(inner, 'rules'));
                result.append(scope);
            }
            return result;
        };
        const body = element('div', 'world');
        body.append(...draw(0).childNodes);
        const root = node.frame[0];
        if (root && (touched(0) || (option.root !== undefined && signature(root, definition) !== option.root))) {
            if (root.particle.length) body.append(environment(0, 'rules'));
            else body.append(element('span', 'label', 'no rules'));
        }
        return body;
    };

    book.render = { element, vector, message, tally, legend, unwrap, catalog, touch, token, brief, deduction, signature, state };
})();
