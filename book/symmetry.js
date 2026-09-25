(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, series } = book.render;
    const title = { global: 'Global', local: 'Local', block: 'Block' };
    const meaning = { global: 'what they exchange', local: 'rules that repeat under other names', block: 'atoms that always appear together' };
    const shown = 12;
    const selector = '.token:not(.rule), .atom, [data-event]';

    const rename = (from, to) => {
        const next = new Map(from.map((atom, index) => [atom, to[index]]).filter(([atom, image]) => atom !== image));
        const image = new Set(next.values());
        const seen = new Set();
        const walk = start => {
            const path = [start];
            seen.add(start);
            for (let atom = next.get(start); atom !== undefined && !seen.has(atom); atom = next.get(atom)) {
                path.push(atom);
                seen.add(atom);
            }
            return path;
        };
        const piece = [...next.keys()].filter(atom => !image.has(atom)).map(atom => walk(atom).join(' → '));
        for (const atom of next.keys()) {
            if (seen.has(atom)) continue;
            const cycle = walk(atom);
            piece.push(cycle.length === 2 ? cycle.join(' ⇄ ') : `(${cycle.join(' ')})`);
        }
        return piece.join(', ');
    };

    const describe = entry => {
        if (entry.kind === 'block') return entry.part[0].join('.');
        if (entry.kind === 'global') return entry.part.map(series).join('; ');
        if (entry.part.length === 2) return rename(...entry.part);
        return entry.part.map(part => part.join(' ')).join(' / ');
    };

    const create = host => {
        const box = element('section', 'panel symmetry');
        const header = element('header');
        const check = element('label', 'check');
        const toggle = element('input');
        toggle.type = 'checkbox';
        toggle.checked = true;
        check.append(toggle, 'Color');
        header.append(element('h4', undefined, 'Symmetry'), check);
        const summary = element('p');
        const list = element('div', 'class');
        const body = element('div', 'body');
        body.append(summary, list);
        box.append(header, body);
        box.hidden = true;

        let atom = new Map();
        let rule = new Map();
        let event = [];
        let lit;
        let pinned;

        const paint = node => {
            const code = node.dataset.event;
            const name = code === undefined ? node.textContent : event[code]?.rule;
            const kind = (code === undefined ? atom : rule).get(name);
            const member = code === undefined ? lit?.atom : lit?.rule;
            if (kind) node.dataset.symmetry = kind;
            else delete node.dataset.symmetry;
            if (member?.has(name)) node.dataset.glow = lit.kind;
            else delete node.dataset.glow;
        };

        const tint = root => {
            if (root.nodeType !== Node.ELEMENT_NODE) return;
            if (root.matches(selector)) paint(root);
            root.querySelectorAll(selector).forEach(paint);
        };

        const light = entry => {
            lit = entry;
            host.forEach(node => {
                if (entry) node.dataset.glow = entry.kind;
                else delete node.dataset.glow;
                tint(node);
            });
        };

        const color = () => host.forEach(node => {
            if (toggle.checked) node.dataset.tint = '';
            else delete node.dataset.tint;
        });

        const chip = entry => {
            const button = element('button');
            button.type = 'button';
            button.dataset.symmetry = entry.kind;
            button.setAttribute('aria-pressed', 'false');
            button.append(element('i'), element('b', undefined, title[entry.kind]), ` ${describe(entry)}`);
            button.addEventListener('pointerenter', () => light(entry));
            button.addEventListener('pointerleave', () => light(pinned));
            button.addEventListener('focus', () => light(entry));
            button.addEventListener('blur', () => light(pinned));
            button.addEventListener('click', () => {
                pinned = pinned === entry ? undefined : entry;
                list.querySelectorAll('button[data-symmetry]').forEach(other => other.setAttribute('aria-pressed', String(other === button && pinned === entry)));
                light(entry);
            });
            return button;
        };

        const show = (analysis, execution) => {
            const entry = (analysis?.class ?? []).map(value => ({ ...value, atom: new Set(value.part.flat()), rule: new Set(value.rule) }));
            event = execution?.event ?? [];
            atom = new Map();
            rule = new Map();
            for (const value of entry) {
                value.atom.forEach(name => atom.set(name, atom.get(name) ?? value.kind));
                value.rule.forEach(name => rule.set(name, rule.get(name) ?? value.kind));
            }
            pinned = undefined;
            light();
            box.hidden = !analysis;
            if (!analysis) return;
            const count = analysis.size === '1' ? 'Only the identity leaves this program unchanged' : `${analysis.size} renamings leave this program unchanged`;
            const present = Object.keys(meaning).filter(kind => entry.some(value => value.kind === kind));
            summary.textContent = present.length
                ? `${count}. Colors mark ${series(present.map(kind => meaning[kind]))}.`
                : `${count}, and no rules repeat under other names.`;
            list.replaceChildren(...entry.slice(0, shown).map(chip));
            if (entry.length > shown) {
                const more = element('button', 'more', `${entry.length - shown} more`);
                more.type = 'button';
                more.addEventListener('click', () => more.replaceWith(...entry.slice(shown).map(chip)));
                list.append(more);
            }
            list.hidden = !entry.length;
        };

        toggle.addEventListener('change', color);
        color();
        const observer = new MutationObserver(record => record.forEach(value => value.addedNodes.forEach(tint)));
        host.forEach(node => observer.observe(node, { childList: true, subtree: true }));
        return { element: box, show };
    };

    book.symmetry = { create, describe };
})();
