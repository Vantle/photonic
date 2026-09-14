(() => {
    'use strict';
    const find = name => document.getElementById(name);
    const demo = [{
        name: 'Four operations',
        expression: '1212 × 10 ÷ 2 + 11 − 1',
        path: 'mathematics/ternary/infix.wave',
        command: 'bazel test -c opt //mathematics/ternary:infix.check --test_output=all',
        ...globalThis.expression,
    }, ...globalThis.demo];
    const select = find('expression-select');
    demo.forEach((value, index) => {
        const option = document.createElement('option');
        option.value = index;
        option.textContent = `${value.name}: ${value.expression}`;
        select.append(option);
    });
    let index = 0;
    const render = () => {
        const record = demo[Number(select.value)];
        const event = record.event[index];
        find('expression-input').textContent = record.expression;
        find('expression-operation').textContent = event.operation;
        find('expression-result').textContent = `${event.ternary}₃ = ${event.decimal}₁₀`;
        find('expression-position').textContent = `${index + 1} / ${record.event.length}`;
        find('expression-rule').textContent = event.rule;
        find('expression-state').textContent = `Recorded configuration ${event.source} → ${event.target}`;
        find('expression-back').disabled = index === 0;
        find('expression-next').disabled = index === record.event.length - 1;
        find('expression-count').textContent = `${record.count.toLocaleString()} native events · ${record.work.toLocaleString()} work steps · ${record.outcome}`;
        find('expression-command').textContent = record.command;
        find('expression-link').href = record.path;
        find('expression-source').textContent = record.source ?? '';
        find('expression-source-panel').hidden = !record.source;
        find('expression-note').textContent = event.operation === 'Divide' ? 'Integer quotient: division truncates toward zero; the expression evaluator discards the remainder.' : '';
        find('expression-tape').replaceChildren(...record.input.map(value => {
            const tile = document.createElement('span');
            tile.className = 'token';
            tile.textContent = value;
            return tile;
        }));
        find('expression-trit').replaceChildren(...[...event.ternary].map((digit, position) => {
            const power = event.ternary.length - position - 1;
            const weight = 3n ** BigInt(power);
            const tile = document.createElement('div');
            tile.className = 'trit-card';
            const value = document.createElement('strong');
            value.textContent = digit;
            const label = document.createElement('span');
            label.textContent = `${digit} × 3^${power} = ${BigInt(digit) * weight}₁₀`;
            tile.append(value, label);
            return tile;
        }));
    };
    select.addEventListener('change', () => { index = 0; render(); });
    find('expression-back').addEventListener('click', () => { if (index > 0) index--; render(); });
    find('expression-next').addEventListener('click', () => {
        if (index + 1 < demo[Number(select.value)].event.length) index++;
        render();
    });
    render();
})();
