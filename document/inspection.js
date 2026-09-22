(() => {
    const element = (tag, text, style) => {
        const node = document.createElement(tag);
        if (text !== undefined) node.textContent = text;
        if (style) node.className = style;
        return node;
    };
    const particle = (value, kind, index, event, before, definition) => {
        const row = element('div', undefined, 'inspection-token');
        for (const token of value) {
            const matches = place => place[kind]?.[0] === index && place[kind]?.[1] === token.id;
            const role = before && event.exact.some(matches) ? 'consumed' : before && event.footprint.some(matches) ? 'bound' : before && event.read.some(matches) ? 'read' : '';
            const label = `Occurrence ${token.id}${token.capture === undefined ? '' : ` · captured frame ${token.capture}`}${role ? ` · ${role}` : ''}`;
            const display = token.display ?? definition.get(token.label) ?? token.label;
            if (token.capture !== undefined) {
                const detail = element('details', undefined, `inspection-rule ${role}`);
                detail.append(element('summary', `${token.label} · ${label}`));
                const code = element('pre');
                globalThis.syntax.highlight(code, display);
                detail.append(code);
                row.append(detail);
            } else {
                const chip = element('span', undefined, `token ${role}`);
                chip.title = label;
                globalThis.syntax.highlight(chip, display);
                row.append(chip);
            }
        }
        if (!value.length) row.append(element('span', `Empty ${kind === 'world' ? 'coherence' : 'context'}`, 'caption'));
        return row;
    };
    const render = (target, state, event, before, definition = []) => {
        const lookup = new Map(definition.map(value => [value.label, value.display]));
        const fragment = document.createDocumentFragment();
        fragment.append(element('p', `Configuration ${state.id} · ${state.world.length} coherences · ${state.frame.length} contexts`, 'caption'));
        for (const [index, world] of state.world.entries()) {
            const card = element('section', undefined, 'inspection-world');
            card.append(element('h5', `Coherence ${index} · context ${world.frame}`));
            card.append(particle(world.particle, 'world', index, event, before, lookup));
            fragment.append(card);
        }
        if (!state.world.length) fragment.append(element('p', 'No coherences.'));
        for (const [index, frame] of state.frame.entries()) {
            const card = element('section', undefined, 'inspection-world');
            card.append(element('h5', `Context ${index} · ${frame.scope}`));
            card.append(particle(frame.particle, 'context', index, event, before, lookup));
            fragment.append(card);
        }
        const detail = element('details');
        detail.append(element('summary', 'Complete identity and context data'));
        detail.append(element('pre', JSON.stringify(state, null, 2)));
        fragment.append(detail);
        target.replaceChildren(fragment);
    };
    globalThis.inspection = { render };
})();
