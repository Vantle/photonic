(() => {
    'use strict';
    const book = globalThis.book ??= {};

    const link = setting => {
        const query = new URLSearchParams({ source: setting.source });
        setting.target.forEach(value => query.append('target', value));
        setting.library.forEach(value => query.append('library', value));
        if (setting.preserve) query.set('preserve', '');
        return `sandbox.html?${query}`;
    };

    const read = search => {
        const query = new URLSearchParams(search);
        if (!query.has('source')) return undefined;
        return {
            source: query.get('source'),
            target: query.getAll('target'),
            library: query.getAll('library'),
            preserve: query.has('preserve'),
        };
    };

    book.share = { link, read };
})();
