(() => {
    const select = document.getElementById('native-example');
    const source = document.getElementById('native-source');
    const summary = document.getElementById('native-summary');
    const graph = document.getElementById('native-graph');
    const state = document.getElementById('native-state');
    const event = document.getElementById('native-event');
    const detail = document.getElementById('native-detail');
    const previous = document.getElementById('native-back');
    const reset = document.getElementById('native-reset');
    let selected = 0;
    let history = [];
    const element = (tag, text) => {
        const value = document.createElement(tag);
        value.textContent = text;
        return value;
    };
    const drawing = (tag, attribute, text) => {
        const value = document.createElementNS('http://www.w3.org/2000/svg', tag);
        for (const [name, content] of Object.entries(attribute)) value.setAttribute(name, content);
        if (text !== undefined) value.textContent = text;
        return value;
    };
    const move = (target, application) => {
        history.push(selected);
        selected = target;
        render(application);
    };
    const render = application => {
        const fixture = globalThis.native[Number(select.value)];
        const report = fixture.report;
        source.textContent = fixture.source;
        summary.textContent = `${fixture.description} ${report.closed ? 'Closed graph.' : 'Suspended exploration; this is a prefix, not a completed result.'} ${report.state.length} configurations, ${report.event.length} events.`;
        previous.disabled = history.length === 0;
        state.replaceChildren(element('h3', `Configuration ${selected} · ${report.state[selected].status}`));
        for (const [index, world] of report.state[selected].world.entries()) {
            const box = element('div', '');
            box.className = 'native-world';
            box.append(element('strong', `Coherence ${index} · frame ${world.frame}`));
            box.append(element('pre', world.particle.map(token => `${token.display}  #${token.id}${token.capture === undefined ? '' : ` · capture ${token.capture}`}`).join('\n') || '()'));
            state.append(box);
        }
        const inspection = element('details', '');
        inspection.append(element('summary', 'Inspect nested values and captures'));
        inspection.append(element('pre', JSON.stringify(report.state[selected], null, 2)));
        state.append(inspection);
        event.replaceChildren(element('h3', 'Available events'));
        const outgoing = report.event.filter(value => value.source === selected);
        for (const value of outgoing) {
            const button = element('button', `${value.rule} → configuration ${value.target} · ${value.status}`);
            button.addEventListener('click', () => move(value.target, value));
            event.append(button);
        }
        if (!outgoing.length) event.append(element('p', report.closed ? 'No outgoing event in this closed graph.' : 'No outgoing event recorded in this exploration prefix.'));
        detail.textContent = application ? `Event ${application.id} · ${application.status}\n${application.rule}\nConsumed source footprint: ${JSON.stringify(application.footprint)}\nExact source matches: ${JSON.stringify(application.exact)}\nRead dependence: ${JSON.stringify(application.read)}\nEvidence views: ${application.evidence.join(', ')}` : 'Choose an event to move forward. Each move follows an actual Rust event; inferred applications can jump directly from the concrete source.';
        const depth = new Map([[0, 0]]);
        const queue = [0];
        for (let index = 0; index < queue.length; index++) {
            for (const value of report.event.filter(value => value.source === queue[index])) {
                if (depth.has(value.target)) continue;
                depth.set(value.target, depth.get(value.source) + 1);
                queue.push(value.target);
            }
        }
        const column = new Map();
        const position = new Map();
        for (const value of report.state) {
            const level = depth.get(value.id) ?? 0;
            const row = column.get(level) ?? 0;
            column.set(level, row + 1);
            position.set(value.id, [45 + level * 120, 40 + row * 70]);
        }
        const width = Math.max(400, 100 + Math.max(...depth.values()) * 120);
        const height = Math.max(120, 30 + Math.max(...column.values()) * 70);
        const svg = drawing('svg', {viewBox: `0 0 ${width} ${height}`, width, height, role: 'group', 'aria-label': 'Rust configuration graph'});
        const defs = drawing('defs', {});
        const marker = drawing('marker', {id: 'native-arrow', viewBox: '0 0 10 10', refX: 8, refY: 5, markerWidth: 5, markerHeight: 5, orient: 'auto-start-reverse'});
        marker.append(drawing('path', {d: 'M 0 0 L 10 5 L 0 10 z', fill: '#a6a099'}));
        defs.append(marker);
        svg.append(defs);
        for (const value of report.event) {
            const [x, y] = position.get(value.source);
            const [target, row] = position.get(value.target);
            const path = value.source === value.target ? `M ${x - 10} ${y - 16} C ${x - 45} ${y - 60}, ${x + 45} ${y - 60}, ${x + 10} ${y - 16}` : `M ${x + 19} ${y} C ${(x + target) / 2} ${y}, ${(x + target) / 2} ${row}, ${target - 20} ${row}`;
            const edge = drawing('path', {d: path, fill: 'none', stroke: '#bdb6ad', 'stroke-width': 1.2, 'marker-end': 'url(#native-arrow)'});
            edge.append(drawing('title', {}, `Event ${value.id}: ${value.rule} · ${value.status}`));
            svg.append(edge);
        }
        for (const value of report.state) {
            const [x, y] = position.get(value.id);
            const node = drawing('g', {role: 'button', tabindex: 0, 'aria-label': `Inspect configuration ${value.id}, ${value.status}`, style: 'cursor:pointer'});
            node.append(drawing('circle', {cx: x, cy: y, r: 19, fill: value.id === selected ? '#28251f' : value.status === 'supported' ? '#eeece5' : '#fff3d6', stroke: '#28251f'}));
            node.append(drawing('text', {x, y: y + 5, 'text-anchor': 'middle', style: `fill:${value.id === selected ? '#fff' : '#28251f'}`, 'font-size': 13}, value.id));
            node.addEventListener('click', () => move(value.id));
            node.addEventListener('keydown', key => {
                if (key.key !== 'Enter' && key.key !== ' ') return;
                key.preventDefault();
                move(value.id);
            });
            svg.append(node);
        }
        graph.replaceChildren(svg);
    };
    globalThis.native.forEach((fixture, index) => {
        const option = element('option', fixture.title);
        option.value = index;
        select.append(option);
    });
    select.addEventListener('change', () => { selected = 0; history = []; render(); });
    previous.addEventListener('click', () => { selected = history.pop() ?? 0; render(); });
    reset.addEventListener('click', () => { selected = 0; history = []; render(); });
    render();
})();
