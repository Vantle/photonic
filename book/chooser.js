(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, count } = book.render;

    let loading;
    const load = () => {
        loading ??= new Promise((resolve, reject) => {
            if (book.library) {
                resolve();
                return;
            }
            const script = element('script');
            script.src = 'book/library.js';
            script.addEventListener('load', () => resolve());
            script.addEventListener('error', () => {
                script.remove();
                loading = undefined;
                reject(new Error('The standard library did not load.'));
            });
            document.head.append(script);
        });
        return loading;
    };

    const has = name => Object.hasOwn(book.library ?? {}, name) || Object.hasOwn(book.record?.library ?? {}, name);

    const offer = () => [...new Set([...Object.keys(book.library ?? {}), ...Object.keys(book.record?.library ?? {})])].sort();

    const kib = size => `${Math.ceil(size / 1024)} KiB`;

    const create = ({ change, measure }) => {
        const box = element('details', 'chooser');
        const summary = element('summary');
        const chosen = element('code');
        summary.append(element('span', undefined, 'Libraries'), chosen);
        const catalog = element('div', 'catalog');
        const note = element('p');
        box.append(summary, catalog, note);
        let picked = new Set();
        let on = false;
        let failed = false;
        const value = () => offer().filter(name => picked.has(name));

        const describe = () => {
            const library = value();
            chosen.textContent = library.length ? library.map(name => name.replace(/^library\//, '')).join(', ') : 'none';
            delete note.dataset.tone;
            if (!library.length) {
                note.textContent = 'Each library loads the libraries it needs first.';
                return;
            }
            const size = measure();
            const file = count(book.engine.expand(library).length, 'file');
            if (size <= book.engine.capacity) {
                note.textContent = `Loads ${file}, dependencies included: ${kib(size)} with the program, of the 128 KiB a run can send.`;
                return;
            }
            note.textContent = `Loads ${file}, dependencies included: ${kib(size)} with the program, past the 128 KiB a run can send. Choose fewer libraries or shorten the program.`;
            note.dataset.tone = 'error';
        };

        const flip = (button, name) => {
            if (button.hasAttribute('aria-disabled')) return;
            if (picked.has(name)) picked.delete(name);
            else picked.add(name);
            button.setAttribute('aria-pressed', String(picked.has(name)));
            describe();
            change();
        };

        const paint = () => {
            const group = new Map();
            for (const name of offer()) {
                const part = name.split('/').at(-2);
                group.set(part, [...group.get(part) ?? [], name]);
            }
            catalog.replaceChildren(...[...group].map(([part, name]) => {
                const row = element('div', 'group');
                const member = element('div');
                row.append(element('span', undefined, part), member);
                for (const entry of name) {
                    const button = element('button', undefined, entry.split('/').at(-1));
                    button.type = 'button';
                    button.title = `${entry}.particle`;
                    button.setAttribute('aria-pressed', String(picked.has(entry)));
                    if (!on) button.setAttribute('aria-disabled', 'true');
                    button.addEventListener('click', () => flip(button, entry));
                    member.append(button);
                }
                return row;
            }));
            if (failed) catalog.append(element('p', undefined, 'This copy of the book holds only the libraries its examples load.'));
        };

        box.addEventListener('toggle', () => {
            if (!box.open) return;
            load().then(() => { failed = false; }, () => { failed = true; }).then(() => {
                paint();
                describe();
            });
        });

        return {
            element: box,
            describe,
            get value() {
                return value();
            },
            set value(library) {
                picked = new Set(library.filter(has));
                if (box.open) paint();
                describe();
            },
            enable: state => {
                on = state;
                catalog.querySelectorAll('button').forEach(button => {
                    if (on) button.removeAttribute('aria-disabled');
                    else button.setAttribute('aria-disabled', 'true');
                });
            },
        };
    };

    book.chooser = { create, load, has };
})();
