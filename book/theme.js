(() => {
    'use strict';
    const root = document.documentElement;
    const button = document.getElementById('theme');
    const choice = ['system', 'light', 'dark'];

    const paint = value => {
        if (value === 'system') delete root.dataset.theme;
        else root.dataset.theme = value;
        button.textContent = `Theme · ${value[0].toUpperCase()}${value.slice(1)}`;
        button.dataset.choice = value;
    };

    const stored = (() => {
        try {
            return localStorage.getItem('photonic-book-theme');
        } catch {
            return undefined;
        }
    })();
    paint(choice.includes(stored) ? stored : 'system');
    button.addEventListener('click', () => {
        const next = choice[(choice.indexOf(button.dataset.choice) + 1) % choice.length];
        paint(next);
        try {
            localStorage.setItem('photonic-book-theme', next);
        } catch {}
    });
})();
