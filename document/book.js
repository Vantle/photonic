(() => {
    'use strict';
    const find = id => document.getElementById(id);
    const element = (tag, text, style) => {
        const node = document.createElement(tag);
        if (text !== undefined) node.textContent = text;
        if (style) node.className = style;
        return node;
    };
    const token = (text, style = '') => element('span', text, `token ${style}`);
    const names = ['0', '1', '2'];
    const theme = dark => {
        document.documentElement.dataset.theme = dark ? 'dark' : 'light';
        find('theme').setAttribute('aria-label', `Switch to ${dark ? 'light' : 'dark'} appearance`);
        find('theme').setAttribute('aria-pressed', String(dark));
    };
    try { theme(localStorage.getItem('photonic-book-theme') === 'dark'); } catch { theme(false); }
    find('theme').addEventListener('click', () => {
        const dark = document.documentElement.dataset.theme !== 'dark';
        theme(dark);
        try { localStorage.setItem('photonic-book-theme', dark ? 'dark' : 'light'); } catch {}
    });
    find('print').addEventListener('click', () => window.print());
    const reveal = () => {
        const target = document.getElementById(decodeURIComponent(location.hash.slice(1)));
        if (!target) return;
        for (let parent = target; parent; parent = parent.parentElement) {
            if (parent.tagName === 'DETAILS') parent.open = true;
        }
        target.scrollIntoView();
    };
    window.addEventListener('hashchange', reveal);
    if (location.hash) reveal();
    let folded = [];
    window.addEventListener('beforeprint', () => {
        folded = [...document.querySelectorAll('details:not([open])')];
        folded.forEach(value => { value.open = true; });
    });
    window.addEventListener('afterprint', () => {
        folded.forEach(value => { value.open = false; });
        folded = [];
    });

    const chapter = [...document.querySelectorAll('.chapter')];
    const link = [...find('contents').querySelectorAll('a')];
    const position = () => {
        const height = document.documentElement.scrollHeight - innerHeight;
        find('progress').style.width = `${height > 0 ? Math.min(100, scrollY / height * 100) : 100}%`;
        const current = chapter.filter(node => node.getBoundingClientRect().top < innerHeight * .35).at(-1);
        link.forEach(node => current && node.hash === `#${current.id}` ? node.setAttribute('aria-current', 'location') : node.removeAttribute('aria-current'));
        find('position').textContent = current ? `Chapter ${String(chapter.indexOf(current) + 1).padStart(2, '0')} of ${chapter.length}` : 'Begin anywhere. Read in order.';
    };
    let scheduled = false;
    window.addEventListener('scroll', () => {
        if (scheduled) return;
        scheduled = true;
        requestAnimationFrame(() => { position(); scheduled = false; });
    }, { passive: true });
    window.addEventListener('resize', position);
    find('search').addEventListener('input', event => {
        const query = event.target.value.trim().toLowerCase();
        let count = 0;
        link.forEach((node, index) => {
            const match = !query || `${chapter[index].dataset.title} ${chapter[index].textContent}`.toLowerCase().includes(query);
            node.hidden = !match;
            if (match) count++;
        });
        find('search-status').textContent = query ? `${count} matching chapter${count === 1 ? '' : 's'}` : '';
    });
    const remainder = () => {
        const count = Number(find('remainder').value);
        find('remainder-count').textContent = count;
        for (const [id, label] of [['remainder-before', 'A'], ['remainder-after', 'B']]) {
            find(id).replaceChildren(token(label, 'accent'), ...Array.from({ length: count }, () => token('X')));
        }
    };
    find('remainder').addEventListener('input', remainder);
    remainder();
    const grammar = [
        ['Concept: A', 'One concept token. Its program-defined meaning is separate from its spelling.'],
        ['Concept: A → Dot → Concept: B', 'The dot is an explicit source item between the two concepts. Arrows here show reading order, not source punctuation.'],
        ['Concept: A → Comma → Whitespace → Concept: B', 'The comma and the intervening space are distinct source items.'],
        ['Group (\n    Concept: A → Dot → Concept: B\n)', 'Parentheses enclose the inner items as a group. This display describes structure, not an expansion.'],
        ['Source context [ Concept: A ] → Whitespace → Concept: B', 'Brackets delimit the source context. Lowering interprets this complete expression as a rule.']
    ];
    const structure = () => {
        const value = grammar[find('grammar-select').selectedIndex];
        find('grammar-output').textContent = value[0];
        find('grammar-description').textContent = value[1];
    };
    find('grammar-select').addEventListener('change', structure);
    structure();
    const lowering = [
        ['A.B, A.C', 'Parentheses share the A prefix across two alternatives. They introduce no private container.'],
        ['A.C, A.D, B.C, B.D', 'Dot composition distributes across the alternatives. The expanded initial configuration contains four coherences.'],
        ['Field.Pack.Position.0, Field.Pack.Value.2', 'This is only flat expansion into two coherences. It does not invoke the library packing protocol or preserve position and value as fields.'],
        ['Invoke.Field.Pack.([Position] 0).([Value] 2)', 'Lowering preserves this request. Load //library/field:pack first; its ordinary rules and the invoke protocol can then produce ([0] 2). This display does not execute those rules.'],
        ['One empty coherence', 'The empty particle is present. It differs from an empty configuration with no coherence.'],
        ['Consume A; produce no coherences', 'There is no output particle to receive a remainder. Before another declaration, write a comma to end this empty-output rule.'],
        ['Consume A; produce one empty output particle', 'An output coherence exists, and unmatched remainder transfers into it. A.X can therefore become X.']
    ];
    const lower = () => {
        const value = lowering[find('lowering-select').selectedIndex];
        find('lowering-output').textContent = value[0];
        find('lowering-description').textContent = value[1];
    };
    find('lowering-select').addEventListener('change', lower);
    lower();
    const identity = {
        shared: [['B.X₁', 'C.X₁'], 'D.X₁', 'Both coherences inherited X₁ from one introduction. Reunion keeps one X₁.'],
        independent: [['B.X₁', 'C.X₂'], 'D.X₁.X₂', 'Equal X labels were introduced independently. Reunion retains both occurrences. Anonymous subscripts here explain identity; they are not required source syntax.'],
        evolved: [['B.Y₂', 'C.Z₃'], 'D.Y₂.Z₃', 'The two coherences independently computed fresh Y and Z results from their inherited X. Those results are independently usable on reunion.']
    };
    document.querySelectorAll('[data-identity]').forEach(button => button.addEventListener('click', () => showIdentity(button.dataset.identity)));
    function showIdentity(key) {
        const [before, after, description] = identity[key];
        const left = element('div');
        left.append(element('span', 'JOINT INPUT', 'micro'));
        before.forEach(value => left.append(element('pre', value)));
        const right = element('div');
        right.append(element('span', 'OUTPUT', 'micro'), element('pre', after));
        find('identity-diagram').replaceChildren(left, element('div', '[B, C] D →', 'rule-arrow'), right);
        find('identity-explanation').textContent = description;
        document.querySelectorAll('[data-identity]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.identity === key)));
    }
    showIdentity('shared');
    const inference = [
        ['The concrete source', 'Not.True', 'Not and True coexist. The scoped Not rule asks for Boolean evidence.'],
        ['Evidence reveals an abstraction', 'True ⇒ Boolean', 'The ordinary rule [True] Boolean establishes a derivation. Its flow records which concrete information supports Boolean.'],
        ['Apply at the source', 'Not.True ⇒ body(True)', 'The exact Not match is consumed. The body receives the concrete True witness rather than an accumulated Boolean description.'],
        ['The local body computes', 'True ⇒ False', 'The local [True] False rule produces False and returns. This is the same ordinary application machinery.']
    ];
    let inferenceIndex = 0;
    const showInference = () => {
        const [title, code, description] = inference[inferenceIndex];
        find('inference-view').replaceChildren(element('h4', title), element('code', code), element('p', description));
        find('inference-position').textContent = `${inferenceIndex + 1} / ${inference.length}`;
        find('inference-back').disabled = inferenceIndex === 0;
        find('inference-next').disabled = inferenceIndex === inference.length - 1;
    };
    find('inference-back').addEventListener('click', () => { inferenceIndex--; showInference(); });
    find('inference-next').addEventListener('click', () => { inferenceIndex++; showInference(); });
    showInference();
    let traceIndex = 0;
    let stateIndex = 0;
    let history = [];
    globalThis.native.forEach((fixture, index) => {
        const option = element('option', fixture.title || fixture.path);
        option.value = index;
        find('trace-select').append(option);
    });
    const showTrace = event => {
        const fixture = globalThis.native[traceIndex];
        const report = fixture.report;
        const state = report.state[stateIndex];
        find('trace-description').textContent = `${fixture.description} ${report.state.length} recorded configurations; ${report.event.length} events. ${report.closed ? 'Exploration closed.' : 'Recorded exploration prefix.'}`;
        find('trace-source').textContent = fixture.source;
        find('trace-position').textContent = `Configuration ${stateIndex} · ${state.status}`;
        find('trace-back').disabled = history.length === 0;
        globalThis.inspection.render(find('trace-world'), state, event, false, report.definition);
        const outgoing = report.event.filter(value => value.source === stateIndex);
        find('trace-event').replaceChildren();
        outgoing.forEach(value => {
            const button = element('button', `${value.rule}\n→ configuration ${value.target} · ${value.status}`);
            button.type = 'button';
            button.addEventListener('click', () => { history.push(stateIndex); stateIndex = value.target; showTrace(value); });
            find('trace-event').append(button);
        });
        if (!outgoing.length) find('trace-event').append(element('p', 'No outgoing event recorded here.'));
        find('trace-detail').textContent = event ? JSON.stringify({ rule: event.rule, footprint: event.footprint, exact: event.exact, read: event.read, evidence: event.evidence }, null, 2) : 'Select an event to inspect its binding and support.';
    };
    find('trace-select').addEventListener('change', event => { traceIndex = Number(event.target.value); stateIndex = 0; history = []; showTrace(); });
    find('trace-back').addEventListener('click', () => { stateIndex = history.pop(); showTrace(); });
    find('trace-reset').addEventListener('click', () => { stateIndex = 0; history = []; showTrace(); });
    showTrace();
    const unit = count => Array.from({ length: count }, () => 'Unit');
    const natural = () => {
        const left = Number(find('natural-left').value);
        const right = Number(find('natural-right').value);
        find('natural-left-value').textContent = left;
        find('natural-right-value').textContent = right;
        find('natural-input').replaceChildren();
        for (const [index, count] of [left, right].entries()) {
            const world = element('article', undefined, 'world');
            world.append(element('h4', `OPERAND ${index + 1}`));
            const row = element('div', undefined, 'token-row');
            row.append(token('Add', 'accent'), ...unit(count).map(() => token('U', index ? 'gold' : '')));
            world.append(row);
            find('natural-input').append(world);
        }
        find('natural-output').replaceChildren(...unit(left).map(() => token('U')), ...unit(right).map(() => token('U', 'gold')));
        if (left + right === 0) find('natural-output').append(element('code', '()'));
        find('natural-equation').textContent = `${left} + ${right} = ${left + right}`;
        find('natural-source').textContent = `${['Add', ...unit(left)].join('.')},\n${['Add', ...unit(right)].join('.')}\n[Add, Add] ()\n\nExact target: ${unit(left + right).join('.') || '()'} [Add, Add] ()`;
    };
    ['natural-left', 'natural-right'].forEach(id => find(id).addEventListener('input', natural));
    natural();
    const numeral = () => {
        const input = find('numeral');
        const value = Number(input.value);
        if (input.value === '' || !Number.isSafeInteger(value) || value < 0 || value > 1000000) {
            find('numeral-error').textContent = 'Enter a whole number from 0 through 1,000,000.';
            find('trit-strip').replaceChildren();
            find('numeral-expansion').textContent = '';
            return;
        }
        find('numeral-error').textContent = '';
        const digit = value.toString(3).split('').reverse();
        find('trit-strip').replaceChildren();
        for (let index = digit.length - 1; index >= 0; index--) {
            const cell = element('div', undefined, 'trit');
            cell.append(element('small', `3^${index}`), element('strong', digit[index]), element('small', `Position ${index}`));
            find('trit-strip').append(cell);
        }
        const term = digit.map((value, index) => Number(value) ? `${value} × 3^${index}` : '').filter(Boolean).reverse();
        find('numeral-expansion').textContent = `${value.toLocaleString()} = ${term.join(' + ') || '0'}`;
    };
    find('numeral').addEventListener('input', numeral);
    numeral();
    const digit = () => {
        const left = Number(find('digit-left').value);
        const right = Number(find('digit-right').value);
        const value = left * right;
        find('digit-equation').textContent = `${left} × ${right} = ${value % 3} + 3 × ${Math.floor(value / 3)}`;
        const ordered = [left, right].sort((a, b) => a - b);
        find('digit-rule').textContent = `[Function.Ternary.Multiply.${names[ordered[0]]}.${names[ordered[1]]}] Return.([Digit] ${names[value % 3]}).([Carry] ${names[Math.floor(value / 3)]})`;
    };
    ['digit-left', 'digit-right'].forEach(id => find(id).addEventListener('change', digit));
    digit();
    let streamIndex = 0;
    const showStream = () => {
        const event = globalThis.stream.event;
        find('stream-position').textContent = streamIndex === 0 ? 'Initial configuration' : `Event ${streamIndex} / ${event.length}`;
        find('stream-rule').textContent = streamIndex === 0 ? 'Seventeen.Function.Stream.Successor' : event[streamIndex - 1].rule;
        find('stream-back').disabled = streamIndex === 0;
        find('stream-next').disabled = streamIndex === event.length;
        const digit = event.slice(0, streamIndex).flatMap(value => names.filter(name => value.rule === `[([Write] ${name})] Next`));
        find('stream-output').replaceChildren(...digit.map(value => token(value, 'accent')));
        if (!digit.length) find('stream-output').append(element('span', 'No digit acknowledged yet.', 'caption'));
        find('stream-source').textContent = globalThis.stream.source;
    };
    find('stream-back').addEventListener('click', () => { streamIndex--; showStream(); });
    find('stream-next').addEventListener('click', () => { streamIndex++; showStream(); });
    showStream();
    let eventIndex = 0;
    const operation = () => {
        const fixture = globalThis.record.find(value => value.operation === find('operation').value);
        const symbol = { add: '+', subtract: '−', multiply: '×', divide: '÷' }[fixture.operation];
        find('operation-equation').textContent = `1500 ${symbol} 123 =`;
        find('operation-result').textContent = fixture.result.toLocaleString();
        find('operation-remainder').textContent = fixture.remainder === null ? '' : `remainder ${fixture.remainder}`;
        find('operation-event').textContent = fixture.events;
        find('operation-work').textContent = fixture.work.toLocaleString();
        find('operation-command').textContent = `bazel run -c opt //arithmetic:word -- \\\n  --operation ${fixture.operation} --left 1500 --right 123 \\\n  --expected ${fixture.result}${fixture.remainder === null ? '' : ` --remainder ${fixture.remainder}`}`;
        const event = fixture.event[eventIndex];
        find('operation-position').textContent = `Event ${eventIndex + 1} / ${fixture.events}`;
        find('operation-rule').textContent = `Configuration ${event.source} → ${event.target}\n${event.rule}`;
        find('operation-target').textContent = JSON.stringify(fixture.target, null, 2);
        find('operation-source').href = fixture.source;
        find('operation-back').disabled = eventIndex === 0;
        find('operation-next').disabled = eventIndex === fixture.events - 1;
    };
    find('operation').addEventListener('change', () => { eventIndex = 0; find('copy-status').textContent = ''; operation(); });
    find('operation-back').addEventListener('click', () => { eventIndex--; operation(); });
    find('operation-next').addEventListener('click', () => { eventIndex++; operation(); });
    find('command-copy').addEventListener('click', async () => {
        try {
            await navigator.clipboard.writeText(find('operation-command').textContent);
            find('copy-status').textContent = 'Command copied.';
        } catch {
            const range = document.createRange();
            range.selectNodeContents(find('operation-command'));
            const selection = window.getSelection();
            selection.removeAllRanges();
            selection.addRange(range);
            find('copy-status').textContent = 'Command selected. Press ⌘C or Ctrl+C to copy.';
        }
    });
    operation();
    position();
})();
