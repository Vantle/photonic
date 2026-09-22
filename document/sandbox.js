(() => {
    const find = name => document.getElementById(`sandbox-${name}`);
    let worker;
    let timer;
    let count = 0;
    let index = 0;
    const status = message => {
        clearTimeout(timer);
        find('run').disabled = false;
        find('stop').disabled = true;
        find('status').textContent = message;
    };
    const stop = message => {
        worker?.terminate();
        worker = undefined;
        find('trace').hidden = true;
        status(message);
    };
    const inspect = value => {
        if (!worker || !count) return;
        if (!Number.isInteger(value)) { find('step').value = index + 1; return; }
        index = Math.max(0, Math.min(count - 1, value));
        find('step').value = index + 1;
        find('position').textContent = `Loading event ${index + 1}…`;
        worker.postMessage({ kind: 'inspect', index });
    };
    const render = data => {
        if (data.index !== index) return;
        find('position').textContent = `Event ${index + 1} of ${count.toLocaleString()}`;
        find('back').disabled = index === 0;
        find('next').disabled = index + 1 === count;
        find('transition').textContent = `Configuration ${data.event.source} → Configuration ${data.event.target}`;
        globalThis.syntax.highlight(find('rule'), data.event.rule);
        globalThis.inspection.render(find('before'), data.before, data.event, true, data.definition);
        globalThis.inspection.render(find('after'), data.after, data.event, false, data.definition);
        find('path').replaceChildren(...Array.from({ length: Math.min(count, 7) }, (_, offset) => {
            const value = Math.max(0, Math.min(index - 3, count - 7)) + offset;
            const button = document.createElement('button');
            button.type = 'button';
            button.textContent = `Event ${value + 1}`;
            button.setAttribute('aria-pressed', String(value === index));
            button.addEventListener('click', () => inspect(value));
            return button;
        }));
    };
    find('form').addEventListener('submit', event => {
        event.preventDefault();
        stop('Running Photonic in WebAssembly…');
        find('result').textContent = '';
        find('source').textContent = '';
        if (location.protocol === 'file:') {
            status('Start the local server with the command below to load WebAssembly.');
            return;
        }
        find('run').disabled = true;
        find('stop').disabled = false;
        worker = new Worker('document/calculation.js', { type: 'module' });
        worker.onmessage = ({ data }) => {
            if (data.kind === 'inspect') {
                if (data.error) find('position').textContent = data.error;
                else render(data);
                return;
            }
            count = data.event ?? 0;
            globalThis.syntax.highlight(find('source'), data.source ?? '');
            if (data.error) status(data.error);
            else {
                find('result').textContent = `${data.ternary} (base 3) = ${data.decimal} (decimal)`;
                status(`Executed here in WebAssembly · ${count.toLocaleString()} native events · ${data.work.toLocaleString()} work steps`);
            }
            find('trace').hidden = !count;
            find('step').max = count;
            inspect(0);
        };
        worker.onerror = event => { event.preventDefault(); stop('Could not run WebAssembly. Use the local server command below and try again.'); };
        timer = setTimeout(() => stop('Stopped after 20 seconds; no completed result. Try a smaller expression.'), 20000);
        worker.postMessage({ kind: 'run', input: find('input').value });
    });
    find('stop').addEventListener('click', () => stop('Stopped; no completed result.'));
    const preview = () => globalThis.syntax.highlight(find('preview'), find('input').value, true);
    find('input').addEventListener('input', preview);
    find('demo').addEventListener('change', () => { find('input').value = find('demo').value; preview(); });
    find('back').addEventListener('click', () => inspect(index - 1));
    find('next').addEventListener('click', () => inspect(index + 1));
    find('first').addEventListener('click', () => inspect(0));
    find('last').addEventListener('click', () => inspect(count - 1));
    find('step').addEventListener('change', () => inspect(Number(find('step').value) - 1));
    preview();
})();
