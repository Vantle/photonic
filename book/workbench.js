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
        const run = book.run.button();
        bar.append(element('span', 'title', 'Workbench'), badge, lightbox, run);
        const body = element('div', 'body');
        const choice = book.render.preset(sample.map(item => item.name), index => pick(sample[index]));
        const editor = book.editor.create('Workbench program');
        editor.area.readOnly = true;
        const message = book.render.message();
        const viewer = book.viewer.create();
        body.append(choice.element, editor.element, message.element, viewer.element);
        widget.replaceChildren(bar, body);

        const cycle = book.run.create({ trigger: run, message });
        let current;
        const point = () => {
            lightbox.href = book.share.link({ ...current, source: editor.value });
        };

        const load = item => {
            cycle.cancel();
            current = item;
            editor.value = item.source;
            choice.press(item.name);
            message.say();
            point();
            viewer.show(item.result, item.target);
        };

        const pick = item => {
            const entry = item.example ? book.record?.example?.[item.example] : book.record?.workbench?.[item.name];
            if (entry) load({ ...entry, name: item.name });
            else message.say('This preset has no recorded run. Regenerate the records with bazel run -c opt //book:record.', 'error');
        };

        const execute = () => {
            const source = editor.value;
            const submission = book.editor.submit(widget, editor);
            cycle.start(signal => book.engine.explore({ ...current, source }, signal), result => {
                current = { ...current, name: '', source, result };
                choice.press();
                viewer.show(result, current.target);
            }, error => message.say(book.editor.locate(error, submission), 'error'));
        };
        run.addEventListener('click', execute);
        editor.area.addEventListener('input', point);
        book.run.shortcut(editor.area, execute);
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
        if (sample.length) pick(sample[0]);
    };

    document.querySelectorAll('.workbench[data-preset]').forEach(enhance);
})();
