(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const enhance = widget => {
        const sample = JSON.parse(widget.dataset.preset);
        const bar = element('div', 'bar');
        const badge = element('span', 'badge', 'recorded runs');
        const lightbox = element('a', undefined, 'Open in Lightbox');
        lightbox.title = 'Continue with this program in the Lightbox';
        const run = element('button', 'run', 'Run');
        run.type = 'button';
        run.hidden = true;
        run.title = 'Run (⌘ or Ctrl + Enter)';
        bar.append(element('span', 'title', 'Workbench'), badge, lightbox, run);
        const body = element('div', 'body');
        const preset = element('div', 'preset');
        const editor = book.editor.create('Workbench program');
        editor.area.readOnly = true;
        const message = book.render.message();
        const viewer = book.viewer.create();
        body.append(preset, editor.element, message.element, ...viewer.element);
        widget.replaceChildren(bar, body);

        let current;
        let ticket = 0;
        let busy = false;
        const settle = () => {
            busy = false;
            run.removeAttribute('aria-disabled');
        };
        const point = () => {
            lightbox.href = book.share.link({ ...current, source: editor.value });
        };

        const load = item => {
            ticket++;
            settle();
            current = item;
            editor.value = item.source;
            preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === item.name)));
            message.say();
            point();
            viewer.show(item.result, item.target);
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
            message.wait('Running in WebAssembly…');
            try {
                const result = await book.engine.explore(current, source);
                if (mine !== ticket) return;
                current = { ...current, name: '', source, result };
                preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', 'false'));
                message.say();
                viewer.show(result, current.target);
            } catch (error) {
                if (mine === ticket) message.say(book.editor.locate(editor.area, error), 'error');
            } finally {
                if (mine === ticket) settle();
            }
        };
        run.addEventListener('click', execute);
        editor.area.addEventListener('input', point);
        editor.area.addEventListener('keydown', event => {
            if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
            event.preventDefault();
            execute();
        });
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
                viewer.filter(text);
                reveal();
            },
            reveal,
        };
        preset.querySelector('button')?.click();
    };

    document.querySelectorAll('.workbench[data-preset]').forEach(enhance);
})();
