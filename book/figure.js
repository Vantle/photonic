(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { vector } = book.render;

    const pattern = {
        line: /\(\[Line\] ([A-Z])\.([A-Z])\)/g,
        angle: /\(\[Angle\.([A-Z])\] ([A-Z])\.([A-Z])\)/g,
        area: /\(\[Area\] ([A-Z](?:\.[A-Z])+)\)/g,
        square: /\(\[Square\] ([A-Z])\.([A-Z])\)/g,
        point: /\(\[Point\] ([A-Z])\.([A-Z])\)/g,
        parallelogram: /^Parallelogram\.\(\[Diameter\] ([A-Z])\.([A-Z])\)\.\(\[Diameter\] ([A-Z])\.([A-Z])\)$/,
        radius: /^Equal\.\(\[Line\] ([A-Z])\.([A-Z])\)\.\(\[Line\] ([A-Z])\.([A-Z])\)$/,
    };

    const token = text => /^[A-Z]$/.test(text);

    const known = (letter, place) => letter.every(name => place[name]);

    const centre = corner => corner.reduce((sum, value) => [sum[0] + value[0] / corner.length, sum[1] + value[1] / corner.length], [0, 0]);

    const distance = (first, second) => Math.hypot(second[0] - first[0], second[1] - first[1]);

    const direction = (from, to) => {
        const length = distance(from, to) || 1;
        return [(to[0] - from[0]) / length, (to[1] - from[1]) / length];
    };

    const ring = (letter, place) => {
        const middle = centre(letter.map(name => place[name]));
        const turn = name => Math.atan2(place[name][1] - middle[1], place[name][0] - middle[0]);
        const order = [...letter].sort((first, second) => turn(first) - turn(second));
        const start = order.indexOf([...letter].sort()[0]);
        return [...order.slice(start), ...order.slice(0, start)];
    };

    // Opposite sides of a ring of four points are parallel exactly when it is a parallelogram.
    const parallelogram = corner => {
        const side = index => [corner[(index + 1) % 4][0] - corner[index][0], corner[(index + 1) % 4][1] - corner[index][1]];
        const parallel = (first, second) => Math.abs(first[0] * second[1] - first[1] * second[0]) < 1e-3 * Math.hypot(...first) * Math.hypot(...second);
        return corner.length === 4 && parallel(side(0), side(2)) && parallel(side(1), side(3));
    };

    // A square names only its side. The drawn square is the one the figure's points complete; failing
    // that, the one away from the opposite corner of the triangle whose squares a fact compares, or else
    // the one away from the rest of the figure.
    const square = (first, second, place, away) => {
        const [start, end] = [place[first], place[second]];
        const normal = [start[1] - end[1], end[0] - start[0]];
        const side = sign => [start, end, [end[0] + sign * normal[0], end[1] + sign * normal[1]], [start[0] + sign * normal[0], start[1] + sign * normal[1]]];
        const tolerance = Math.hypot(...normal) * 0.01;
        const named = sign => side(sign).slice(2).every(corner => Object.values(place).some(value => distance(value, corner) < tolerance));
        if (named(1)) return side(1);
        if (named(-1)) return side(-1);
        return side(distance(centre(side(1)), away) >= distance(centre(side(-1)), away) ? 1 : -1);
    };

    const empty = () => ({ segment: new Map(), arc: new Map(), region: new Map(), dot: new Set() });

    const shape = (fact, place, middle, fixed) => {
        const result = empty();
        const segment = (first, second) => {
            if (!known([first, second], place) || first === second) return;
            result.segment.set([first, second].sort().join(''), [place[first], place[second]]);
            result.dot.add(first).add(second);
        };
        const region = (key, corner) => {
            result.region.set(key, corner);
            corner.forEach((value, index) => result.segment.set(`${key}:${index}`, [value, corner[(index + 1) % corner.length]]));
        };
        for (const [, first, second] of fact.matchAll(pattern.line)) segment(first, second);
        for (const [, vertex, first, second] of fact.matchAll(pattern.angle)) {
            if (!known([vertex, first, second], place)) continue;
            segment(vertex, first);
            segment(vertex, second);
            result.arc.set(`${vertex}${first}${second}`, [vertex, first, second]);
        }
        for (const [, list] of fact.matchAll(pattern.area)) {
            const letter = list.split('.');
            if (!known(letter, place)) continue;
            letter.forEach(name => result.dot.add(name));
            region(letter.join(''), ring(letter, place).map(name => place[name]));
        }
        const side = [...fact.matchAll(pattern.square)].map(([, first, second]) => [first, second]).filter(pair => known(pair, place));
        const triangle = new Set(side.flat());
        for (const [first, second] of side) {
            const opposite = side.length === 3 && triangle.size === 3 ? [...triangle].find(name => name !== first && name !== second) : undefined;
            result.dot.add(first).add(second);
            const key = `square${first}${second}`;
            region(key, fixed?.get(key) ?? square(first, second, place, opposite ? place[opposite] : middle));
        }
        for (const [, first, second] of fact.matchAll(pattern.point)) {
            [first, second].filter(name => place[name]).forEach(name => result.dot.add(name));
        }
        const [, one, three, two, four] = pattern.parallelogram.exec(fact) ?? [];
        if (one && known([one, two, three, four], place)) {
            [one, two, three, four].forEach(name => result.dot.add(name));
            region([one, two, three, four].sort().join(''), [one, two, three, four].map(name => place[name]));
        }
        if (token(fact) && place[fact]) result.dot.add(fact);
        return result;
    };

    const merge = (list, place, middle, fixed) => {
        const result = empty();
        for (const fact of list) {
            const part = shape(fact, place, middle, fixed);
            part.segment.forEach((value, key) => result.segment.set(key, value));
            part.arc.forEach((value, key) => result.arc.set(key, value));
            part.region.forEach((value, key) => result.region.set(key, value));
            part.dot.forEach(value => result.dot.add(value));
        }
        return result;
    };

    // One frame holds every arrangement and every square, so the figure stays still while a proof moves
    // between cases. A square keeps the side it first takes, though a straight line can be a side of two
    // triangles on opposite sides of it.
    const frame = proof => {
        const middle = centre(Object.values(proof.point));
        const every = [...proof.given, ...proof.step.flatMap(value => [...value.input, ...value.output])];
        const fixed = new Map();
        for (const fact of every) {
            shape(fact, proof.point, middle, fixed).region.forEach((corner, key) => {
                if (key.startsWith('square') && !fixed.has(key)) fixed.set(key, corner);
            });
        }
        const whole = merge(every, proof.point, middle, fixed);
        const all = [proof.point, ...Object.values(proof.arrangement ?? {})].flatMap(Object.values).concat([...whole.region.values()].flat());
        const horizontal = all.map(value => value[0]);
        const vertical = all.map(value => value[1]);
        const [left, right, bottom, top] = [Math.min(...horizontal), Math.max(...horizontal), Math.min(...vertical), Math.max(...vertical)];
        const unit = Math.max(right - left, top - bottom, 1) / 30;
        const margin = unit * 2.6;
        const join = [...whole.segment.keys()].filter(key => key.length === 2);
        const bearing = new Map(Object.entries(proof.point).map(([name, point]) => {
            const sum = join.filter(key => key.includes(name)).reduce((total, key) => {
                const way = direction(point, proof.point[key.replace(name, '')]);
                return [total[0] + way[0], total[1] + way[1]];
            }, [0, 0]);
            const away = Math.hypot(...sum) > 0.3 ? [-sum[0], -sum[1]] : [point[0] - middle[0], point[1] - middle[1]];
            const length = Math.hypot(...away);
            return [name, length > 1e-6 ? [away[0] / length, away[1] / length] : [0, 1]];
        }));
        return { unit, middle, fixed, bearing, box: [left - margin, -top - margin, right - left + 2 * margin, top - bottom + 2 * margin].join(' ') };
    };

    // A case's facts leave the figure when the case closes, so the figure shows what holds where the
    // step runs: the given facts, what earlier steps gave there, and the step itself.
    const scene = (proof, index) => {
        const step = proof.step[index];
        const fact = [...proof.given];
        proof.step.slice(0, index).forEach(value => {
            if (value.world === undefined || value.world === step.world) fact.push(...value.output.filter(text => !token(text)));
        });
        return { step, fact, output: step.output.filter(text => !token(text)), place: { ...proof.point, ...proof.arrangement?.[step.world] } };
    };

    // Points that coincide, such as F and the image G that falls on it, share one dot and one label.
    const cluster = (name, place) => {
        const group = new Map();
        for (const value of name) {
            const key = place[value].map(coordinate => coordinate.toFixed(6)).join(',');
            if (!group.has(key)) group.set(key, []);
            group.get(key).push(value);
        }
        return [...group.values()].map(member => member.sort());
    };

    const paint = (plane, proof, index) => {
        const { step, fact, output, place } = scene(proof, index);
        const { unit, middle, fixed, bearing, box } = proof.frame;
        const base = merge([...fact, ...step.input, ...output], place, middle, fixed);
        const read = merge(step.input, place, middle, fixed);
        const made = merge(output, place, middle, fixed);
        const flip = value => `${value[0]},${-value[1]}`;
        const [fill, ground, mark, dot] = ['fill', 'ground', 'mark', 'dot'].map(name => vector('g', { class: name }));
        const polygon = (corner, style) => fill.append(vector('polygon', { points: corner.map(flip).join(' '), class: style }));
        const segment = (value, style, host) => host.append(vector('line', { x1: value[0][0], y1: -value[0][1], x2: value[1][0], y2: -value[1][1], class: style }));
        read.region.forEach((corner, key) => { if (!made.region.has(key)) polygon(corner, 'read'); });
        made.region.forEach(corner => polygon(corner, 'made'));
        base.segment.forEach((value, key) => { if (!read.segment.has(key) && !made.segment.has(key)) segment(value, 'base', ground); });
        read.segment.forEach((value, key) => { if (!made.segment.has(key)) segment(value, 'read', mark); });
        made.segment.forEach(value => segment(value, 'made', mark));
        const stack = new Map();
        const arc = ([vertex, first, second], style) => {
            const level = stack.get(vertex) ?? 0;
            stack.set(vertex, level + 1);
            const radius = unit * (1.4 + level * 0.55);
            const at = place[vertex];
            const [outward, inward] = [direction(at, place[first]), direction(at, place[second])];
            const start = [at[0] + radius * outward[0], at[1] + radius * outward[1]];
            const end = [at[0] + radius * inward[0], at[1] + radius * inward[1]];
            const sweep = outward[0] * inward[1] - outward[1] * inward[0] > 0 ? 1 : 0;
            mark.append(vector('path', { d: `M ${flip(start)} A ${radius} ${radius} 0 0 ${sweep} ${flip(end)}`, class: style }));
        };
        read.arc.forEach((value, key) => { if (!made.arc.has(key)) arc(value, 'read'); });
        made.arc.forEach(value => arc(value, 'made'));
        if (step.why === 'circle' || step.why === 'produce') {
            output.forEach(text => {
                const [, first, second, third, fourth] = pattern.radius.exec(text) ?? [];
                const pivot = [first, second].find(name => name === third || name === fourth);
                const rim = pivot === first ? second : first;
                if (!pivot || !known([pivot, rim], place)) return;
                ground.append(vector('circle', { cx: place[pivot][0], cy: -place[pivot][1], r: distance(place[pivot], place[rim]), class: 'orbit' }));
            });
        }
        const fresh = new Set(step.input.filter(token));
        for (const member of cluster(base.dot, place)) {
            const at = place[member[0]];
            const bright = member.some(name => fresh.has(name));
            const away = bearing.get(member[0]) ?? [0, 1];
            const circle = vector('circle', { cx: at[0], cy: -at[1], r: unit * (bright ? 0.32 : 0.2) });
            if (bright) circle.classList.add('fresh');
            const label = vector('text', { x: at[0] + away[0] * unit * (member.length > 1 ? 2.2 : 1.25), y: -(at[1] + away[1] * unit * 1.25) + unit * 0.42, 'font-size': unit * 1.2 });
            label.textContent = member.join(' = ');
            dot.append(circle, label);
        }
        plane.setAttribute('viewBox', box);
        plane.replaceChildren(fill, ground, mark, dot);
    };

    book.figure = { token, ring, parallelogram, frame, paint };
})();
