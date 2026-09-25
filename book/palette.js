(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const overlay = element('div', 'palette');
    overlay.hidden = true;
    const dialog = element('div');
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    dialog.setAttribute('aria-label', 'Search and commands');
    const input = element('input');
    input.spellcheck = false;
    input.autocomplete = 'off';
    input.placeholder = 'Filter by a pattern such as B.X, or jump to a chapter';
    input.setAttribute('role', 'combobox');
    input.setAttribute('aria-expanded', 'true');
    input.setAttribute('aria-controls', 'palette');
    const list = element('ul');
    list.id = 'palette';
    list.setAttribute('role', 'listbox');
    const footer = element('footer');
    footer.append(element('span', undefined, '↑↓ choose'), element('span', undefined, 'Enter run'), element('span', undefined, 'Esc close'));
    dialog.append(input, list, footer);
    overlay.append(dialog);
    document.body.append(overlay);

    let entry = [];
    let chosen = 0;
    let previous;

    const chapter = () => [...document.querySelectorAll('#outline a')].map(link => ({
        number: link.querySelector('span')?.textContent ?? '',
        title: [...link.childNodes].filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent).join('').trim(),
        anchor: link.hash.slice(1),
    }));

    const go = anchor => {
        close();
        document.getElementById(anchor)?.scrollIntoView();
        history.replaceState(null, '', `#${anchor}`);
    };

    const collect = () => {
        const query = input.value.trim();
        const lower = query.toLowerCase();
        const result = [];
        if (query && book.workbench) {
            result.push({ label: 'Filter the workbench by', code: query, hint: 'pattern', run: () => { close(); book.workbench.filter(query); } });
        }
        chapter().filter(item => !lower || item.title.toLowerCase().includes(lower)).forEach(item => {
            result.push({ label: `${item.number} · ${item.title}`, hint: 'chapter', run: () => go(item.anchor) });
        });
        const command = [
            { label: 'Open the workbench', hint: 'command', run: () => { close(); book.workbench?.reveal(); } },
            { label: 'Open the Lightbox', hint: 'command', run: () => { close(); location.href = 'lightbox.html'; } },
            { label: 'Clear the workbench filter', hint: 'command', run: () => { close(); book.workbench?.filter(''); } },
            { label: 'Switch theme', hint: 'command', run: () => { close(); document.getElementById('theme')?.click(); } },
        ];
        command.filter(item => !lower || item.label.toLowerCase().includes(lower)).forEach(item => result.push(item));
        return result;
    };

    const paint = () => {
        entry = collect();
        chosen = Math.min(chosen, Math.max(0, entry.length - 1));
        list.replaceChildren(...entry.map((item, index) => {
            const row = element('li');
            row.id = `palette${index}`;
            row.setAttribute('role', 'option');
            row.setAttribute('aria-selected', String(index === chosen));
            row.append(item.label);
            if (item.code) {
                const code = element('code');
                code.append(book.syntax.fragment(item.code));
                row.append(code);
            }
            row.append(element('span', undefined, item.hint));
            row.addEventListener('mousemove', () => {
                if (chosen === index) return;
                chosen = index;
                mark();
            });
            row.addEventListener('click', () => item.run());
            return row;
        }));
        if (!entry.length) list.append(element('li', undefined, 'Nothing matches.'));
        mark();
    };

    const mark = () => {
        list.querySelectorAll('[role="option"]').forEach((row, index) => row.setAttribute('aria-selected', String(index === chosen)));
        const row = list.querySelector(`#palette${chosen}`);
        if (!row) {
            input.removeAttribute('aria-activedescendant');
            return;
        }
        input.setAttribute('aria-activedescendant', row.id);
        row.scrollIntoView({ block: 'nearest' });
    };

    const shield = on => {
        for (const node of document.body.children) {
            if (node !== overlay) node.inert = on;
        }
    };

    const open = (text = '') => {
        previous = document.activeElement;
        overlay.hidden = false;
        shield(true);
        input.value = text;
        chosen = 0;
        paint();
        input.focus();
    };

    const close = () => {
        if (overlay.hidden) return;
        overlay.hidden = true;
        shield(false);
        previous?.focus?.({ preventScroll: true });
    };

    input.addEventListener('input', () => {
        chosen = 0;
        paint();
    });
    input.addEventListener('keydown', event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            if (!entry.length) return;
            chosen = (chosen + (event.key === 'ArrowDown' ? 1 : entry.length - 1)) % entry.length;
            mark();
        } else if (event.key === 'Enter') {
            event.preventDefault();
            entry[chosen]?.run();
        } else if (event.key === 'Tab') {
            event.preventDefault();
        }
    });
    overlay.addEventListener('click', event => {
        if (event.target === overlay) close();
    });
    document.addEventListener('keydown', event => {
        const typing = event.target.closest?.('input, textarea, select, [contenteditable="true"]');
        if (event.key === 'Escape' && !overlay.hidden) {
            event.preventDefault();
            close();
        } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
            event.preventDefault();
            if (overlay.hidden) open();
            else close();
        } else if (event.key === '/' && !typing && overlay.hidden) {
            event.preventDefault();
            open();
        }
    });
    document.getElementById('search')?.addEventListener('click', () => open());
})();
