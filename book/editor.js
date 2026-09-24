(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const create = label => {
        const box = element('div', 'editor');
        const shade = element('pre');
        shade.setAttribute('aria-hidden', 'true');
        const area = element('textarea');
        area.spellcheck = false;
        area.wrap = 'off';
        area.setAttribute('aria-label', label);
        box.append(shade, area);
        const paint = () => {
            book.syntax.highlight(shade, `${area.value}\n`);
            shade.scrollLeft = area.scrollLeft;
        };
        area.addEventListener('input', paint);
        area.addEventListener('scroll', () => { shade.scrollLeft = area.scrollLeft; });
        return {
            element: box,
            area,
            get value() {
                return area.value;
            },
            set value(text) {
                area.value = text;
                paint();
            },
        };
    };

    const span = error => error.detail?.code === 'source' ? error.detail.span : undefined;

    const describe = error => {
        const place = span(error);
        return place ? `${error.message} (at character ${place.offset + 1})` : error.message;
    };

    const locate = (area, error) => {
        const place = span(error);
        if (place) {
            area.focus();
            area.setSelectionRange(place.offset, place.offset + Math.max(1, place.length));
        }
        return describe(error);
    };

    book.editor = { create, describe, locate };
})();
