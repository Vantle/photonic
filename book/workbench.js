(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const enhance = widget => {
        const sample = JSON.parse(widget.dataset.preset);
        const bar = element('div', 'bar');
        const badge = element('span', 'badge', 'recorded runs');
        const run = element('button', 'run', 'Run');
        run.type = 'button';
        run.hidden = true;
        run.title = 'Run (⌘ or Ctrl + Enter)';
        bar.append(element('span', 'title', 'Workbench'), badge, run);
        const body = element('div', 'body');
        const preset = element('div', 'preset');
        const editor = book.editor.create('Workbench program');
        editor.area.readOnly = true;
        const message = book.render.message();
        const filter = element('div', 'filter');
        const field = element('label', 'field');
        field.append('Filter by pattern');
        const input = element('input');
        input.spellcheck = false;
        input.autocomplete = 'off';
        input.placeholder = 'B.X · B, C · [B, C] D · ([A] B)';
        field.append(input);
        const clear = element('button', 'tool', 'Clear');
        clear.type = 'button';
        const note = element('p');
        filter.append(field, clear, note);
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
        const out = tool('−', 'Zoom out');
        const fit = tool('Fit', 'Fit the graph');
        const into = tool('+', 'Zoom in');
        const graph = panel('State graph', [out, fit, into]);
        const label = element('span');
        const flow = panel('Execution hypergraph', [label]);
        body.append(preset, editor.element, message.element, filter, graph.box, flow.box);
        widget.replaceChildren(bar, body);

        let current;
        let data;
        let pattern;
        let problem;
        let chosen;
        let control;
        let ticket = 0;
        let busy = false;
        const settle = () => {
            busy = false;
            run.removeAttribute('aria-disabled');
        };

        const follow = path => {
            label.textContent = path.length ? `s0 → s${path.at(-1).target} · ${path.length} event${path.length === 1 ? '' : 's'}` : 's0';
            book.hypergraph.draw(flow.host, data, path, { pattern });
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
            clear.hidden = !input.value;
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

        const load = item => {
            ticket++;
            settle();
            current = item;
            chosen = undefined;
            editor.value = item.source;
            preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === item.name)));
            message.say();
            render();
        };

        sample.forEach(item => {
            const button = element('button', undefined, item.name);
            button.type = 'button';
            button.addEventListener('click', () => {
                const entry = item.example ? book.record?.example?.[item.example] : book.record?.workbench?.[item.name];
                if (entry) load({ ...entry, name: item.name });
                else message.say('This preset has no recorded run. Regenerate the records with bazel run -c opt //book:record.', 'error');
            });
            preset.append(button);
        });

        const execute = async () => {
            if (run.hidden || busy) return;
            const mine = ++ticket;
            const source = editor.value;
            busy = true;
            run.setAttribute('aria-disabled', 'true');
            message.say('Running in WebAssembly…');
            try {
                const result = await book.engine.request({ kind: 'explore', request: book.example.request(current, source) });
                if (mine !== ticket) return;
                current = { ...current, name: '', source, result };
                chosen = undefined;
                preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', 'false'));
                message.say();
                render();
            } catch (error) {
                if (mine === ticket) message.say(book.editor.locate(editor.area, error), 'error');
            } finally {
                if (mine === ticket) settle();
            }
        };
        run.addEventListener('click', execute);
        editor.area.addEventListener('keydown', event => {
            if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
            event.preventDefault();
            execute();
        });
        let timer;
        input.addEventListener('input', () => {
            clearTimeout(timer);
            timer = setTimeout(() => apply(input.value), 160);
        });
        clear.addEventListener('click', () => apply(''));
        out.addEventListener('click', () => control.zoom(0.8));
        into.addEventListener('click', () => control.zoom(1.25));
        fit.addEventListener('click', () => control.zoom('fit'));
        book.engine.watch(state => {
            const on = state === 'live';
            run.hidden = !on;
            editor.area.readOnly = !on;
            badge.textContent = on ? 'live' : 'recorded runs';
        });

        const reveal = () => widget.scrollIntoView({ block: 'start' });
        book.workbench = {
            open: item => {
                load(item);
                reveal();
            },
            filter: text => {
                apply(text);
                reveal();
            },
            reveal,
        };
        preset.querySelector('button')?.click();
    };

    document.querySelectorAll('.workbench[data-preset]').forEach(enhance);
})();
