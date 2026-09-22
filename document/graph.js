(() => {
    'use strict';
    const find = id => document.getElementById(id);
    const svg = (tag, attribute = {}, text) => {
        const node = document.createElementNS('http://www.w3.org/2000/svg', tag);
        for (const [key, value] of Object.entries(attribute)) node.setAttribute(key, value);
        if (text !== undefined) node.textContent = text;
        return node;
    };
    const text = (tag, value) => {
        const node = document.createElement(tag);
        node.textContent = value;
        return node;
    };
    let report = globalThis.evaluation.report;
    let selected = 0;
    let worker;
    let timer;
    const label = state => state.world.map(world => world.particle.map(value => value.display).join('.') || '()').join(', ') || 'No coherences';
    const render = () => {
        const graph = find('evaluation-graph');
        const state = report.state.slice(0, 32);
        const edge = report.event.filter(event => event.source < state.length && event.target < state.length);
        const distance = new Map([[0, 0]]);
        const queue = [0];
        for (let index = 0; index < queue.length; index++) {
            for (const event of edge.filter(event => event.source === queue[index])) {
                if (distance.has(event.target)) continue;
                distance.set(event.target, distance.get(event.source) + 1);
                queue.push(event.target);
            }
        }
        const level = [];
        for (const item of state) {
            const depth = distance.get(item.id) ?? 0;
            (level[depth] ??= []).push(item);
        }
        const width = Math.max(680, level.length * 235);
        const height = Math.max(260, Math.max(...level.map(row => row.length)) * 112 + 64);
        const position = new Map();
        level.forEach((row, column) => row.forEach((item, index) => position.set(item.id, [110 + column * 235, (height - row.length * 112) / 2 + index * 112 + 56])));
        graph.replaceChildren();
        graph.setAttribute('viewBox', `0 0 ${width} ${height}`);
        const marker = svg('marker', { id: 'evaluation-arrow', viewBox: '0 0 10 10', refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: 'auto-start-reverse' });
        marker.append(svg('path', { d: 'M 0 0 L 10 5 L 0 10 z', fill: 'context-stroke' }));
        const defs = svg('defs');
        defs.append(marker);
        graph.append(defs);
        for (const event of edge) {
            const [x, y] = position.get(event.source);
            const [tx, ty] = position.get(event.target);
            const d = event.source === event.target
                ? `M ${x - 24} ${y - 31} C ${x - 85} ${y - 92}, ${x + 85} ${y - 92}, ${x + 24} ${y - 31}`
                : tx > x
                    ? `M ${x + 87} ${y} C ${x + 155} ${y}, ${tx - 155} ${ty}, ${tx - 89} ${ty}`
                    : `M ${x} ${y + 31} C ${x} ${height - 10}, ${tx} ${height - 10}, ${tx} ${ty + 33}`;
            const path = svg('path', { d, class: `evaluation-edge ${event.source === selected ? 'active' : ''}`, 'marker-end': 'url(#evaluation-arrow)' });
            path.append(svg('title', {}, `Event ${event.id}: ${event.rule}; ${event.status}`));
            graph.append(path);
        }
        for (const item of state) {
            const [x, y] = position.get(item.id);
            const group = svg('g', { transform: `translate(${x} ${y})`, class: `evaluation-node ${item.id === selected ? 'selected' : ''}`, tabindex: 0, role: 'button', 'aria-label': `Inspect configuration ${item.id}: ${label(item)}`, 'aria-pressed': String(item.id === selected), 'data-state': item.id });
            group.append(svg('rect', { x: -86, y: -31, width: 172, height: 62, rx: 13 }));
            group.append(svg('text', { x: 0, y: -7, class: 'graph-caption' }, `CONFIGURATION ${item.id}`));
            const value = label(item);
            group.append(svg('text', { x: 0, y: 15 }, value.length > 22 ? value.slice(0, 19) + '…' : value));
            group.append(svg('title', {}, `${value}\n${item.status}`));
            const choose = () => { selected = item.id; render(); };
            group.addEventListener('click', choose);
            group.addEventListener('keydown', event => {
                if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); choose(); }
            });
            graph.append(group);
        }
        const item = report.state[selected];
        find('evaluation-state').textContent = label(item);
        find('evaluation-detail').textContent = JSON.stringify({ ...item, definition: report.definition }, null, 2);
        find('evaluation-summary').textContent = `${report.closed ? 'Closed exploration' : 'Exploration suspended'} · ${report.state.length} configurations · ${report.event.length} events · ${report.work} work steps${report.state.length > 32 ? ' · drawing shows the first 32 configurations' : ''}`;
        const event = find('evaluation-event');
        event.replaceChildren();
        for (const application of report.event.filter(event => event.source === selected)) {
            const button = text('button', `${application.rule} → configuration ${application.target}`);
            button.type = 'button';
            button.addEventListener('click', () => { selected = application.target; render(); });
            event.append(button);
        }
        if (!event.children.length) event.append(text('p', report.closed ? 'No outgoing events in this completed graph.' : 'No outgoing events recorded yet. This is not evidence that the configuration is final.'));
    };
    const finish = () => {
        clearTimeout(timer);
        worker?.terminate();
        worker = undefined;
        find('evaluation-run').disabled = false;
        find('evaluation-stop').disabled = true;
    };
    find('evaluation-run').addEventListener('click', () => {
        finish();
        if (location.protocol === 'file:') {
            find('evaluation-status').textContent = 'For live Rust execution, run bazel run -c opt //toolchain/browser:serve and open http://127.0.0.1:8080. The recorded graph works offline.';
            return;
        }
        let targets;
        try {
            targets = JSON.parse(find('evaluation-target').value);
            if (!Array.isArray(targets) || targets.some(value => typeof value !== 'string')) throw new Error();
        } catch {
            find('evaluation-status').textContent = 'Write targets as a JSON list of strings, such as ["D [A] B.C [B] D", "C.D [A] B.C [B] D"].';
            return;
        }
        const source = find('evaluation-source').value;
        const current = new Worker('document/worker.js', { type: 'module' });
        worker = current;
        find('evaluation-run').disabled = true;
        find('evaluation-stop').disabled = false;
        find('evaluation-verdict').replaceChildren();
        find('evaluation-status').textContent = 'The Rust runtime is exploring this program…';
        timer = setTimeout(() => { finish(); find('evaluation-status').textContent = 'Stopped after five seconds. No verdict was established; the previous graph remains visible.'; }, 5000);
        current.onerror = () => { finish(); find('evaluation-status').textContent = 'The worker could not run. Start the live book with bazel run -c opt //toolchain/browser:serve.'; };
        current.onmessage = ({ data }) => {
            if (worker !== current) return;
            finish();
            if (data.version !== 1 || data.error) {
                find('evaluation-status').textContent = `No new result. ${data.error?.message || 'The browser and runtime use different response versions.'} The previous graph remains visible.`;
                return;
            }
            report = data.execution;
            find('evaluation-used').textContent = source;
            selected = 0;
            find('evaluation-status').textContent = 'Executed here by the Rust runtime compiled to WebAssembly.';
            find('evaluation-verdict').replaceChildren(...data.verdict.map((verdict, index) => text('li', `${targets[index]}: ${verdict.outcome}${verdict.witness === null ? '' : ` (configuration ${verdict.witness})`}`)));
            render();
        };
        current.postMessage({ source, targets });
    });
    find('evaluation-stop').addEventListener('click', () => { finish(); find('evaluation-status').textContent = 'Stopped. No verdict was established; the previous graph remains visible.'; });
    find('evaluation-reset').addEventListener('click', () => {
        finish();
        report = globalThis.evaluation.report;
        find('evaluation-used').textContent = globalThis.evaluation.source;
        selected = 0;
        find('evaluation-source').value = globalThis.evaluation.source;
        find('evaluation-target').value = '["D [A] B.C [B] D", "C.D [A] B.C [B] D"]';
        find('evaluation-verdict').replaceChildren();
        find('evaluation-status').textContent = 'Recorded Rust execution. Run the source to explore it live.';
        render();
    });
    find('evaluation-source').value = globalThis.evaluation.source;
    find('evaluation-used').textContent = globalThis.evaluation.source;
    render();
})();
