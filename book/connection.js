(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, series, count } = book.render;

    const tag = (host, letter, prefix) => {
        host.querySelectorAll('.atom').forEach(node => {
            const found = letter.get(node.textContent);
            if (found === undefined) delete node.dataset.atom;
            else node.dataset.atom = `${prefix}${found}`;
        });
    };

    const enhance = widget => {
        const preset = JSON.parse(widget.dataset.preset);
        const bar = element('div', 'bar');
        const badge = element('span', 'badge', 'recorded run');
        bar.append(element('span', 'title', 'Connections'), badge);
        const body = element('div', 'body');
        const choice = book.render.preset(preset.map(entry => entry.name), index => choose(preset[index]));
        const program = element('div', 'program');
        const message = book.render.message();
        const verdict = element('p', 'verdict');
        const output = element('div', 'output');
        body.append(choice.element, program, message.element, verdict, output);
        widget.replaceChildren(bar, body);
        let field = [];
        let editor = [];
        let result;
        let ticket = 0;
        let timer;
        let lit = '';

        const light = key => {
            const signature = [...key].sort().join(' ');
            if (signature === lit) return;
            lit = signature;
            widget.querySelectorAll('[data-atom]').forEach(node => {
                if (key.has(node.dataset.atom)) node.dataset.lit = '';
                else delete node.dataset.lit;
            });
        };

        const hover = (node, key) => {
            node.addEventListener('pointerenter', () => light(key));
            node.addEventListener('pointerleave', () => light(new Set()));
            node.addEventListener('focus', () => light(key));
            node.addEventListener('blur', () => light(new Set()));
        };

        const paint = () => {
            result?.shape.forEach((shape, index) => shape.member.forEach((member, position) => {
                tag(editor[member].shade, new Map(shape.atom.map(row => [row.name[position], row.letter])), `${index}:`);
            }));
        };

        const read = (shape, index, code, note, reading) => {
            const name = new Map(shape.atom.map(row => [row.letter, reading < 0 ? row.letter : row.name[reading]]));
            const rename = text => book.syntax.scan(text).map(piece => piece.kind === 'concept' ? name.get(piece.text) : piece.text).join('');
            book.syntax.highlight(code, [...shape.initial, ...shape.rule].map(rename).join(',\n'));
            tag(code, new Map([...name].map(([letter, value]) => [value, letter])), `${index}:`);
            const shared = shape.member.length > 1;
            note.replaceChildren(element('span', undefined, shape.size === '1'
                ? `Only the identity leaves this shape unchanged${shared ? ', so this dictionary is the only one' : ''}.`
                : `${shape.size} renamings leave this shape unchanged${shared ? ', and each gives another dictionary' : ''}.`));
            if (shape.symmetry.length) note.append(element('span', undefined, 'Among them:'));
            for (const permutation of shape.symmetry) {
                const button = element('button', undefined, permutation.map(cycle => `(${cycle.map(letter => name.get(letter)).join(' ')})`).join(''));
                button.type = 'button';
                hover(button, new Set(permutation.flat().map(letter => `${index}:${letter}`)));
                note.append(button);
            }
            if (shape.block.length) note.append(element('span', undefined, `Atoms joined by a dot always appear together and exchange freely: ${shape.block.map(block => block.map(letter => name.get(letter)).join('.')).join(', ')}.`));
        };

        const panel = (shape, index) => {
            const member = shape.member.map(position => field[position]);
            const box = element('section', 'panel');
            const head = element('header');
            const option = element('div', 'option');
            head.append(element('h4', undefined, `The shape of ${series(member)}`), option);
            const heading = element('tr');
            ['Shape', ...member].forEach(text => heading.append(element('th', undefined, text)));
            const header = element('thead');
            header.append(heading);
            const row = element('tbody');
            for (const atom of shape.atom) {
                const line = element('tr');
                line.dataset.atom = `${index}:${atom.letter}`;
                line.tabIndex = 0;
                line.append(element('td', undefined, atom.letter), ...atom.name.map(name => element('td', undefined, name)));
                hover(line, new Set([line.dataset.atom]));
                row.append(line);
            }
            const grid = element('table');
            grid.append(header, row);
            const table = element('div', 'table');
            table.append(grid);
            const code = element('pre', 'code');
            code.addEventListener('pointerover', event => {
                const atom = event.target.closest('.atom[data-atom]');
                light(new Set(atom ? [atom.dataset.atom] : []));
            });
            code.addEventListener('pointerleave', () => light(new Set()));
            const note = element('p', 'summary');
            const inner = element('div', 'body');
            inner.append(table, code, note);
            box.append(head, inner);
            option.append('Read as');
            ['Shape', ...member].forEach((text, position) => {
                const button = element('button', undefined, text);
                button.type = 'button';
                button.addEventListener('click', () => {
                    option.querySelectorAll('button').forEach(other => other.setAttribute('aria-pressed', String(other === button)));
                    read(shape, index, code, note, position - 1);
                });
                option.append(button);
            });
            option.querySelector('button').click();
            return box;
        };

        const draw = value => {
            result = value;
            const [only] = value.shape;
            verdict.textContent = value.shape.length === 1 && only.member.length > 1
                ? `One shape: ${series(field)} differ only in the names of their atoms.`
                : `${count(value.shape.length, 'shape')}: ${value.shape.map(shape => {
                    const member = shape.member.map(position => field[position]);
                    return member.length > 1 ? `${series(member)} share one` : `${member[0]} has its own`;
                }).join('; ')}.`;
            output.replaceChildren(...value.shape.map(panel));
            lit = '';
            paint();
        };

        const clear = () => {
            result = undefined;
            verdict.textContent = '';
            output.replaceChildren();
        };

        const run = async () => {
            const mine = ++ticket;
            message.wait('Comparing in WebAssembly…');
            try {
                const value = await book.engine.send('compare', { program: editor.map(item => item.value) });
                if (mine !== ticket) return;
                message.say();
                draw(value);
            } catch (error) {
                if (mine !== ticket) return;
                clear();
                const place = error.detail?.program;
                const text = book.editor.describe(error);
                message.say(place === undefined ? text : `${field[place]}: ${text}`, 'error');
            }
        };

        const choose = entry => {
            ticket++;
            clearTimeout(timer);
            choice.press(entry.name);
            const record = book.record?.connection?.[entry.name];
            field = entry.program.map(item => item.field);
            editor = entry.program.map((item, position) => {
                const pane = book.editor.create(`Edit the ${item.field} program`);
                pane.value = record?.program[position]?.source ?? item.source ?? '';
                pane.area.readOnly = book.engine.state !== 'live';
                pane.area.addEventListener('input', () => {
                    clearTimeout(timer);
                    timer = setTimeout(run, 260);
                });
                return pane;
            });
            program.replaceChildren(...editor.map((pane, position) => {
                const box = element('section', 'panel');
                const head = element('header');
                head.append(element('h4', undefined, field[position]));
                box.append(head, pane.element);
                return box;
            }));
            if (record) {
                message.say();
                draw(record.result);
                return;
            }
            clear();
            if (book.engine.state === 'live') run();
            else message.say('No recorded comparison. Regenerate the records with bazel run -c opt //book:record.', 'error');
        };

        book.engine.watch(state => {
            badge.textContent = state === 'live' ? 'live' : 'recorded run';
            editor.forEach(pane => { pane.area.readOnly = state !== 'live'; });
        });
        choose(preset[0]);
    };

    document.querySelectorAll('.connection[data-preset]').forEach(enhance);
})();
