(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, tally } = book.render;

    const card = (title, node, definition, touch) => {
        const box = element('div', 'state');
        box.append(element('span', 'name', title));
        if (node) box.append(book.render.state(node, { definition, touch }));
        return box;
    };

    const draw = (host, source) => {
        const box = element('div', 'stepper');
        const summary = element('p', 'summary');
        if (source.outcome) summary.append(element('span', `badge ${source.outcome}`, source.outcome));
        summary.append(tally(source.count, 'event'), tally(source.work, 'work step'));
        if (!source.quiet) box.append(summary);
        if (!source.count) {
            box.append(element('p', 'summary', 'No rule applies to the initial configuration.'));
            host.replaceChildren(box);
            return;
        }
        const control = element('div', 'row');
        const button = (label, text) => {
            const node = element('button', 'tool', text);
            node.type = 'button';
            node.setAttribute('aria-label', label);
            control.append(node);
            return node;
        };
        const first = button('First event', '⏮');
        const back = button('Previous event', '←');
        const slider = element('input');
        slider.type = 'range';
        slider.min = 1;
        slider.max = source.count;
        slider.value = 1;
        slider.setAttribute('aria-label', 'Event');
        control.append(slider);
        const next = button('Next event', '→');
        const last = button('Last event', '⏭');
        const position = element('span', 'arrow');
        control.append(position);
        const rule = element('pre', 'code');
        const pair = element('div', 'pair');
        box.append(control, rule, pair);
        host.replaceChildren(box);
        let index = 0;
        let wanted;
        let busy = false;
        const paint = step => {
            book.syntax.highlight(rule, step.event.rule);
            const before = element('div');
            before.append(element('span', 'label', 'before · consumed tokens outlined'));
            before.append(card(`s${step.event.source}`, step.before, source.definition, book.render.touch(step.event)));
            const after = element('div');
            after.append(element('span', 'label', 'after'));
            after.append(card(`s${step.event.target}`, step.after, source.definition));
            pair.replaceChildren(before, after);
        };
        const load = async () => {
            busy = true;
            while (wanted !== undefined) {
                const at = wanted;
                wanted = undefined;
                try {
                    const step = await source.get(at);
                    if (wanted === undefined) paint(step);
                } catch (error) {
                    if (wanted !== undefined) continue;
                    rule.textContent = error.message;
                    pair.replaceChildren();
                }
            }
            busy = false;
        };
        const show = value => {
            index = Math.max(0, Math.min(source.count - 1, value));
            slider.value = index + 1;
            position.textContent = `event ${(index + 1).toLocaleString()} of ${source.count.toLocaleString()}`;
            const focused = [first, back, next, last].includes(document.activeElement);
            first.disabled = back.disabled = index === 0;
            next.disabled = last.disabled = index === source.count - 1;
            if (focused && document.activeElement.disabled) slider.focus();
            wanted = index;
            if (!busy) load();
        };
        first.addEventListener('click', () => show(0));
        back.addEventListener('click', () => show(index - 1));
        next.addEventListener('click', () => show(index + 1));
        last.addEventListener('click', () => show(source.count - 1));
        slider.addEventListener('input', () => show(Number(slider.value) - 1));
        show(source.start ?? 0);
    };

    const recorded = result => ({
        outcome: result.outcome,
        count: result.event,
        work: result.work,
        definition: result.definition,
        get: async index => {
            const event = result.step[index];
            return { event, before: result.state[event.source], after: result.state[event.target] };
        },
    });

    book.trace = { draw, recorded };
})();
