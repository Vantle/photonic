(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const option = figure => ({
        name: figure.dataset.example,
        mode: figure.dataset.mode ?? 'explore',
        library: (figure.dataset.library ?? '').split(/\s+/).filter(Boolean),
        target: JSON.parse(figure.dataset.target ?? '[]'),
        preserve: figure.hasAttribute('data-preserve'),
    });

    const shelf = setting => {
        const row = element('div', 'library');
        row.append(element('span', undefined, 'Loads'));
        for (const name of setting.library) {
            const detail = element('details');
            const summary = element('summary');
            summary.append(element('code', undefined, `${name}.particle`));
            const code = element('pre', 'code');
            detail.addEventListener('toggle', () => {
                if (detail.open && !code.childNodes.length) book.syntax.highlight(code, book.record?.library?.[name] ?? 'Library source not recorded.');
            });
            detail.append(summary, code);
            row.append(detail);
        }
        return row;
    };

    const show = (host, setting, result, target) => {
        if (setting.mode === 'path') {
            book.trace.draw(host, result.step ? book.trace.recorded(result) : result);
            return;
        }
        book.graph.draw(host, book.graph.model(result.execution), { verdict: result.verdict, target });
    };

    const enhance = figure => {
        const setting = option(figure);
        const pre = figure.querySelector('pre');
        const original = pre.textContent;
        const record = book.record?.example?.[setting.name];
        const bar = element('div', 'bar');
        bar.append(element('span', 'title', setting.mode === 'path' ? 'Direct path' : 'Program'));
        const badge = element('span', 'badge', 'recorded run');
        const bench = element('button', undefined, 'Workbench');
        bench.type = 'button';
        bench.title = 'Open this program in the workbench';
        bench.hidden = setting.mode === 'path' || !document.querySelector('.workbench');
        const lightbox = element('a', undefined, 'Lightbox');
        lightbox.title = 'Open this program in the Lightbox';
        lightbox.hidden = setting.mode === 'path';
        const reset = element('button', undefined, 'Reset');
        reset.type = 'button';
        reset.hidden = true;
        const stop = element('button', undefined, 'Stop');
        stop.type = 'button';
        stop.hidden = true;
        const run = book.run.button();
        bar.append(badge, bench, lightbox, reset, stop, run);
        figure.insertBefore(bar, pre);
        if (setting.library.length) figure.insertBefore(shelf(setting), pre);
        const editor = book.editor.create(`Edit the ${setting.name} program`);
        editor.element.hidden = true;
        const goal = book.editor.goal(setting.mode === 'path' ? 'Target configuration' : 'Target configurations, one per line', Math.max(1, setting.target.length));
        goal.value = setting.target;
        goal.area.readOnly = true;
        goal.element.hidden = !setting.target.length;
        const message = book.render.message();
        const output = element('div', 'output');
        pre.after(editor.element, goal.element, message.element, output);
        book.syntax.highlight(pre, original);
        const recorded = record && { source: original, target: setting.target, result: record.result };
        let latest = recorded;
        const restore = () => {
            latest = recorded;
            if (record) show(output, setting, record.result, setting.target);
        };
        if (!record) message.say('No recorded run. Regenerate the records with bazel run -c opt //book:record.', 'error');
        else if (record.source !== original) message.say('This recorded run is stale. Regenerate it with bazel run -c opt //book:record.', 'error');
        restore();
        const channel = setting.mode === 'path' ? book.engine.open() : undefined;
        const execute = async (source, target, signal) => {
            if (!channel) return book.engine.explore(setting, source, target, signal);
            const progress = await channel.send('path', book.engine.request(setting, source, target), { timeout: 60000, signal });
            return {
                outcome: progress.outcome,
                count: progress.event,
                work: progress.work,
                definition: progress.definition,
                get: index => channel.send('inspect', { index }),
            };
        };
        const changed = () => editor.value !== original || goal.area.value !== setting.target.join('\n');
        const point = () => {
            lightbox.href = book.share.link({ ...setting, source: editor.element.hidden ? original : editor.value, target: goal.value });
        };
        point();
        const cycle = book.run.create({ trigger: run, stop: channel && stop, message });
        const start = () => {
            const source = editor.value;
            const target = goal.value;
            const submission = book.editor.submit(figure, editor, goal);
            cycle.start(signal => execute(source, target, signal), result => {
                if (!channel) latest = { source, target, result };
                show(output, setting, result, target);
            }, error => {
                if (channel && !error.detail) restore();
                message.say(book.editor.locate(error, submission), 'error');
            });
        };
        book.engine.watch(state => {
            const on = state === 'live';
            run.hidden = !on;
            badge.textContent = on ? 'live' : 'recorded run';
            goal.area.readOnly = !on;
            editor.area.readOnly = !on;
            if (!on || !editor.element.hidden) return;
            editor.element.hidden = false;
            editor.value = original;
            pre.hidden = true;
        });
        run.addEventListener('click', start);
        for (const area of [editor.area, goal.area]) {
            area.addEventListener('input', () => {
                reset.hidden = !changed();
                point();
            });
            book.run.shortcut(area, start);
        }
        reset.addEventListener('click', () => {
            cycle.cancel();
            editor.value = original;
            goal.value = setting.target;
            reset.hidden = true;
            message.say();
            point();
            restore();
        });
        bench.addEventListener('click', () => {
            if (!latest) return;
            book.workbench.open({ name: setting.name, library: setting.library, preserve: setting.preserve, ...latest });
        });
    };

    document.querySelectorAll('figure.example[data-example]').forEach(enhance);
})();
