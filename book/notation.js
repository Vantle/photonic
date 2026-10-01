(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { series } = book.render;

    const basis = {
        circle: ['Postulate 3 and the figure', 'Describe a circle with any centre and radius. Where it meets the figure, Euclid reads from his drawing; the rule states when.'],
        produce: ['Postulates 2 and 3 and the figure', 'Produce a straight line, and cut off a length on it with a circle that meets it.'],
        line: ['Postulate 1', 'One straight line joins two points, so any two of its points name it.'],
        'postulate 5': ['Postulate 5', 'Straight lines making the interior angles on one side less than two right angles meet on that side.'],
        'common 1': ['Common notion 1', 'Things equal to the same thing are equal to one another.'],
        'common 2': ['Common notion 2', 'Equals added to equals give equal wholes.'],
        'common 3': ['Common notion 3', 'Equals taken from equals leave equal remainders.'],
        'common 5': ['Common notion 5', 'The whole is greater than the part.'],
        substitute: ['Common notions 1 and 2', 'Equals replace equals inside a sum.'],
        ray: ['Common notion 4', 'Things that coincide are equal: an angle is the same whichever point of its arm names it.'],
        superposition: ['Superposition', 'A triangle moved so that a side falls on an equal straight line keeps its sides, angles and area.'],
        right: ['Definition 10', 'Equal adjacent angles on a straight line are right angles.'],
        definition: ['Definition 22', 'A quadrilateral with equal sides and right angles is a square.'],
        parallelogram: ['Parallelogram', 'A quadrilateral whose opposite sides are parallel, each diameter between the other two corners.'],
        parallel: ['Definition 23', 'Straight lines that meet on neither side are parallel.'],
        cross: ['The figure', 'Where the lines of the figure cross, or how it divides into parts.'],
        suppose: ['Supposition', 'Opens a case whose facts hold inside its scope alone.'],
        trichotomy: ['Trichotomy', 'Of two magnitudes one is greater, or the other is, or they are equal; refuting two leaves the third.'],
        coincide: ['Coincidence', 'Every other arrangement is impossible, so the figures coincide.'],
        cases: ['Every case', 'The claim holds in every arrangement of the figure.'],
    };

    // A rule is justified by a postulate, common notion or definition, by the conclusion, by the end of
    // a case, or else by the proposition it names.
    const cite = why => why === 'theorem' || why === 'close' || Object.hasOwn(basis, why) ? undefined : why;

    const split = text => {
        const piece = [];
        let depth = 0;
        let start = 0;
        for (let index = 0; index < text.length; index++) {
            if (text[index] === '(' || text[index] === '[') depth++;
            if (text[index] === ')' || text[index] === ']') depth--;
            if (text[index] !== ',' || depth) continue;
            piece.push(text.slice(start, index).trim());
            start = index + 1;
        }
        return [...piece, text.slice(start).trim()].filter(Boolean);
    };

    const close = text => {
        let depth = 0;
        for (let index = 0; index < text.length; index++) {
            if (text[index] === '(' || text[index] === '[') depth++;
            if (text[index] !== ')' && text[index] !== ']') continue;
            depth--;
            if (!depth) return index;
        }
        return -1;
    };

    const rule = text => {
        const end = close(text);
        const output = text.slice(end + 1).trim();
        const group = output.startsWith('(') && close(output) === output.length - 1;
        return { input: split(text.slice(1, end)), output: split(group ? output.slice(1, -1) : output) };
    };

    const term = text => {
        const piece = book.syntax.scan(text).filter(value => value.kind !== 'space');
        let at = 0;
        const unit = () => {
            const current = piece[at++];
            if (current.kind === 'concept') return { atom: current.text };
            let head = [];
            if (piece[at].kind === '[') {
                at++;
                head = chain();
                at++;
            }
            const body = piece[at].kind === ')' ? [] : chain();
            at++;
            return { head, body };
        };
        const chain = () => {
            const part = [unit()];
            while (piece[at]?.kind === '.') {
                at++;
                part.push(unit());
            }
            return part;
        };
        return chain();
    };

    const magnitude = (part, place) => {
        if (part.atom === 'Right') return '∟';
        if (part.atom === 'Excess') return '…';
        if (part.atom) return part.atom;
        const [kind, vertex] = part.head.map(value => value.atom);
        const letter = part.body.map(value => value.atom);
        if (kind === 'Line') return letter.join('');
        if (kind === 'Angle') return `∠${letter[0]}${vertex}${letter[1]}`;
        if (kind === 'Square') return `${letter.join('')}²`;
        if (kind === 'Sum') return part.body.map(value => magnitude(value, place)).join(' + ');
        if (kind !== 'Area') return letter.join('');
        if (!letter.every(name => place[name])) return `area ${letter.join('')}`;
        const order = book.figure.ring(letter, place);
        if (order.length === 3) return `△${order.join('')}`;
        return `${book.figure.parallelogram(order.map(name => place[name])) ? '▱' : 'area '}${order.join('')}`;
    };

    const excess = part => part.head?.[0]?.atom === 'Sum' && part.body.some(value => value.atom === 'Excess');

    const rest = part => {
        const kept = part.body.filter(value => value.atom !== 'Excess');
        return kept.length === 1 ? kept[0] : { head: part.head, body: kept };
    };

    // A whole equal to a part and some excess is the greater, so the reading names the order directly.
    const compare = (left, right, place) => {
        if (excess(right) && !excess(left)) return `${magnitude(left, place)} > ${magnitude(rest(right), place)}`;
        if (excess(left) && !excess(right)) return `${magnitude(right, place)} > ${magnitude(rest(left), place)}`;
        return `${magnitude(left, place)} = ${magnitude(right, place)}`;
    };

    const gloss = (fact, place) => {
        if (book.figure.token(fact)) return `a new point ${fact}`;
        const [head, first, second] = term(fact);
        const pair = part => series(part.body.map(value => value.atom));
        switch (head.atom) {
        case 'Equal': return compare(first, second, place);
        case 'Parallel': return `${magnitude(first, place)} ∥ ${magnitude(second, place)}`;
        case 'Across': return `${magnitude(first, place)} separates ${pair(second)}`;
        case 'Same': return `${pair(second)} lie on one side of ${magnitude(first, place)}`;
        case 'Parallelogram': return `▱${first.body[0].atom}${second.body[0].atom}${first.body[1].atom}${second.body[1].atom}`;
        case 'Absurd': return 'a contradiction';
        case 'Theorem': return 'the proposition holds';
        case 'Suppose': return `case ${first.atom}`;
        case 'Refuted': return `case ${first.atom} is impossible`;
        case 'Proved': return `the claim holds in case ${first.atom}`;
        case 'Shown': return 'the claim holds in this case';
        default: return '';
        }
    };

    book.notation = { basis, cite, split, rule, term, gloss };
})();
