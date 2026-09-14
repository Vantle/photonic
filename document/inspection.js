(() => {
    const element = (tag, text, style) => {
        const node = document.createElement(tag);
        if (text !== undefined) node.textContent = text;
        if (style) node.className = style;
        return node;
    };
    const render = (target, state, event, before) => {
        const fragment = document.createDocumentFragment();
        fragment.append(element('p', `Configuration ${state.id} · ${state.world.length} coherences · ${state.frame.length} frames`, 'caption'));
        for (const [index, world] of state.world.entries()) {
            const card = element('section', undefined, 'inspection-world');
            card.append(element('h5', `Coherence ${index} · frame ${world.frame}`));
            const row = element('div', undefined, 'inspection-token');
            for (const token of world.particle) {
                const matches = place => place.world?.[0] === index && place.world?.[1] === token.id;
                const role = before && event.exact.some(matches) ? 'consumed' : before && event.footprint.some(matches) ? 'bound' : before && event.read.some(matches) ? 'read' : '';
                const label = `Occurrence ${token.id}${token.capture === undefined ? '' : ` · captured frame ${token.capture}`}${role ? ` · ${role}` : ''}`;
                if (token.capture !== undefined) {
                    const detail = element('details', undefined, `inspection-rule ${role}`);
                    detail.append(element('summary', `${token.label} · ${label}`));
                    const code = element('pre');
                    globalThis.syntax.highlight(code, token.display);
                    detail.append(code);
                    row.append(detail);
                } else {
                    const chip = element('span', undefined, `token ${role}`);
                    chip.title = label;
                    globalThis.syntax.highlight(chip, token.display);
                    row.append(chip);
                }
            }
            if (!world.particle.length) row.append(element('span', 'Empty coherence', 'caption'));
            card.append(row);
            fragment.append(card);
        }
        if (!state.world.length) fragment.append(element('p', 'No coherences.'));
        const detail = element('details');
        detail.append(element('summary', 'Complete identity and frame data'));
        detail.append(element('pre', JSON.stringify(state, null, 2)));
        fragment.append(detail);
        target.replaceChildren(fragment);
    };
    globalThis.inspection = { render };
})();
