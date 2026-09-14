(() => {
    'use strict';
    const find = name => document.getElementById(name);
    const record = globalThis.expression;
    let index = 0;
    const render = () => {
        const event = record.event[index];
        find('expression-operation').textContent = event.operation;
        find('expression-result').textContent = `${event.ternary}₃ = ${event.decimal}₁₀`;
        find('expression-position').textContent = `${index + 1} / ${record.event.length}`;
        find('expression-rule').textContent = event.rule;
        find('expression-state').textContent = `Recorded configuration ${event.source} → ${event.target}`;
        find('expression-back').disabled = index === 0;
        find('expression-next').disabled = index === record.event.length - 1;
    };
    find('expression-back').addEventListener('click', () => { index--; render(); });
    find('expression-next').addEventListener('click', () => { index++; render(); });
    find('expression-count').textContent = `${record.count.toLocaleString()} native events · ${record.work.toLocaleString()} work steps · ${record.outcome}`;
    render();
})();
