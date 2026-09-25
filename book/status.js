(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const status = document.getElementById('status');

    book.engine.watch(state => {
        status.dataset.live = String(state === 'live');
        status.textContent = state === 'live' ? 'Live engine' : state === 'unknown' ? 'Loading engine…' : 'Recorded runs';
        status.title = state === 'live'
            ? 'Programs run in the Rust runtime compiled to WebAssembly.'
            : 'Showing runs recorded by the Rust runtime. Open photonic.vantle.org or serve a checkout to run your own.';
    });
})();
