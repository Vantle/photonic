(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, vector, tally } = book.render;
    let serial = 0;

    const model = execution => {
        const outgoing = new Map(execution.state.map(value => [value.id, []]));
        execution.event.forEach(value => outgoing.get(value.source)?.push(value));
        const search = allow => {
            const previous = new Map([[0, undefined]]);
            const queue = [0];
            for (let index = 0; index < queue.length; index++) {
                for (const value of outgoing.get(queue[index]) ?? []) {
                    if (!allow(value) || previous.has(value.target)) continue;
                    previous.set(value.target, value);
                    queue.push(value.target);
                }
            }
            return previous;
        };
        const tree = [search(value => !value.deduction.length), search(() => true)];
        return { ...execution, definition: book.render.catalog(execution.definition), outgoing, tree };
    };

    const route = (data, target) => {
        const previous = data.tree.find(value => value.has(target));
        const path = [];
        for (let value = previous?.get(target); value; value = previous.get(value.source)) path.unshift(value);
        return path;
    };

    const layer = (node, event) => {
        const member = new Set(node);
        const outgoing = new Map(node.map(state => [state, []]));
        const parent = new Map(node.map(state => [state, []]));
        event.forEach(value => {
            if (!member.has(value.source) || !member.has(value.target) || value.source === value.target) return;
            outgoing.get(value.source).push(value.target);
            parent.get(value.target).push(value.source);
        });
        const mark = new Map();
        const back = new Set();
        const order = [];
        const visit = root => {
            const stack = [[root, 0]];
            mark.set(root, 1);
            while (stack.length) {
                const top = stack.at(-1);
                const [state, index] = top;
                if (index < outgoing.get(state).length) {
                    top[1]++;
                    const next = outgoing.get(state)[index];
                    if (mark.get(next) === 1) back.add(`${state}:${next}`);
                    if (mark.has(next)) continue;
                    mark.set(next, 1);
                    stack.push([next, 0]);
                    continue;
                }
                mark.set(state, 2);
                order.push(state);
                stack.pop();
            }
        };
        node.filter(state => !parent.get(state).length).forEach(state => { if (!mark.has(state)) visit(state); });
        node.forEach(state => { if (!mark.has(state)) visit(state); });
        const depth = new Map(node.map(state => [state, 0]));
        for (const state of order.reverse()) {
            for (const next of outgoing.get(state)) {
                if (back.has(`${state}:${next}`)) continue;
                depth.set(next, Math.max(depth.get(next), depth.get(state) + 1));
            }
        }
        const column = [];
        node.forEach(state => (column[depth.get(state)] ??= []).push(state));
        const position = new Map();
        column.forEach(value => value?.forEach((state, index) => position.set(state, index)));
        for (let pass = 0; pass < 3; pass++) {
            column.forEach((value, index) => {
                if (!index || !value) return;
                const center = new Map(value.map(state => {
                    const above = parent.get(state).filter(source => depth.get(source) < index).map(source => position.get(source));
                    return [state, above.length ? above.reduce((sum, item) => sum + item, 0) / above.length : position.get(state)];
                }));
                value.sort((left, right) => center.get(left) - center.get(right) || left - right);
                value.forEach((state, order) => position.set(state, order));
            });
        }
        return column.filter(Boolean);
    };

    const draw = (host, data, option = {}) => {
        const filter = option.filter;
        const node = data.state.map(value => value.id).filter(state => !filter || filter.state.has(state));
        const event = data.event.filter(value => !filter || filter.event.has(value.id));
        const initial = book.render.signature(data.state[0].frame[0] ?? { particle: [] }, data.definition);
        const witness = new Map();
        (option.verdict ?? []).forEach((value, index) => {
            if (value.witness === null || value.witness === undefined) return;
            witness.set(value.witness, [...(witness.get(value.witness) ?? []), index]);
        });
        const identifier = `graph${serial++}`;
        const scroll = element('div', 'graph');
        scroll.tabIndex = 0;
        scroll.setAttribute('role', 'group');
        scroll.setAttribute('aria-label', `Execution graph with ${node.length} configurations and ${event.length} events`);
        const sizer = element('div');
        const canvas = element('div', 'canvas');
        const drawing = vector('svg', { 'aria-hidden': 'true' });
        const definition = vector('defs');
        for (const kind of ['plain', 'active']) {
            const marker = vector('marker', { id: `${identifier}${kind}`, viewBox: '0 0 10 10', refX: '9', refY: '5', markerWidth: '7', markerHeight: '7', orient: 'auto-start-reverse' });
            marker.append(vector('path', { d: 'M0 0L10 5L0 10z', class: kind === 'plain' ? 'head' : `head ${kind}` }));
            definition.append(marker);
        }
        drawing.append(definition);
        canvas.append(drawing);
        sizer.append(canvas);
        scroll.append(sizer);
        const inspector = element('div', 'inspector');
        host.replaceChildren(scroll, inspector);

        const card = new Map();
        const fill = (state, touch) => {
            const name = element('span', 'name', `s${state}`);
            if (state === 0) name.append(element('span', 'badge', 'start'));
            if (data.closed && !data.outgoing.get(state).length) name.append(element('span', 'badge', 'end'));
            if (witness.has(state)) name.append(element('span', 'badge', witness.get(state).length > 1 ? 'targets' : 'target'));
            const box = card.get(state);
            box.replaceChildren(name, book.render.state(data.state[state], { definition: data.definition, touch, root: initial, match: filter?.match.get(state) }));
            if (touch) box.dataset.focus = '';
            else delete box.dataset.focus;
        };
        node.forEach(state => {
            const box = element('div', 'state');
            box.tabIndex = 0;
            box.dataset.state = state;
            box.setAttribute('role', 'button');
            box.setAttribute('aria-pressed', 'false');
            if (witness.has(state)) box.dataset.witness = '';
            if (filter?.match.has(state)) box.dataset.match = '';
            card.set(state, box);
            fill(state);
            box.addEventListener('click', () => select(state, true));
            box.addEventListener('keydown', value => {
                if (value.key !== 'Enter' && value.key !== ' ') return;
                value.preventDefault();
                select(state, true);
            });
            canvas.append(box);
        });

        const column = layer(node, event);
        const gap = { x: 64, y: 22, pad: 24 };
        const half = gap.x / 2;
        const size = new Map(node.map(state => [state, { width: card.get(state).offsetWidth, height: card.get(state).offsetHeight }]));
        const width = column.map(value => Math.max(...value.map(state => size.get(state).width)));
        const height = column.map(value => value.reduce((sum, state) => sum + size.get(state).height, 0) + gap.y * (value.length - 1));
        const tallest = Math.max(...height, 0);
        const left = [];
        const place = new Map();
        let x = gap.pad;
        column.forEach((value, index) => {
            left[index] = x;
            let y = gap.pad + (tallest - height[index]) / 2;
            value.forEach(state => {
                place.set(state, { x: x + (width[index] - size.get(state).width) / 2, y, width: size.get(state).width, height: size.get(state).height, column: index });
                y += size.get(state).height + gap.y;
            });
            x += width[index] + gap.x;
        });
        const right = index => left[index] + width[index];
        const total = { width: Math.max(x - gap.x + gap.pad, 2 * gap.pad), height: tallest + gap.pad * 2 };

        const parallel = new Map();
        event.forEach(value => {
            const pair = `${value.source}:${value.target}`;
            parallel.set(pair, [...(parallel.get(pair) ?? []), value.id]);
        });
        const stack = column.map(value => value.map(state => place.get(state)));
        const locate = x => {
            let low = 0;
            let high = left.length - 1;
            while (low < high) {
                const middle = Math.ceil((low + high) / 2);
                if (left[middle] <= x) low = middle;
                else high = middle - 1;
            }
            return low;
        };
        const clear = (control, from, to) => Array.from({ length: 33 }, (_, step) => {
            const t = step / 32;
            const s = 1 - t;
            return [0, 1].map(axis => s * s * s * control[0][axis] + 3 * s * s * t * control[1][axis] + 3 * s * t * t * control[2][axis] + t * t * t * control[3][axis]);
        }).every(([x, y]) => {
            const index = locate(x);
            if (index <= from || index >= to) return true;
            return stack[index].every(box => x < box.x - 8 || x > box.x + box.width + 8 || y < box.y - 8 || y > box.y + box.height + 8);
        });
        const forward = (from, to, exit, offset) => {
            const entry = [to.x - 2, to.y + to.height / 2 + offset];
            const outlet = right(from.column);
            const inlet = Math.min(left[to.column], entry[0]);
            const bend = Math.max(28, (inlet - outlet) / 2);
            const direct = [[outlet, exit[1]], [outlet + bend, exit[1]], [inlet - bend, entry[1]], [inlet, entry[1]]];
            const path = [];
            if (!clear(direct, from.column, to.column)) {
                const obstacle = stack.slice(from.column + 1, to.column).flat();
                const above = Math.min(...obstacle.map(box => box.y)) - 18 - Math.abs(offset);
                const below = Math.max(...obstacle.map(box => box.y + box.height)) + 18 + Math.abs(offset);
                const middle = (exit[1] + entry[1]) / 2;
                const level = middle - above <= below - middle ? above : below;
                const rise = left[from.column + 1];
                const fall = right(to.column - 1);
                path.push(['C', [outlet + half, exit[1]], [rise - half, level], [rise, level]]);
                if (fall > rise) path.push(['L', [fall, level]]);
                path.push(['C', [fall + half, level], [inlet - half, entry[1]], [inlet, entry[1]]]);
            } else {
                path.push(['C', ...direct.slice(1)]);
            }
            if (entry[0] > inlet) path.push(['L', entry]);
            return path;
        };
        const backward = (from, to, exit, offset) => {
            const entry = [to.x + to.width + 2, to.y + to.height / 2 + offset];
            const outlet = right(from.column);
            const inlet = Math.max(right(to.column), entry[0]);
            const reach = half + Math.abs(offset);
            const path = [];
            if (from.column === to.column) {
                path.push(['C', [outlet + reach, exit[1]], [inlet + reach, entry[1]], [inlet, entry[1]]]);
            } else {
                const obstacle = stack.slice(to.column + 1, from.column + 1).flat();
                const floor = Math.max(...obstacle.map(box => box.y + box.height), entry[1]) + 22 + Math.abs(offset);
                path.push(['C', [outlet + reach, exit[1]], [outlet + reach, floor], [outlet, floor]]);
                path.push(['L', [inlet + half, floor]]);
                path.push(['C', [inlet + half - 12, floor], [inlet + reach, entry[1]], [inlet, entry[1]]]);
            }
            if (entry[0] < inlet) path.push(['L', entry]);
            return path;
        };
        const curve = value => {
            const from = place.get(value.source);
            const to = place.get(value.target);
            if (!from || !to) return undefined;
            const sibling = parallel.get(`${value.source}:${value.target}`);
            const offset = (sibling.indexOf(value.id) - (sibling.length - 1) / 2) * 9;
            if (value.source === value.target) {
                const start = from.x + from.width - 30 + offset;
                return [['M', [start, from.y]], ['C', [start - 4, from.y - 16], [start + 22, from.y - 16], [start + 16, from.y - 1]]];
            }
            const exit = [from.x + from.width, from.y + from.height / 2 + offset];
            const path = [['M', exit]];
            if (right(from.column) > exit[0]) path.push(['L', [right(from.column), exit[1]]]);
            return [...path, ...(to.column > from.column ? forward : backward)(from, to, exit, offset)];
        };
        const geometry = new Map(event.map(value => [value.id, curve(value)]));
        let low = gap.pad;
        let high = total.height - gap.pad;
        geometry.forEach(path => path?.forEach(([, ...point]) => point.forEach(([, y]) => {
            low = Math.min(low, y);
            high = Math.max(high, y);
        })));
        const shift = gap.pad - low;
        place.forEach(box => { box.y += shift; });
        card.forEach((box, state) => {
            box.style.left = `${place.get(state).x}px`;
            box.style.top = `${place.get(state).y}px`;
        });
        total.height = high + shift + gap.pad;
        canvas.style.width = `${total.width}px`;
        canvas.style.height = `${total.height}px`;
        drawing.setAttribute('width', total.width);
        drawing.setAttribute('height', total.height);
        const line = new Map();
        event.forEach(value => {
            const path = geometry.get(value.id);
            if (!path) return;
            const stroke = vector('path', {
                d: path.map(([command, ...point]) => command + point.map(([left, top]) => `${left} ${top + shift}`).join(' ')).join(''),
                class: value.deduction.length ? 'link inferred' : 'link',
                'marker-end': `url(#${identifier}plain)`,
            });
            stroke.dataset.event = value.id;
            drawing.append(stroke);
            line.set(value.id, stroke);
        });

        let scale = 1;
        const zoom = value => {
            scale = Math.min(2, Math.max(0.25, value === 'fit' ? Math.min(1, (scroll.clientWidth - 8) / total.width) : scale * value));
            canvas.style.transform = scale === 1 ? '' : `scale(${scale})`;
            sizer.style.width = `${total.width * scale}px`;
            sizer.style.height = `${total.height * scale}px`;
            sizer.style.margin = '0 auto';
        };
        zoom(1);

        const activate = (number, on) => {
            const stroke = line.get(number);
            if (!stroke) return;
            stroke.classList.toggle('active', on);
            stroke.setAttribute('marker-end', `url(#${identifier}${on ? 'active' : 'plain'})`);
            if (on) drawing.append(stroke);
        };

        let hovered;
        let pinned;
        const paint = () => {
            line.forEach(stroke => stroke.classList.remove('deduction'));
            card.forEach(box => delete box.dataset.deduction);
            for (const value of [pinned, hovered]) {
                if (!value?.deduction.length) continue;
                value.deduction.forEach(number => {
                    const stroke = line.get(number);
                    if (!stroke) return;
                    stroke.classList.add('deduction');
                    drawing.append(stroke);
                });
                const box = card.get(data.event[value.deduction.at(-1)].target);
                if (box) box.dataset.deduction = '';
            }
        };
        const explain = value => {
            pinned = value;
            paint();
        };

        const reveal = state => {
            const box = place.get(state);
            if (!box) return;
            const across = (box.x + box.width / 2) * scale - scroll.clientWidth / 2;
            const down = (box.y + box.height / 2) * scale - scroll.clientHeight / 2;
            scroll.scrollTo({ left: Math.max(0, across), top: Math.max(0, down) });
        };

        const summary = element('p', 'summary');
        summary.append(tally(data.state.length, 'configuration'), tally(data.event.length, 'event'), tally(data.work ?? 0, 'work step'));
        summary.append(element('span', 'badge', data.closed ? 'explored completely' : 'budget reached'));
        if (filter) summary.append(element('span', undefined, `${node.length} shown`));
        const verdict = element('div', 'verdict');
        (option.verdict ?? []).forEach((value, index) => {
            const row = element('div', 'claim');
            row.append(element('span', `badge ${value.outcome}`, value.outcome));
            const code = element('code');
            code.append(book.syntax.fragment(option.target?.[index] ?? ''));
            row.append(code);
            if (value.witness !== null && value.witness !== undefined && card.has(value.witness)) {
                const jump = element('button', 'tool', `witness s${value.witness}`);
                jump.type = 'button';
                jump.addEventListener('click', () => select(value.witness, true));
                row.append(jump);
            }
            verdict.append(row);
        });
        const departure = element('div', 'departure');
        inspector.append(summary);
        if (verdict.childNodes.length) inspector.append(verdict);
        const legend = [['', 'rule applied directly'], ['touch', 'consumed on hover']];
        if (event.some(value => value.deduction.length)) legend.splice(1, 0, ['inferred', 'rule inferred at its source'], ['deduction', 'where an inferred rule matched']);
        inspector.append(departure, book.render.legend(legend));

        let current;
        let lit = [];
        const select = (state, focus) => {
            if (!card.has(state)) return;
            if (current !== undefined) card.get(current).setAttribute('aria-pressed', 'false');
            lit.forEach(value => activate(value, false));
            current = state;
            card.get(state).setAttribute('aria-pressed', 'true');
            const path = option.route ? route(data, state) : undefined;
            lit = (path ?? data.outgoing.get(state)).map(value => value.id);
            lit.forEach(value => activate(value, true));
            const leaving = data.outgoing.get(state).filter(value => !filter || filter.event.has(value.id));
            const heading = element('p');
            const count = leaving.length === 1 ? '1 event leaves' : `${leaving.length} events leave`;
            heading.append(element('b', undefined, `s${state}`), leaving.length
                ? ` · ${count} this configuration`
                : data.closed ? ' · no rule applies here' : ' · no events recorded before the budget ran out');
            departure.replaceChildren(heading);
            leaving.forEach(value => {
                const row = element('button', 'event');
                row.type = 'button';
                row.append(element('span', 'kind', value.deduction.length ? 'inferred' : 'direct'));
                const code = element('code');
                code.append(book.syntax.fragment(value.rule));
                row.append(code, element('span', 'arrow', `→ s${value.target}`));
                if (value.deduction.length) row.append(book.render.deduction(value, data));
                const enter = () => {
                    fill(state, book.render.touch(value));
                    hovered = value;
                    paint();
                };
                const leave = () => {
                    fill(state);
                    hovered = undefined;
                    paint();
                };
                row.addEventListener('mouseenter', enter);
                row.addEventListener('focus', enter);
                row.addEventListener('mouseleave', leave);
                row.addEventListener('blur', leave);
                row.addEventListener('click', () => { leave(); select(value.target, true); });
                departure.append(row);
            });
            if (focus) {
                reveal(state);
                card.get(state).focus({ preventScroll: true });
            }
            option.select?.(state, path);
        };
        const first = option.start ?? (card.has(0) ? 0 : node[0]);
        if (first !== undefined) select(first, false);
        else departure.replaceChildren(element('p', undefined, 'No configuration matches the filter.'));
        return { zoom, explain };
    };

    book.graph = { draw, model, route };
})();
