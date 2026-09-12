(() => {
    const quantity = document.getElementById('quantity');
    const label = document.getElementById('quantity-label');
    const particle = document.getElementById('particle');
    const numeral = document.getElementById('numeral');
    const identity = document.getElementById('identity');
    const successor = document.getElementById('successor');
    let renamed = false;
    const render = () => {
        const count = Number(quantity.value);
        label.textContent = count;
        particle.replaceChildren();
        if (count === 0) {
            const empty = document.createElement('span');
            empty.className = 'empty';
            empty.textContent = 'One coherence · no occurrences';
            particle.append(empty);
        }
        for (let index = 0; index < count; index++) {
            const unit = document.createElement('span');
            unit.className = 'unit';
            unit.textContent = 'Unit';
            const name = document.createElement('small');
            name.textContent = `#${renamed ? 100 + count - index : index + 1}`;
            unit.append(name);
            particle.append(unit);
        }
        numeral.textContent = count ? Array(count).fill('Unit').join('.') : '()';
        identity.textContent = renamed ? 'The identifiers changed; the numeral stayed equal. Multiplicity is preserved.' : 'Occurrence identifiers are bookkeeping. Only the Unit multiplicity belongs to this numeral.';
        successor.disabled = count === Number(quantity.max);
    };
    quantity.addEventListener('input', render);
    successor.addEventListener('click', () => { quantity.value = Number(quantity.value) + 1; render(); });
    document.getElementById('zero').addEventListener('click', () => { quantity.value = 0; render(); });
    document.getElementById('rename').addEventListener('click', () => { renamed = !renamed; render(); });
    render();
    document.addEventListener('DOMContentLoaded', () => {
        const sample = document.getElementById('native-example');
        sample.value = quantity.value;
        sample.dispatchEvent(new Event('change'));
    });
})();
