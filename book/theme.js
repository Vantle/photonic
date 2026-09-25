(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const root = document.documentElement;
    const button = document.getElementById('theme');
    const choice = ['system', 'light', 'dark'];

    const paint = value => {
        if (value === 'system') delete root.dataset.theme;
        else root.dataset.theme = value;
        button.textContent = `Theme · ${value[0].toUpperCase()}${value.slice(1)}`;
        button.dataset.choice = value;
    };

    paint(root.dataset.theme ?? 'system');
    button.addEventListener('click', () => {
        const next = choice[(choice.indexOf(button.dataset.choice) + 1) % choice.length];
        paint(next);
        try {
            localStorage.setItem(book.storage.theme, next);
        } catch {}
    });
})();
