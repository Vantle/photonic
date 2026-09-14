import initialize, { multiply } from '../toolchain/browser/module/runtime.js';

(async () => {
    const root = document.getElementById('product');
    if (!root) return;
    const find = name => document.getElementById(`product-${name}`);
    const render = () => {
        const left = Number(find('left').value);
        const right = Number(find('right').value);
        const result = JSON.parse(multiply(left, right));
        if (result.error) { find('equation').textContent = result.error.message; return; }
        const node = [];
        const edge = [];
        for (let i = 0; i < 2; i++) {
            for (let j = 0; j < 2; j++) {
                const pair = result.pair[i * 2 + j];
                const a = pair.left;
                const b = pair.right;
                const digit = pair.digit;
                const carry = pair.carry;
                const x = 95 + (i * 2 + j) * 180;
                for (const [position, value, kind] of [[i + j, digit, 'digit'], [i + j + 1, carry, 'carry']]) {
                    const target = 95 + position * 180;
                    edge.push(`<path class="product-${kind}" d="M ${x} 112 C ${x} 180 ${target} 180 ${target} 242"><title>${kind} ${value} contributes to position ${position}</title></path>`);
                }
                node.push(`<g><rect x="${x - 76}" y="32" width="152" height="80" rx="12"/><text x="${x}" y="54">a${i} × b${j}</text><text x="${x}" y="78">${a} × ${b} = ${digit + 3 * carry}</text><text x="${x}" y="99">digit ${digit} · carry ${carry}</text></g>`);
            }
        }
        const row = result.column.map((column, position) => {
            const x = 95 + position * 180;
            node.push(`<g><rect x="${x - 76}" y="242" width="152" height="65" rx="12"/><text x="${x}" y="266">position ${position}</text><text x="${x}" y="289">${column.contribution} contributions</text></g>`);
            return `<tr><th scope="row">${position} · weight ${3 ** position}</th><td>${column.contribution}</td><td>${column.incoming}</td><td>${column.digit}</td><td>${column.carry}</td></tr>`;
        });
        const decimal = [...result.ternary].reduce((value, digit) => value * 3 + Number(digit), 0);
        find('equation').textContent = `${left.toString(3)} (base 3) × ${right.toString(3)} (base 3) = ${result.ternary} (base 3) · ${left} × ${right} = ${decimal}`;
        find('graph').innerHTML = `<svg viewBox="0 0 730 328" role="img" aria-labelledby="product-title product-description"><title id="product-title">Four independent digit products</title><desc id="product-description">Each digit pair sends its low digit to position i plus j and its carry to the next position. The table below shows every column contribution.</desc>${edge.join('')}${node.join('')}</svg>`;
        find('column').innerHTML = row.join('');
        find('result').textContent = `Read the result from position 3 down to 0: ${result.column.map(value => value.digit).reverse().join('')} (base 3). Leading zeroes may be omitted when displaying the number.`;
    };
    if (location.protocol === 'file:') {
        find('equation').textContent = 'Run bazel run -c opt //toolchain/browser:serve to load this Wasm diagram.';
        return;
    }
    try { await initialize(); } catch {
        find('equation').textContent = 'Could not load WebAssembly. Start the local server and reload.';
        return;
    }
    for (const name of ['left', 'right']) {
        for (let value = 0; value < 9; value++) {
            const option = document.createElement('option');
            option.value = value;
            option.textContent = `${value.toString(3).padStart(2, '0')} (base 3) (${value})`;
            option.selected = value === (name === 'left' ? 5 : 7);
            find(name).append(option);
        }
        find(name).addEventListener('change', render);
    }
    render();
})();
