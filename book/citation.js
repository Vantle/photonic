(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { vector } = book.render;
    const gap = 16;
    const base = 104;

    const draw = (node, pick) => {
        const plane = vector('svg', { viewBox: `0 0 ${gap * node.length} ${base + 18}`, class: 'citation', role: 'img', 'aria-label': 'Each arc joins a proposition to an earlier one it cites' });
        const position = new Map(node.map((value, index) => [value.name, gap * index + gap / 2]));
        const layer = vector('g');
        const arc = node.flatMap(value => [...value.cited].map(cited => {
            const [from, to] = [position.get(value.name), position.get(cited)];
            const radius = Math.abs(from - to) / 2;
            const path = vector('path', { d: `M ${from} ${base} A ${radius} ${Math.min(radius, base - 8)} 0 0 0 ${to} ${base}` });
            layer.append(path);
            return { from: value.name, to: cited, path };
        }));
        plane.append(layer);
        const group = new Map(node.map(value => {
            const item = vector('g', { class: 'node' });
            const hint = vector('title');
            hint.textContent = value.label;
            const label = vector('text', { x: position.get(value.name), y: base + 15 });
            label.textContent = value.mark;
            item.append(hint, vector('circle', { cx: position.get(value.name), cy: base, r: 3.2 }), vector('circle', { cx: position.get(value.name), cy: base, r: gap / 2, class: 'target' }), label);
            item.addEventListener('click', () => pick(value.name));
            plane.append(item);
            return [value.name, item];
        }));
        const cite = new Map(node.map(value => [value.name, value.cited]));
        const show = current => {
            const cited = cite.get(current);
            arc.forEach(({ from, to, path }) => {
                if (from === current) path.dataset.link = 'cite';
                else if (to === current) path.dataset.link = 'cited';
                else delete path.dataset.link;
            });
            layer.append(...arc.filter(({ path }) => path.dataset.link === 'cited').map(({ path }) => path));
            layer.append(...arc.filter(({ path }) => path.dataset.link === 'cite').map(({ path }) => path));
            node.forEach((value, index) => {
                const item = group.get(value.name);
                const citing = value.cited.has(current);
                item.dataset.role = value.name === current ? 'current' : cited.has(value.name) ? 'cite' : citing ? 'cited' : '';
                const shown = index === 0 || index === node.length - 1 || (index + 1) % 5 === 0 || !/^\d+$/.test(value.mark) || item.dataset.role === 'current' || cited.has(value.name);
                item.querySelector('text').style.display = shown ? '' : 'none';
            });
        };
        return { element: plane, show };
    };

    book.citation = { draw };
})();
