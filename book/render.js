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
        let timer;
        const say = (text = '', tone) => {
            clearTimeout(timer);
            node.textContent = text;
            if (tone) node.dataset.tone = tone;
            else delete node.dataset.tone;
        };
        const wait = text => {
            clearTimeout(timer);
            timer = setTimeout(() => say(text), 400);
        };
        return { element: node, say, wait };
    };

    const series = word => word.length < 3 ? word.join(' and ') : `${word.slice(0, -1).join(', ')} and ${word.at(-1)}`;

    const noun = (number, word) => number === 1 ? word : `${word}s`;

    const count = (number, word) => `${number.toLocaleString()} ${noun(number, word)}`;

    const tally = (number, word) => {
        const node = element('span');
        node.append(element('b', undefined, number.toLocaleString()), ` ${noun(number, word)}`);
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

    const preset = (name, pick) => {
        const host = element('div', 'preset');
        name.forEach((label, index) => {
            const button = element('button', undefined, label);
            button.type = 'button';
            button.addEventListener('click', () => pick(index));
            host.append(button);
        });
        const press = chosen => host.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === chosen)));
        return { element: host, press };
    };

    const key = place => {
        const [kind, value] = Object.entries(place)[0];
        return `${kind}:${value[0]}:${value[1]}`;
    };

    const touch = event => new Map([
        ...event.footprint.map(place => [key(place), 'projected']),
        ...event.exact.map(place => [key(place), 'exact']),
    ]);

    const token = (kind, content, consumed, place) => {
        const node = element('span', kind === 'rule' ? 'token rule' : 'token');
        if (kind === 'rule') node.append(book.syntax.fragment(content));
        else node.textContent = content;
        const contact = consumed?.get(key(place));
        if (contact) node.dataset.touch = contact;
        return node;
    };

    const coherence = (world, option = {}) => {
        const node = element('div', world.particle.length ? 'coherence' : 'coherence empty');
        world.particle.forEach(occurrence => {
            const content = occurrence.kind === 'rule' ? option.definition[occurrence.rule] : occurrence.label;
            const item = token(occurrence.kind, content, option.touch, { world: [option.index, occurrence.id] });
            if (option.match?.has(occurrence.id)) item.dataset.match = '';
            node.append(item);
        });
        return node;
    };

    const shorten = rule => {
        const [head, ...rest] = rule.split('\n');
        if (!rest.length) return rule;
        return head.trimEnd().endsWith('(') ? `${head.trimEnd()}…)` : `${head.trimEnd()} …`;
    };

    const brief = rule => {
        const code = element('code');
        code.append(book.syntax.fragment(shorten(rule)));
        if (rule.includes('\n')) code.title = rule;
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

    const signature = (frame, definition) => frame.particle.map(value => definition[value.rule]).sort().join('\n');

    const state = (node, option = {}) => {
        const definition = option.definition ?? [];
        const world = new Map();
        node.world.forEach((value, index) => {
            if (!world.has(value.frame)) world.set(value.frame, []);
            world.get(value.frame).push([index, value]);
        });
        const child = new Map();
        node.frame.forEach((value, index) => {
            if (value.parent === null) return;
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
            const box = element('div', 'environment');
            if (label) box.append(element('span', 'label', label));
            node.frame[index].particle.forEach(value => box.append(token('rule', definition[value.rule], option.touch, { context: [index, value.id] })));
            return box;
        };
        const holding = index => {
            const box = element('div', 'held');
            box.append(element('span', 'label', 'holds'));
            node.frame[index].held.forEach(value => {
                const place = { held: [index, value.id] };
                if (value.kind === 'atom') {
                    box.append(token('atom', value.label, option.touch, place));
                    return;
                }
                const rule = definition[value.rule];
                const item = token('rule', shorten(rule), option.touch, place);
                if (rule.includes('\n')) item.title = rule;
                box.append(item);
            });
            return box;
        };
        const draw = index => {
            const result = element('div', 'world');
            for (const [position, value] of world.get(index) ?? []) {
                result.append(coherence(value, { index: position, definition, touch: option.touch, match: option.match?.get(position) }));
            }
            for (const inner of child.get(index) ?? []) {
                if (!filled(inner)) continue;
                const frame = node.frame[inner];
                const scope = element('div', 'scope');
                const rule = frame.particle.map(value => definition[value.rule]);
                const inline = rule.join(' ').length <= 48 || touched(inner);
                const summary = frame.particle.length && !inline ? ` · ${count(frame.particle.length, 'rule')}` : '';
                const label = element('span', 'label', `scope${summary}`);
                if (summary) label.title = rule.join('\n');
                scope.append(label);
                if (frame.held.length) scope.append(holding(inner));
                scope.append(draw(inner));
                if (frame.particle.length && inline) scope.append(environment(inner, 'rules'));
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

    book.render = { element, vector, message, series, count, tally, legend, preset, touch, token, coherence, brief, deduction, signature, state };
})();
