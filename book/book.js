(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const find = name => document.getElementById(name);
    const root = document.documentElement;

    find('menu').addEventListener('click', () => {
        const open = find('rail').dataset.open !== 'true';
        find('rail').dataset.open = String(open);
        find('menu').setAttribute('aria-expanded', String(open));
    });
    find('outline').addEventListener('click', event => {
        if (!event.target.closest('a')) return;
        find('rail').dataset.open = 'false';
        find('menu').setAttribute('aria-expanded', 'false');
    });

    const link = new Map([...find('outline').querySelectorAll('a')].map(node => [node.hash.slice(1), node]));
    const chapter = [...document.querySelectorAll('.chapter')].filter(node => link.has(node.id));
    let scheduled = false;
    const locate = () => {
        scheduled = false;
        const height = root.scrollHeight - innerHeight;
        find('meter').style.width = `${height > 0 ? Math.min(100, scrollY / height * 100) : 100}%`;
        const current = chapter.filter(node => node.getBoundingClientRect().top < innerHeight * 0.3).at(-1);
        link.forEach((node, anchor) => {
            if (current?.id === anchor) node.setAttribute('aria-current', 'location');
            else node.removeAttribute('aria-current');
        });
    };
    addEventListener('scroll', () => {
        if (scheduled) return;
        scheduled = true;
        requestAnimationFrame(locate);
    }, { passive: true });
    addEventListener('resize', locate);
    locate();

    document.querySelectorAll('pre.listing').forEach(pre => {
        const button = book.render.element('button', 'copy', 'Copy');
        button.type = 'button';
        button.addEventListener('click', async () => {
            const text = [...pre.childNodes].filter(node => node !== button).map(node => node.textContent).join('');
            try {
                await navigator.clipboard.writeText(text.trim());
                button.textContent = 'Copied';
            } catch {
                getSelection().selectAllChildren(pre);
                button.textContent = 'Selected';
            }
            setTimeout(() => { button.textContent = 'Copy'; }, 1600);
        });
        pre.append(button);
    });

    document.querySelectorAll('pre.code').forEach(pre => {
        if (!pre.firstElementChild && !pre.closest('figure.example')) book.syntax.highlight(pre);
    });
})();
