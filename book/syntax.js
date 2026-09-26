(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const space = new Set([' ', '\t', '\r', '\n', '\u000B', '\u000C']);
    const delimiter = new Set(['(', ')', '[', ']', '.', ',']);
    const scan = text => {
        const piece = [];
        let index = 0;
        while (index < text.length) {
            const character = text[index];
            if (space.has(character)) {
                let end = index;
                while (end < text.length && space.has(text[end])) end++;
                piece.push({ kind: 'space', text: text.slice(index, end) });
                index = end;
            } else if (delimiter.has(character)) {
                piece.push({ kind: character, text: character });
                index++;
            } else {
                let end = index;
                while (end < text.length && !space.has(text[end]) && !delimiter.has(text[end])) end++;
                piece.push({ kind: 'concept', text: text.slice(index, end) });
                index = end;
            }
        }
        return piece;
    };

    const atom = label => {
        const node = document.createElement('span');
        node.className = 'atom';
        node.textContent = label;
        return node;
    };

    const fragment = text => {
        const result = document.createDocumentFragment();
        for (const piece of scan(text)) {
            if (piece.kind === 'space') {
                result.append(piece.text);
                continue;
            }
            if (piece.kind === 'concept') {
                result.append(atom(piece.text));
                continue;
            }
            const node = document.createElement('span');
            node.className = piece.kind === '[' || piece.kind === ']' ? 'syntax context'
                : piece.kind === '(' || piece.kind === ')' ? 'syntax group' : 'syntax';
            node.textContent = piece.text;
            result.append(node);
        }
        return result;
    };

    const highlight = (element, text = element.textContent) => {
        element.replaceChildren(fragment(text));
    };

    const mark = (element, key) => element.querySelectorAll('.atom').forEach(node => {
        const value = key.get(node.textContent);
        if (value === undefined) delete node.dataset.atom;
        else node.dataset.atom = value;
    });

    book.syntax = { scan, fragment, highlight, mark };
})();
