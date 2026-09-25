(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;
    const storage = 'photonic-lightbox';

    const text = value => typeof value === 'string';

    const valid = value => text(value?.source)
        && Array.isArray(value.target) && value.target.every(text)
        && Array.isArray(value.library) && value.library.every(text)
        && typeof value.preserve === 'boolean';

    const remembered = () => {
        try {
            const value = JSON.parse(localStorage.getItem(storage));
            return valid(value) ? value : undefined;
        } catch {
            return undefined;
        }
    };

    const advice = (outcome, value) => value.preserve || !outcome.verdict.some(verdict => verdict.outcome === 'unreachable')
        ? undefined
        : 'An exact target lists the program’s live rules too. To compare data alone, tick “Targets keep the program’s rules”.';

    const same = (left, right) => left.source === right.source
        && left.target.join('\n') === right.target.join('\n')
        && [...left.library].sort().join('\n') === [...right.library].sort().join('\n')
        && left.preserve === right.preserve;

    const enhance = root => {
        const sample = JSON.parse(root.dataset.preset);
        const entry = item => item.example ? book.record?.example?.[item.example] : book.record?.workbench?.[item.workbench];
        const card = element('section', 'widget');
        const bar = element('div', 'bar');
        const badge = element('span', 'badge', 'recorded runs');
        const fresh = element('button', undefined, 'New');
        fresh.type = 'button';
        const copy = element('button', undefined, 'Copy link');
        copy.type = 'button';
        const run = book.run.button();
        bar.append(element('span', 'title', 'Program'), badge, fresh, copy, run);
        const notice = element('p', 'notice', 'Write and run your own programs at photonic.vantle.org or in a served checkout. Here the examples show their recorded runs.');
        notice.hidden = true;
        const body = element('div', 'body');
        const choice = book.render.preset(sample.map(item => item.name), index => pick(sample[index]));
        choice.element.setAttribute('aria-label', 'Examples');
        const editor = book.editor.create('Lightbox program');
        const goal = book.editor.goal('Target configurations, one per line', 2);
        const option = element('div', 'option');
        option.append(element('span', undefined, 'Libraries'));
        const toggle = new Map(Object.keys(book.record?.library ?? {}).map(name => {
            const button = element('button', undefined, name.replace(/^library\//, ''));
            button.type = 'button';
            button.title = `${name}.particle`;
            button.setAttribute('aria-pressed', 'false');
            button.addEventListener('click', () => {
                if (button.hasAttribute('aria-disabled')) return;
                button.setAttribute('aria-pressed', String(button.getAttribute('aria-pressed') !== 'true'));
                changed();
            });
            option.append(button);
            return [name, button];
        }));
        const check = element('label', 'check');
        const keep = element('input');
        keep.type = 'checkbox';
        check.append(keep, 'Targets keep the program’s rules');
        option.append(check);
        const message = book.render.message();
        body.append(choice.element, editor.element, goal.element, option, message.element);
        card.append(bar, notice, body);
        const result = element('section', 'result');
        const viewer = book.viewer.create();
        result.append(...viewer.element);
        root.replaceChildren(card, result);

        const setting = () => ({
            source: editor.value,
            target: goal.value,
            library: [...toggle].filter(([, button]) => button.getAttribute('aria-pressed') === 'true').map(([name]) => name),
            preserve: keep.checked,
        });
        const fill = value => {
            editor.value = value.source;
            goal.value = value.target;
            toggle.forEach((button, name) => button.setAttribute('aria-pressed', String(value.library.includes(name))));
            keep.checked = value.preserve;
            const missing = value.library.filter(name => !toggle.has(name));
            if (missing.length) message.say(`The Lightbox does not have ${missing.map(name => `${name}.particle`).join(', ')}.`, 'error');
        };
        const remember = () => {
            try {
                localStorage.setItem(storage, JSON.stringify(setting()));
            } catch {}
        };
        const address = () => history.replaceState(null, '', book.share.link(setting()));

        const cycle = book.run.create({ trigger: run, message });
        let pending = false;
        const execute = () => {
            const value = setting();
            const submission = book.editor.submit(root, editor, goal);
            cycle.start(signal => book.engine.explore(value, value.source, value.target, signal), outcome => {
                message.say(advice(outcome, value));
                viewer.show(outcome, value.target);
                remember();
                address();
            }, error => message.say(book.editor.locate(error, submission), 'error'));
        };

        const pick = item => {
            const value = entry(item);
            if (!value) {
                message.say('This example has no recorded run. Regenerate the records with bazel run -c opt //book:record.', 'error');
                return;
            }
            cycle.cancel();
            message.say();
            fill(value);
            choice.press(item.name);
            viewer.show(value.result, value.target);
            remember();
            address();
        };

        let timer;
        const changed = () => {
            choice.press();
            clearTimeout(timer);
            timer = setTimeout(remember, 300);
        };
        for (const area of [editor.area, goal.area]) {
            area.addEventListener('input', changed);
            book.run.shortcut(area, execute);
        }
        keep.addEventListener('change', changed);
        run.addEventListener('click', execute);
        fresh.addEventListener('click', () => {
            cycle.cancel();
            message.say();
            fill({ source: '', target: [], library: [], preserve: false });
            choice.press();
            viewer.blank('Write a program, then press Run.');
            remember();
            history.replaceState(null, '', 'lightbox.html');
            editor.area.focus();
        });
        copy.addEventListener('click', async () => {
            const link = new URL(book.share.link(setting()), location.href).href;
            try {
                await navigator.clipboard.writeText(link);
                copy.textContent = 'Copied';
            } catch {
                message.say(link);
                copy.textContent = 'Link below';
            }
            setTimeout(() => { copy.textContent = 'Copy link'; }, 1600);
        });

        const begin = () => {
            const start = book.share.read(location.search) ?? remembered();
            if (!start) {
                pick(sample[0]);
                return;
            }
            fill(start);
            const known = sample.find(item => entry(item) && same(entry(item), start));
            if (known) {
                choice.press(known.name);
                viewer.show(entry(known).result, entry(known).target);
                return;
            }
            viewer.blank('Run the program to draw its graph.');
            pending = true;
        };
        begin();

        book.engine.watch(state => {
            const on = state === 'live';
            run.hidden = !on;
            fresh.hidden = !on;
            notice.hidden = state !== 'recorded';
            editor.area.readOnly = !on;
            goal.area.readOnly = !on;
            keep.disabled = !on;
            toggle.forEach(button => {
                if (on) button.removeAttribute('aria-disabled');
                else button.setAttribute('aria-disabled', 'true');
            });
            badge.textContent = on ? 'live' : 'recorded runs';
            if (!on || !pending) return;
            pending = false;
            execute();
        });
    };

    document.querySelectorAll('.lightbox[data-preset]').forEach(enhance);
})();
