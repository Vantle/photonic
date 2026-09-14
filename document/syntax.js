(() => {
    const highlight = (target, source, expression = false) => {
        const fragment = document.createDocumentFragment();
        const pattern = expression ? /\s+|[0-9]+|[+*/−×÷()\-]|[^\s0-9+*/−×÷()\-]+/gu : /\s+|[\[\]()⟨⟩.,]|[^\s\[\]()⟨⟩.,]+/gu;
        for (const token of source.match(pattern) ?? []) {
            if (/^\s+$/.test(token)) { fragment.append(document.createTextNode(token)); continue; }
            const span = document.createElement('span');
            span.className = /^[\[\]()⟨⟩]$/.test(token) ? 'syntax-group' : /^[.,]$/.test(token) ? 'syntax-separator' : /^[0-9]+$/.test(token) ? 'syntax-number' : /^[+*/−×÷-]$/.test(token) ? 'syntax-operator' : 'syntax-concept';
            span.textContent = token;
            fragment.append(span);
        }
        target.replaceChildren(fragment);
    };
    globalThis.syntax = { highlight };
})();
