(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;
    const storage = 'photonic-sandbox';

    const remembered = () => {
        try {
            const value = JSON.parse(localStorage.getItem(storage));
            return typeof value?.source === 'string' ? value : undefined;
        } catch {
            return undefined;
        }
    };

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
        const run = element('button', 'run', 'Run');
        run.type = 'button';
        run.hidden = true;
        run.title = 'Run (⌘ or Ctrl + Enter)';
        bar.append(element('span', 'title', 'Program'), badge, fresh, copy, run);
        const notice = element('p', 'notice', 'Write and run your own programs at photonic.vantle.org or in a served checkout. Here the examples show their recorded runs.');
        notice.hidden = true;
        const body = element('div', 'body');
        const preset = element('div', 'preset');
        preset.setAttribute('aria-label', 'Examples');
        const editor = book.editor.create('Sandbox program');
        const field = element('label', 'field');
        field.append('Target configurations, one per line');
        const goal = element('textarea');
        goal.spellcheck = false;
        goal.rows = 2;
        field.append(goal);
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
        body.append(preset, editor.element, field, option, message.element);
        card.append(bar, notice, body);
        const result = element('section', 'result');
        const viewer = book.viewer.create();
        result.append(...viewer.element);
        root.replaceChildren(card, result);

        const setting = () => ({
            source: editor.value,
            target: goal.value.split('\n').map(line => line.trim()).filter(Boolean),
            library: [...toggle].filter(([, button]) => button.getAttribute('aria-pressed') === 'true').map(([name]) => name),
            preserve: keep.checked,
        });
        const fill = value => {
            editor.value = value.source;
            goal.value = value.target.join('\n');
            toggle.forEach((button, name) => button.setAttribute('aria-pressed', String(value.library.includes(name))));
            keep.checked = value.preserve;
            const missing = value.library.filter(name => !toggle.has(name));
            if (missing.length) message.say(`The sandbox does not have ${missing.map(name => `${name}.particle`).join(', ')}.`, 'error');
        };
        const mark = name => preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === name)));
        const remember = () => {
            try {
                localStorage.setItem(storage, JSON.stringify(setting()));
            } catch {}
        };
        const address = () => history.replaceState(null, '', book.share.link(setting()));

        let ticket = 0;
        let busy = false;
        let pending = false;
        const settle = () => {
            busy = false;
            run.removeAttribute('aria-disabled');
        };
        const execute = async () => {
            if (run.hidden || busy) return;
            const mine = ++ticket;
            const value = setting();
            busy = true;
            run.setAttribute('aria-disabled', 'true');
            message.wait('Running in WebAssembly…');
            try {
                const outcome = await book.engine.explore(value, value.source);
                if (mine !== ticket) return;
                message.say();
                viewer.show(outcome, value.target);
                remember();
                address();
            } catch (error) {
                if (mine === ticket) message.say(book.editor.locate(editor.area, error), 'error');
            } finally {
                if (mine === ticket) settle();
            }
        };

        const pick = item => {
            const value = entry(item);
            if (!value) {
                message.say('This example has no recorded run. Regenerate the records with bazel run -c opt //book:record.', 'error');
                return;
            }
            ticket++;
            settle();
            message.say();
            fill(value);
            mark(item.name);
            viewer.show(value.result, value.target);
            remember();
            address();
        };
        sample.forEach(item => {
            const button = element('button', undefined, item.name);
            button.type = 'button';
            button.addEventListener('click', () => pick(item));
            preset.append(button);
        });

        let timer;
        const changed = () => {
            mark();
            clearTimeout(timer);
            timer = setTimeout(remember, 300);
        };
        for (const area of [editor.area, goal]) {
            area.addEventListener('input', changed);
            area.addEventListener('keydown', event => {
                if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
                event.preventDefault();
                execute();
            });
        }
        keep.addEventListener('change', changed);
        run.addEventListener('click', execute);
        fresh.addEventListener('click', () => {
            ticket++;
            settle();
            message.say();
            fill({ source: '', target: [], library: [], preserve: false });
            mark();
            viewer.blank('Write a program, then press Run.');
            remember();
            history.replaceState(null, '', 'sandbox.html');
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
                mark(known.name);
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
            goal.readOnly = !on;
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

    document.querySelectorAll('.sandbox[data-preset]').forEach(enhance);
})();
