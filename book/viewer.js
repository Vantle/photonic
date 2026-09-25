(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const panel = (title, extra) => {
        const box = element('section', 'panel');
        const header = element('header');
        header.append(element('h4', undefined, title), ...extra);
        const host = element('div');
        box.append(header, host);
        return { box, host };
    };

    const tool = (text, label) => {
        const button = element('button', 'tool', text);
        button.type = 'button';
        button.setAttribute('aria-label', label);
        return button;
    };

    const create = () => {
        const filter = element('div', 'filter');
        const field = element('label', 'field');
        field.append('Filter by pattern');
        const input = element('input');
        input.spellcheck = false;
        input.autocomplete = 'off';
        input.placeholder = 'B.X · B, C · [B, C] D · ([A] B)';
        field.append(input);
        const clear = tool('Clear', 'Clear the filter');
        const note = element('p');
        filter.append(field, clear, note);
        const out = tool('−', 'Zoom out');
        const fit = tool('Fit', 'Fit the graph');
        const into = tool('+', 'Zoom in');
        const graph = panel('State graph', [out, fit, into]);
        const label = element('span');
        const flow = panel('Execution hypergraph', [label]);
        const symmetry = book.symmetry.create([graph.box, flow.box]);

        let current;
        let data;
        let pattern;
        let problem;
        let chosen;
        let control;
        let explained;

        const explain = event => {
            explained = event;
            control?.explain(event);
        };

        const follow = path => {
            label.textContent = path.length ? `s0 → s${path.at(-1).target} · ${path.length} event${path.length === 1 ? '' : 's'}` : 's0';
            explain();
            book.hypergraph.draw(flow.host, data, path, { pattern, select: explain });
        };

        const deepest = visible => {
            const witness = (current.result.verdict ?? []).map(value => value.witness).find(value => value !== null && value !== undefined && visible(value));
            if (witness !== undefined) return witness;
            const candidate = data.state.map(value => value.id).filter(visible);
            const end = candidate.filter(state => !data.outgoing.get(state).length);
            let best;
            let length = -1;
            for (const state of end.length ? end : candidate) {
                const size = book.graph.route(data, state).length;
                if (size <= length) continue;
                best = state;
                length = size;
            }
            return best;
        };

        const render = () => {
            clear.hidden = !input.value;
            if (!current) return;
            data = book.graph.model(current.result.execution);
            const match = pattern ? book.pattern.state(pattern, data) : undefined;
            const visible = state => !match || match.state.has(state);
            if (chosen !== undefined && !visible(chosen)) chosen = undefined;
            chosen ??= deepest(visible);
            note.textContent = problem ?? (match
                ? `Showing ${match.state.size} of ${data.state.length} configurations: the matches and everything computed after them.`
                : 'Type a pattern to keep what matches and everything computed after it.');
            if (problem) note.dataset.tone = 'error';
            else delete note.dataset.tone;
            explained = undefined;
            control = book.graph.draw(graph.host, data, {
                verdict: current.result.verdict,
                target: current.target,
                filter: match,
                route: true,
                start: chosen,
                select: (state, path) => {
                    chosen = state;
                    follow(path);
                },
            });
            if (chosen === undefined) follow([]);
            control.explain(explained);
        };

        const apply = text => {
            input.value = text;
            try {
                pattern = book.pattern.read(text);
                problem = undefined;
            } catch (error) {
                problem = error.message;
            }
            render();
        };

        const show = (result, target) => {
            current = { result, target };
            chosen = undefined;
            symmetry.show(result.symmetry, result.execution);
            render();
        };

        const blank = text => {
            current = undefined;
            control = undefined;
            explained = undefined;
            note.textContent = 'Type a pattern to keep what matches and everything computed after it.';
            label.textContent = '';
            symmetry.show();
            graph.host.replaceChildren(element('p', 'blank', text));
            flow.host.replaceChildren();
        };

        let timer;
        input.addEventListener('input', () => {
            clearTimeout(timer);
            timer = setTimeout(() => apply(input.value), 160);
        });
        clear.addEventListener('click', () => apply(''));
        out.addEventListener('click', () => control?.zoom(0.8));
        into.addEventListener('click', () => control?.zoom(1.25));
        fit.addEventListener('click', () => control?.zoom('fit'));

        return { element: [filter, symmetry.element, graph.box, flow.box], filter: apply, show, blank };
    };

    book.viewer = { create };
})();
