(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const status = document.getElementById('status');

    const text = { live: 'Live engine', unknown: 'Loading engine…', recorded: 'Recorded runs', stale: 'Reload the page' };
    const title = {
        live: 'Programs run in the Rust runtime compiled to WebAssembly.',
        unknown: 'Showing runs recorded by the Rust runtime while the engine loads.',
        recorded: 'Showing runs recorded by the Rust runtime. Open photonic.vantle.org or serve a checkout to run your own.',
        stale: 'This page and its engine come from different versions of the book. Reload the page to run your own programs.',
    };

    book.engine.watch(state => {
        status.dataset.live = String(state === 'live');
        status.textContent = text[state];
        status.title = title[state];
    });
})();
