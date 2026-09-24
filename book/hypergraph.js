(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, vector, tally } = book.render;

    const trace = (data, route) => {
        const lifeline = [];
        const hyperedge = [];
        const create = (column, world) => {
            const line = { start: column, end: undefined, world };
            lifeline.push(line);
            return line;
        };
        let current = data.state[0].world.map(world => create(0, world));
        route.forEach((event, step) => {
            const column = step + 1;
            const consumed = new Set(event.world);
            const input = [...consumed].map(index => current[index]).filter(Boolean);
            input.forEach(line => { line.end = column; });
            const output = [];
            current = data.state[event.target].world.map((world, index) => {
                const source = event.context[index] ?? [];
                if (source.length === 1 && !consumed.has(source[0]) && current[source[0]]) return current[source[0]];
                const line = create(column, world);
                output.push(line);
                return line;
            });
            hyperedge.push({ column, event, input, output });
        });
        return { lifeline, hyperedge, count: route.length + 1 };
    };

    const coherence = (world, match) => {
        const node = element('div', world.particle.length ? 'coherence' : 'coherence empty');
        world.particle.forEach(occurrence => {
            const token = book.render.token(occurrence);
            if (match?.has(occurrence.id)) token.dataset.match = '';
            node.append(token);
        });
        return node;
    };

    const draw = (host, data, route, option = {}) => {
        const model = trace(data, route);
        const filter = option.pattern ? book.pattern.lane(option.pattern, model) : undefined;
        const visible = line => !filter || filter.lifeline.has(line);
        const shown = edge => !filter || filter.hyperedge.has(edge);
        const scroll = element('div', 'hypergraph');
        scroll.tabIndex = 0;
        scroll.setAttribute('role', 'group');
        scroll.setAttribute('aria-label', `Execution hypergraph with ${model.hyperedge.length} events`);
        const canvas = element('div', 'canvas');
        const drawing = vector('svg', { 'aria-hidden': 'true' });
        canvas.append(drawing);
        scroll.append(canvas);
        const inspector = element('div', 'inspector');
        host.replaceChildren(scroll, inspector);

        const capsule = new Map();
        model.lifeline.filter(visible).forEach(line => {
            const box = element('div', line.world.frame ? 'capsule scoped' : 'capsule');
            const match = filter?.match.get(line);
            if (match) box.dataset.match = '';
            box.title = line.world.frame ? 'A coherence inside a scope' : 'A coherence';
            box.append(coherence(line.world, match));
            canvas.append(box);
            capsule.set(line, box);
        });
        const size = new Map([...capsule].map(([line, box]) => [line, { width: box.offsetWidth, height: box.offsetHeight }]));
        const row = Math.max(34, ...[...size.values()].map(value => value.height)) + 18;
        const gap = { event: 70, idle: 14, top: 34, pad: 22 };
        const width = Array.from({ length: model.count }, () => 0);
        model.lifeline.filter(visible).forEach(line => {
            width[line.start] = Math.max(width[line.start], size.get(line).width);
        });
        const column = [];
        let x = gap.pad;
        for (let index = 0; index < model.count; index++) {
            if (index) x += shown(model.hyperedge[index - 1]) ? gap.event : gap.idle;
            column[index] = x;
            x += width[index];
        }
        const right = x + gap.pad + 20;
        const lane = new Map();
        const free = [];
        let count = 0;
        const take = () => (free.length ? free.shift() : count++);
        model.lifeline.filter(line => line.start === 0 && visible(line)).forEach(line => lane.set(line, take()));
        model.hyperedge.forEach(edge => {
            free.push(...edge.input.filter(line => lane.has(line)).map(line => lane.get(line)));
            free.sort((left, right) => left - right);
            edge.output.filter(visible).forEach(line => lane.set(line, take()));
        });
        const y = line => gap.top + lane.get(line) * row + row / 2;
        const height = gap.top + Math.max(1, count) * row + gap.pad;
        canvas.style.width = `${right}px`;
        canvas.style.height = `${height}px`;
        drawing.setAttribute('width', right);
        drawing.setAttribute('height', height);

        for (let index = 0; index < model.count; index++) {
            if (!width[index]) continue;
            const label = vector('text', { x: column[index], y: 16, class: 'column' });
            label.textContent = `s${index ? model.hyperedge[index - 1].event.target : 0}`;
            drawing.append(label);
        }
        const hub = new Map();
        model.hyperedge.filter(shown).forEach(edge => {
            const level = [...edge.input, ...edge.output].filter(line => lane.has(line)).map(y);
            hub.set(edge, {
                x: column[edge.column] - gap.event / 2,
                y: level.length ? level.reduce((sum, value) => sum + value, 0) / level.length : gap.top + row / 2,
            });
        });
        const strand = new Map();
        const curve = (from, to, style, edge) => {
            const bend = Math.max(12, Math.abs(to[0] - from[0]) / 2);
            const path = vector('path', { d: `M${from[0]} ${from[1]}C${from[0] + bend} ${from[1]} ${to[0] - bend} ${to[1]} ${to[0]} ${to[1]}`, class: style });
            drawing.append(path);
            strand.set(edge, [...(strand.get(edge) ?? []), path]);
        };
        model.lifeline.filter(line => lane.has(line)).forEach(line => {
            const box = capsule.get(line);
            box.style.left = `${column[line.start]}px`;
            box.style.top = `${y(line) - size.get(line).height / 2}px`;
            const start = column[line.start] + size.get(line).width;
            const consumer = line.end === undefined ? undefined : model.hyperedge[line.end - 1];
            const end = consumer && shown(consumer) ? hub.get(consumer).x - 22 : right - gap.pad;
            if (end > start) drawing.append(vector('path', { d: `M${start} ${y(line)}H${end}`, class: line.world.frame ? 'lane scoped' : 'lane' }));
        });
        model.hyperedge.filter(shown).forEach(edge => {
            const center = hub.get(edge);
            const style = edge.event.direct ? 'strand' : 'strand inferred';
            edge.input.filter(line => lane.has(line)).forEach(line => curve([center.x - 22, y(line)], [center.x - 7, center.y], style, edge));
            edge.output.filter(line => lane.has(line)).forEach(line => curve([center.x + 7, center.y], [column[edge.column], y(line)], style, edge));
        });

        const summary = element('p', 'summary');
        summary.append(tally(model.hyperedge.length, 'event'), tally(model.lifeline.length, 'coherence'));
        if (filter) summary.append(element('span', undefined, `${model.lifeline.filter(visible).length} shown`));
        const focus = element('div', 'departure');
        inspector.append(summary, focus, book.render.legend([['', 'coherence lane'], ['hyperedge', 'event'], ['inferred', 'inferred event']]));

        const button = new Map();
        let current;
        const select = edge => {
            if (current) {
                button.get(current).setAttribute('aria-pressed', 'false');
                strand.get(current)?.forEach(path => path.classList.remove('active'));
            }
            current = edge;
            button.get(edge).setAttribute('aria-pressed', 'true');
            strand.get(edge)?.forEach(path => path.classList.add('active'));
            const heading = element('p');
            heading.append(element('b', undefined, `s${edge.event.source} → s${edge.event.target}`), ` · ${edge.event.direct ? 'direct' : 'inferred'} · consumes ${edge.input.length}, produces ${edge.output.length}`);
            const rule = element('pre', 'code');
            book.syntax.highlight(rule, edge.event.rule);
            const flow = element('div', 'row');
            const side = (text, line) => {
                flow.append(element('span', 'kind', text));
                if (!line.length) flow.append(element('span', 'arrow', 'nothing'));
                line.forEach(value => flow.append(coherence(value.world)));
            };
            side('consumes', edge.input);
            side('produces', edge.output);
            focus.replaceChildren(heading, rule, flow);
        };
        model.hyperedge.filter(shown).forEach(edge => {
            const center = hub.get(edge);
            const glyph = element('button', edge.event.direct ? 'hyperedge' : 'hyperedge inferred');
            glyph.type = 'button';
            glyph.style.left = `${center.x}px`;
            glyph.style.top = `${center.y}px`;
            glyph.title = edge.event.rule;
            glyph.setAttribute('aria-label', `Event ${edge.column}: ${edge.event.rule}`);
            glyph.setAttribute('aria-pressed', 'false');
            glyph.addEventListener('click', () => select(edge));
            canvas.append(glyph);
            button.set(edge, glyph);
        });
        const last = [...button.keys()].at(-1);
        if (last) select(last);
        else focus.replaceChildren(element('p', undefined, model.hyperedge.length ? 'No event matches the filter.' : 'No event has happened yet: this is the start.'));
    };

    book.hypergraph = { draw };
})();
