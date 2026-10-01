(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, vector, tally, series, count } = book.render;
    const { basis, cite, split, rule, gloss } = book.notation;

    const section = [['Triangles', '26'], ['Parallels', '32'], ['Parallelograms', '45'], ['Squares', '48']];

    const prepare = (entry, record) => {
        const line = record.source.split('\n');
        const first = line.findIndex(text => text.startsWith('['));
        const position = new Map();
        line.forEach((text, at) => {
            if (text.startsWith('[')) position.set(at, position.size);
        });
        const given = line.slice(0, first).flatMap(text => split(text.replace(/,$/, ''))).filter(text => !text.startsWith('(') && !book.figure.token(text));
        const step = record.event.map(([at, world]) => {
            const text = line[at].replace(/,$/, '');
            if (text.startsWith('(')) return { at, world, why: 'close', figure: new Set(), ...rule(text.slice(text.indexOf('['), text.lastIndexOf(')'))) };
            const detail = entry.rule[position.get(at)];
            return { at, world, why: detail.why, figure: new Set(detail.figure ?? []), ...rule(text) };
        });
        const cited = new Set(step.map(value => cite(value.why)).filter(name => name && name !== entry.name));
        return { ...entry, source: record.source, work: record.work, line, given, step, cited };
    };

    // Replacing each citation by the proof it cites, down to rules that cite nothing, counts the events
    // the proof would take without the propositions before it. A citation of an earlier part of the same
    // proposition counts as one event, so a count that meets one is a lower bound.
    const unfold = (name, table, known) => {
        if (known.has(name)) return known.get(name);
        const result = table.get(name).step.reduce(({ total, exact }, value) => {
            const cited = cite(value.why);
            if (!cited) return { total: total + 1, exact };
            if (cited === name) return { total: total + 1, exact: false };
            const inner = unfold(cited, table, known);
            return { total: total + inner.total, exact: exact && inner.exact };
        }, { total: 0, exact: true });
        known.set(name, result);
        return result;
    };

    const tool = (label, text) => {
        const node = element('button', 'tool', text);
        node.type = 'button';
        node.setAttribute('aria-label', label);
        return node;
    };

    const list = (label, fact, place, figure) => {
        const box = element('div');
        const group = element('ul', 'fact');
        const seen = new Map();
        fact.forEach((text, at) => {
            const prior = seen.get(text);
            if (prior) {
                prior.count++;
                prior.node.textContent = `×${prior.count}`;
                return;
            }
            const item = element('li');
            const code = element('code');
            code.append(book.syntax.fragment(text));
            const note = element('span', 'count');
            item.append(code, element('span', 'gloss', gloss(text, place)), note);
            if (figure.has(at)) item.append(element('span', 'origin', 'read from the figure'));
            seen.set(text, { count: 1, node: note });
            group.append(item);
        });
        box.append(element('span', 'label', label), group);
        return box;
    };

    const enhance = widget => {
        const record = book.record?.euclid;
        const bar = element('div', 'bar');
        bar.append(element('span', 'title', 'Euclid, Book I'));
        if (!record || !book.euclid || book.euclid.some(entry => !record[entry.name])) {
            const message = book.render.message();
            message.say('No recorded proofs. Regenerate the records with bazel run -c opt //book:record.', 'error');
            widget.replaceChildren(bar, message.element);
            return;
        }
        const order = book.euclid.map(entry => entry.name);
        const table = new Map(book.euclid.map(entry => [entry.name, prepare(entry, record[entry.name])]));
        const known = new Map();
        table.forEach((proof, name) => {
            proof.frame = book.figure.frame(proof);
            proof.unfolded = unfold(name, table, known);
        });
        const title = name => table.get(name).label;

        const badge = element('span', 'badge', 'recorded runs');
        const stop = element('button', undefined, 'Stop');
        stop.type = 'button';
        stop.hidden = true;
        const prove = element('button', 'run', 'Prove all');
        prove.type = 'button';
        prove.hidden = true;
        prove.title = `Prove all ${order.length} in WebAssembly`;
        bar.append(badge, stop, prove);

        const picker = element('select');
        picker.setAttribute('aria-label', 'Proposition');
        section.forEach(([name, last], index) => {
            const group = element('optgroup');
            group.label = name;
            const from = index ? order.indexOf(section[index - 1][1]) + 1 : 0;
            order.slice(from, order.indexOf(last) + 1).forEach(value => {
                const option = element('option', undefined, title(value));
                option.value = value;
                group.append(option);
            });
            picker.append(group);
        });
        const previous = tool('Previous proposition', '←');
        const next = tool('Next proposition', '→');
        const back = element('button', 'tool');
        back.type = 'button';
        back.hidden = true;
        const navigation = element('div', 'row navigation');
        navigation.append(previous, picker, next, back);
        const citation = book.citation.draw(order.map(name => table.get(name)), name => choose(name, 0, true));
        const chart = element('div', 'chart');
        chart.append(citation.element);
        const heading = element('div', 'heading');
        const summary = element('p', 'summary');
        const plane = vector('svg', { class: 'plane', role: 'img', tabindex: '0', 'aria-keyshortcuts': 'ArrowLeft ArrowRight Home End' });
        const panel = element('div', 'step');
        const board = element('div', 'board');
        board.append(plane, panel);
        const first = tool('First step', '⏮');
        const earlier = tool('Previous step', '←');
        const slider = element('input');
        slider.type = 'range';
        slider.min = 1;
        slider.setAttribute('aria-label', 'Step');
        const later = tool('Next step', '→');
        const last = tool('Last step', '⏭');
        const position = element('span', 'arrow');
        const transport = element('div', 'row transport');
        transport.append(first, earlier, slider, later, last, position);
        const program = element('details', 'program');
        const caption = element('summary');
        const message = book.render.message();
        const announcement = element('p', 'offscreen');
        announcement.setAttribute('aria-live', 'polite');
        const body = element('div', 'body');
        body.append(navigation, chart, heading, summary, board, transport, program, message.element, announcement);
        widget.replaceChildren(bar, body);

        let current;
        let index = 0;
        let row = [];
        let listing;
        let timer;
        let quiet = true;
        const history = [];

        const link = (name, text = title(name)) => {
            const node = element('button', 'cite', text);
            node.type = 'button';
            node.addEventListener('click', () => {
                history.push([current.name, index]);
                choose(name, 0, false);
                heading.querySelector('h3').focus();
            });
            return node;
        };

        const justify = step => {
            const cited = cite(step.why);
            if (cited) return title(cited);
            if (step.why === 'close') return `The end of case ${step.world}`;
            if (step.why === 'theorem') return 'Conclusion';
            return basis[step.why][0];
        };

        const explain = step => {
            const cited = cite(step.why);
            if (cited) return cited === current.name ? 'An earlier part of this proposition.' : table.get(cited).title;
            if (step.why === 'theorem') return current.kind === 'problem' ? 'Which was to be done.' : 'Which was to be proved.';
            if (step.why !== 'close') return basis[step.why][1];
            return step.output[0]?.startsWith('Refuted.')
                ? 'The case reached a contradiction, so its scope closes and reports the case impossible.'
                : 'The case reached the claim, so its scope closes and reports it.';
        };

        const reason = step => {
            const box = element('div', 'reason');
            const cited = cite(step.why);
            const name = cited && cited !== current.name ? link(cited) : element('span', 'basis', justify(step));
            box.append(element('span', 'label', 'by'), name, element('p', undefined, explain(step)));
            return box;
        };

        // A screen reader hears one line per settled step, not every position the slider passes.
        const announce = text => {
            clearTimeout(timer);
            if (quiet) return;
            timer = setTimeout(() => { announcement.textContent = text; }, 400);
        };

        const show = value => {
            index = Math.max(0, Math.min(current.step.length - 1, value));
            const step = current.step[index];
            slider.value = index + 1;
            position.textContent = `step ${index + 1} of ${current.step.length}`;
            const focused = [first, earlier, later, last].includes(document.activeElement);
            first.disabled = earlier.disabled = index === 0;
            later.disabled = last.disabled = index === current.step.length - 1;
            if (focused && document.activeElement.disabled) slider.focus();
            book.figure.paint(plane, current, index);
            plane.setAttribute('aria-label', `The figure of ${title(current.name)} at step ${index + 1}`);
            const place = { ...current.point, ...current.arrangement?.[step.world] };
            const where = element('p', 'where');
            where.append(`Step ${index + 1} of ${current.step.length}`);
            if (step.world) where.append(element('span', 'case', `case ${step.world}`));
            panel.replaceChildren(where, list('from', step.input, place, new Set()), reason(step), list('gives', step.output, place, step.figure));
            const reading = step.output.map(text => gloss(text, place)).filter(Boolean).join(', ');
            announce(`Step ${index + 1} of ${current.step.length}${step.world ? `, case ${step.world}` : ''}: ${justify(step)}. ${reading.charAt(0).toUpperCase()}${reading.slice(1)}.`);
            row.forEach((node, at) => {
                if (at === step.at) node.dataset.current = '';
                else delete node.dataset.current;
            });
            if (!program.open) return;
            listing.scrollTop = row[step.at].offsetTop - listing.clientHeight / 2 + row[step.at].offsetHeight / 2;
        };

        const choose = (name, start, clear) => {
            if (clear) history.length = 0;
            current = table.get(name);
            picker.value = name;
            const at = order.indexOf(name);
            const focused = [previous, next].includes(document.activeElement);
            previous.disabled = at === 0;
            next.disabled = at === order.length - 1;
            if (focused && document.activeElement.disabled) picker.focus();
            const [origin, step] = history.at(-1) ?? [];
            back.hidden = !origin;
            back.textContent = origin ? `Back to ${title(origin)}, step ${step + 1}` : '';
            citation.show(name);
            const label = element('h3', undefined, title(name));
            label.tabIndex = -1;
            heading.replaceChildren(label, element('span', 'kind', current.kind === 'problem' ? 'Problem' : 'Theorem'), element('p', undefined, current.title));
            summary.replaceChildren(tally(current.step.length, 'event'), tally(current.work, 'work step'));
            const cited = order.filter(value => current.cited.has(value));
            if (cited.length) {
                const reference = element('span', undefined, 'cites ');
                cited.forEach((value, ordinal) => {
                    if (ordinal) reference.append(ordinal === cited.length - 1 ? ' and ' : ', ');
                    reference.append(link(value, /^\d+$/.test(table.get(value).mark) ? value : title(value)));
                });
                const unfolded = tally(current.unfolded.total, 'event');
                if (!current.unfolded.exact) unfolded.prepend('at least ');
                unfolded.append(' with every citation unfolded to the axioms');
                summary.append(reference, unfolded);
            }
            listing = element('pre', 'code');
            row = current.line.map(text => {
                const node = element('span', 'line');
                node.append(text ? book.syntax.fragment(text) : ' ');
                listing.append(node);
                return node;
            });
            caption.textContent = `${widget.dataset.source}/${name}.wave · ${count(current.line.filter(text => text.startsWith('[')).length, 'rule')}`;
            program.replaceChildren(caption, listing);
            slider.max = current.step.length;
            show(start);
        };

        picker.addEventListener('change', () => choose(picker.value, 0, true));
        previous.addEventListener('click', () => choose(order[order.indexOf(current.name) - 1], 0, true));
        next.addEventListener('click', () => choose(order[order.indexOf(current.name) + 1], 0, true));
        back.addEventListener('click', () => {
            const [name, step] = history.pop();
            choose(name, step, false);
            heading.querySelector('h3').focus();
        });
        first.addEventListener('click', () => show(0));
        earlier.addEventListener('click', () => show(index - 1));
        later.addEventListener('click', () => show(index + 1));
        last.addEventListener('click', () => show(current.step.length - 1));
        slider.addEventListener('input', () => show(Number(slider.value) - 1));
        program.addEventListener('toggle', () => show(index));
        const key = { ArrowLeft: () => index - 1, ArrowRight: () => index + 1, Home: () => 0, End: () => current.step.length - 1 };
        plane.addEventListener('keydown', event => {
            if (!key[event.key] || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
            event.preventDefault();
            show(key[event.key]());
        });

        const follow = book.engine.path();
        const cycle = book.run.create({ trigger: prove, stop, message, text: 'Proving in WebAssembly…' });
        prove.addEventListener('click', () => cycle.start(async signal => {
            const begin = performance.now();
            const result = [];
            for (const name of order) {
                const request = book.engine.request({ source: table.get(name).source, library: [], target: ['Theorem'], preserve: true });
                result.push([name, await follow('path', request, { timeout: 30000, signal })]);
            }
            return { result, time: performance.now() - begin };
        }, ({ result, time }) => {
            const failed = result.filter(([, progress]) => progress.outcome !== 'reached').map(([name]) => title(name));
            if (failed.length) {
                message.say(`${series(failed)} did not reach Theorem.`, 'error');
                return;
            }
            const event = result.reduce((sum, [, progress]) => sum + progress.event, 0);
            const work = result.reduce((sum, [, progress]) => sum + progress.work, 0);
            const duration = time < 1000 ? `${Math.round(time)} ms` : `${(time / 1000).toFixed(1)} s`;
            message.say(`All ${result.length} proofs reach Theorem in WebAssembly: ${count(event, 'event')} and ${count(work, 'work step')} in ${duration}.`);
        }, error => message.say(error.message, 'error')));

        book.engine.watch(state => {
            const on = state === 'live';
            prove.hidden = !on;
            badge.textContent = on ? 'live' : 'recorded runs';
        });

        choose(order[0], 0, true);
        quiet = false;
    };

    document.querySelectorAll('.widget.geometry').forEach(enhance);
})();
