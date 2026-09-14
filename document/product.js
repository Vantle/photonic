(() => {
    const root = document.getElementById('product');
    if (!root) return;
    const find = name => document.getElementById(`product-${name}`);
    const render = () => {
        const left = Number(find('left').value);
        const right = Number(find('right').value);
        const column = [0, 0, 0, 0];
        const node = [];
        const edge = [];
        for (let i = 0; i < 2; i++) {
            for (let j = 0; j < 2; j++) {
                const a = Math.floor(left / 3 ** i) % 3;
                const b = Math.floor(right / 3 ** j) % 3;
                const digit = a * b % 3;
                const carry = Math.floor(a * b / 3);
                column[i + j] += digit;
                column[i + j + 1] += carry;
                const x = 95 + (i * 2 + j) * 180;
                for (const [position, value, kind] of [[i + j, digit, 'digit'], [i + j + 1, carry, 'carry']]) {
                    const target = 95 + position * 180;
                    edge.push(`<path class="product-${kind}" d="M ${x} 112 C ${x} 180 ${target} 180 ${target} 242"><title>${kind} ${value} contributes to position ${position}</title></path>`);
                }
                node.push(`<g><rect x="${x - 76}" y="32" width="152" height="80" rx="12"/><text x="${x}" y="54">a${i} × b${j}</text><text x="${x}" y="78">${a} × ${b} = ${a * b}</text><text x="${x}" y="99">digit ${digit} · carry ${carry}</text></g>`);
            }
        }
        let carry = 0;
        const digit = [];
        const row = [];
        for (let position = 0; position < 4; position++) {
            const sum = column[position] + carry;
            digit.push(sum % 3);
            row.push(`<tr><th scope="row">${position} · weight ${3 ** position}</th><td>${column[position]}</td><td>${carry}</td><td>${sum % 3}</td><td>${Math.floor(sum / 3)}</td></tr>`);
            carry = Math.floor(sum / 3);
            const x = 95 + position * 180;
            node.push(`<g><rect x="${x - 76}" y="242" width="152" height="65" rx="12"/><text x="${x}" y="266">position ${position}</text><text x="${x}" y="289">${column[position]} contributions</text></g>`);
        }
        const result = digit.reverse().join('').replace(/^0+(?=.)/, '');
        find('equation').textContent = `${left.toString(3)}₃ × ${right.toString(3)}₃ = ${result}₃ · ${left} × ${right} = ${left * right}`;
        find('graph').innerHTML = `<svg viewBox="0 0 730 328" role="img" aria-labelledby="product-title product-description"><title id="product-title">Four independent digit products</title><desc id="product-description">Each digit pair sends its low digit to position i plus j and its carry to the next position. The table below shows every column contribution.</desc>${edge.join('')}${node.join('')}</svg>`;
        find('column').innerHTML = row.join('');
        find('result').textContent = `Read the result from position 3 down to 0: ${digit.join('')}₃. Leading zeroes may be omitted when displaying the number.`;
    };
    for (const name of ['left', 'right']) {
        for (let value = 0; value < 9; value++) {
            const option = document.createElement('option');
            option.value = value;
            option.textContent = `${value.toString(3).padStart(2, '0')}₃ (${value})`;
            option.selected = value === (name === 'left' ? 5 : 7);
            find(name).append(option);
        }
        find(name).addEventListener('change', render);
    }
    render();
})();
