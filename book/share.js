(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const format = '1';

    // The program travels in the fragment, which browsers never send to the host, because hosts
    // refuse long addresses; the leading number names the format, so a later one can read old links.
    const link = setting => {
        const query = new URLSearchParams({ source: setting.source });
        setting.target.forEach(value => query.append('target', value));
        setting.library.forEach(value => query.append('library', value));
        if (setting.preserve) query.set('preserve', '');
        return `lightbox.html#${format}&${query}`;
    };

    const decode = query => {
        if (!query.has('source')) return undefined;
        return {
            source: query.get('source'),
            target: query.getAll('target'),
            library: query.getAll('library'),
            preserve: query.has('preserve'),
        };
    };

    const read = hash => {
        const text = hash.replace(/^#/, '');
        const split = text.indexOf('&');
        if (split < 0 || text.slice(0, split) !== format) return undefined;
        return decode(new URLSearchParams(text.slice(split + 1)));
    };

    const legacy = search => decode(new URLSearchParams(search));

    book.share = { link, read, legacy };
})();
