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
        let marking = new Map();
        const paint = () => {
            book.syntax.highlight(shade, `${area.value}\n`);
            book.syntax.mark(shade, marking);
            shade.scrollLeft = area.scrollLeft;
        };
        area.addEventListener('input', paint);
        area.addEventListener('scroll', () => { shade.scrollLeft = area.scrollLeft; });
        return {
            element: box,
            area,
            mark: key => {
                marking = key;
                book.syntax.mark(shade, key);
            },
            get value() {
                return area.value;
            },
            set value(text) {
                area.value = text;
                paint();
            },
        };
    };

    const goal = (label, height) => {
        const field = element('label', 'field');
        const area = element('textarea');
        area.spellcheck = false;
        area.rows = height;
        field.append(label, area);
        const offset = index => {
            let start = 0;
            let seen = -1;
            for (const line of area.value.split('\n')) {
                if (line.trim()) seen++;
                if (line.trim() && seen === index) return start + line.length - line.trimStart().length;
                start += line.length + 1;
            }
            return undefined;
        };
        return {
            element: field,
            area,
            offset,
            get value() {
                return area.value.split('\n').map(line => line.trim()).filter(Boolean);
            },
            set value(target) {
                area.value = target.join('\n');
            },
        };
    };

    const describe = error => {
        const place = error.detail?.span;
        const target = error.detail?.target;
        const text = place ? `${error.message} (at character ${place.offset + 1})` : error.message;
        return target === undefined ? text : `Target ${target + 1}: ${text}`;
    };

    const submit = (widget, editor, field) => ({
        widget,
        editor,
        field,
        focus: document.activeElement,
        source: editor.area.value,
        target: field?.area.value,
    });

    const settled = submission => {
        const active = document.activeElement;
        if (active !== submission.focus) return false;
        if (active !== document.body && !submission.widget.contains(active)) return false;
        return submission.editor.area.value === submission.source && submission.field?.area.value === submission.target;
    };

    const locate = (error, submission) => {
        const place = error.detail?.span;
        const target = error.detail?.target;
        const area = target === undefined ? submission.editor.area : submission.field?.area;
        const start = target === undefined ? 0 : submission.field?.offset(target);
        if (place && area && start !== undefined && settled(submission)) {
            area.focus();
            area.setSelectionRange(start + place.offset, start + place.offset + Math.max(1, place.length));
        }
        return describe(error);
    };

    book.editor = { create, goal, describe, submit, locate };
})();
