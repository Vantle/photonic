import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const [chrome, chromedriver, webassembly, javascript] = process.argv.slice(2);
const missing = [];
const type = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
const module = new Map([
    [resolve(root, 'toolchain/browser/module/runtime_bg.wasm'), webassembly],
    [resolve(root, 'toolchain/browser/module/runtime.js'), javascript],
]);
const server = createServer(async (request, response) => {
    const path = resolve(root, `.${new URL(request.url, 'http://localhost').pathname}`);
    if (!path.startsWith(root + sep)) {
        response.writeHead(403).end();
        return;
    }
    try {
        const content = await readFile(module.get(path) ?? path);
        response.writeHead(200, { 'Content-Type': type[extname(path)] ?? 'text/plain' }).end(content);
    } catch {
        missing.push(request.url);
        response.writeHead(404).end();
    }
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const origin = `http://127.0.0.1:${server.address().port}`;
const driver = spawn(chromedriver, ['--port=0'], { stdio: ['ignore', 'pipe', 'pipe'] });
let log = '';
driver.stdout.on('data', value => { log += value; });
driver.stderr.on('data', value => { log += value; });
let session;
let endpoint;
const command = async (path, body) => {
    const response = await fetch(endpoint + path, {
        method: body === undefined ? 'GET' : 'POST',
        ...(body === undefined ? {} : { body: JSON.stringify(body), headers: { 'Content-Type': 'application/json' } }),
        signal: AbortSignal.timeout(30000),
    });
    const result = await response.json();
    assert.ok(response.ok, JSON.stringify(result));
    return result.value;
};
const evaluate = script => command(`/session/${session}/execute/sync`, { script, args: [] });
const until = async (script, limit = 600) => {
    for (let attempt = 0; attempt < limit; attempt++) {
        if (await evaluate(script)) return;
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.fail(`${script}\n${JSON.stringify(await command(`/session/${session}/log`, { type: 'browser' }))}`);
};
const open = async (url, ready) => {
    await command(`/session/${session}/url`, { url });
    await until(ready);
};
const resize = (width, height) => command(`/session/${session}/window/rect`, { width, height });
const capture = async name => {
    const directory = process.env.TEST_UNDECLARED_OUTPUTS_DIR;
    if (!directory) return;
    await writeFile(join(directory, `${name}.png`), Buffer.from(await command(`/session/${session}/screenshot`), 'base64'));
};
const figure = name => `document.querySelector('figure[data-example="${name}"]')`;
const ready = `return document.querySelectorAll('figure.example').length > 0 && [...document.querySelectorAll('figure.example')].every(value => value.querySelector('.graph, .stepper'))`;
const narrow = async () => {
    await resize(390, 844);
    assert.ok(await evaluate('return document.documentElement.scrollWidth <= window.innerWidth + 1'));
    await resize(1440, 1000);
};

try {
    for (let attempt = 0; attempt < 200 && !endpoint; attempt++) {
        const port = /started successfully on port (\d+)/.exec(log)?.[1];
        if (port) endpoint = `http://127.0.0.1:${port}`;
        else await new Promise(resolve => setTimeout(resolve, 50));
        assert.equal(driver.exitCode, null, log);
    }
    assert.ok(endpoint, log);
    session = (await command('/session', { capabilities: { alwaysMatch: {
        browserName: 'chrome',
        'goog:chromeOptions': { binary: chrome, args: ['--headless', '--no-sandbox', '--disable-dev-shm-usage'] },
        'goog:loggingPrefs': { browser: 'ALL' },
    } } })).sessionId;
    await resize(1440, 1000);

    await open(pathToFileURL(join(root, 'index.html')).href, ready);
    assert.equal(await evaluate("return book.engine.state"), 'recorded');
    assert.equal(await evaluate("return document.getElementById('status').textContent"), 'Recorded runs');
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.run').hidden`), true);
    assert.equal(await evaluate("return document.querySelectorAll('figure.example').length"), await evaluate('return Object.keys(book.record.example).length'));
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.message')].filter(value => value.hidden || value.textContent).map(value => value.textContent)"), []);
    assert.equal(await evaluate(`return ${figure('light')}.querySelectorAll('.state').length`), 4);
    assert.deepEqual(await evaluate(`return [...${figure('check')}.querySelectorAll('.verdict .badge')].map(value => value.textContent)`), ['reached', 'unreachable', 'reached']);
    assert.equal(await evaluate(`return ${figure('involution')}.querySelector('.stepper .badge').textContent`), 'reached');
    assert.equal(await evaluate(`return Number(${figure('involution')}.querySelector('input[type=range]').max)`), await evaluate('return book.record.example.involution.result.event'));
    await evaluate("document.querySelector('#field .lens .preset button:nth-child(4)').click()");
    assert.match(await evaluate("return document.querySelector('#field .lens .lowered').textContent"), /2 coherences/);
    assert.deepEqual(await evaluate(`return [...${figure('forever')}.querySelectorAll('.state .badge')].map(value => value.textContent)`), ['start']);
    await evaluate(`${figure('forever')}.querySelector('.state[data-state="16"]').click(); return true`);
    assert.match(await evaluate(`return ${figure('forever')}.querySelector('.departure').textContent`), /no events recorded before the budget ran out/);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.state[data-state="1"] .name').textContent`), 's1end');
    assert.equal(await evaluate("return [...document.querySelectorAll('.graph')].every(graph => !graph.hasAttribute('tabindex') && graph.querySelectorAll('.state[tabindex=\"0\"]').length === 1)"), true);
    await evaluate(`const start = ${figure('first')}.querySelector('.state[data-state="0"]'); start.focus(); start.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })); return true`);
    assert.equal(await evaluate(`return document.activeElement === ${figure('first')}.querySelector('.state[data-state="1"]')`), true);
    await evaluate("document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return true");
    assert.deepEqual(await evaluate(`return [...${figure('first')}.querySelectorAll('.state[aria-current], .state[tabindex="0"]')].map(value => [value.dataset.state, value.getAttribute('aria-current')])`), [['1', 'true']]);
    assert.equal(await evaluate("return document.querySelectorAll('.state[aria-pressed]').length"), 0);
    assert.match(await evaluate("return document.querySelector('#calculator .result').textContent"), /2220₃= 78 in decimal/);
    assert.equal(await evaluate("return document.querySelectorAll('#bench .graph .state').length"), 5);
    assert.equal(await evaluate("return document.querySelectorAll('#bench button.hyperedge').length"), 3);
    assert.equal(await evaluate("return document.querySelectorAll('#bench .capsule').length"), 5);
    await evaluate("const input = document.querySelector('#bench .filter input'); input.value = 'C'; input.dispatchEvent(new Event('input')); return true");
    await until("return document.querySelectorAll('#bench .graph .state').length === 3");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .graph .state')].map(value => value.dataset.state)"), ['1', '3', '4']);
    assert.equal(await evaluate("return document.querySelectorAll('#bench .graph .state[data-match]').length"), 2);
    assert.equal(await evaluate("return document.querySelectorAll('#bench .capsule').length"), 3);
    await evaluate("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true })); return true");
    assert.equal(await evaluate("return document.querySelector('.palette').hidden"), false);
    await evaluate("const input = document.querySelector('.palette input'); input.value = '[C, D] E'; input.dispatchEvent(new Event('input')); return true");
    assert.match(await evaluate("return document.querySelector('.palette li').textContent"), /Filter the workbench by/);
    await evaluate("document.querySelector('.palette input').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return true");
    assert.equal(await evaluate("return document.querySelector('.palette').hidden"), true);
    await evaluate("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true })); const input = document.querySelector('.palette input'); input.value = 'Prism'; input.dispatchEvent(new Event('input')); input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true })); input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return true");
    assert.equal(await evaluate("return document.activeElement === document.querySelector('#prism h2')"), true);
    await until("return document.querySelector('#bench .filter input').value === '[C, D] E'");
    const matched = await evaluate("return [...document.querySelectorAll('#bench .graph .state')].map(value => value.dataset.state).sort()");
    const filter = async text => {
        await evaluate("document.querySelector('#bench .filter .tool').click(); return true");
        await until("return document.querySelector('#bench .filter p').textContent.startsWith('Type a pattern')");
        await evaluate(`const input = document.querySelector('#bench .filter input'); input.value = ${JSON.stringify(text)}; input.dispatchEvent(new Event('input')); return true`);
    };
    await filter('[D,C]E');
    await until("return document.querySelector('#bench .filter p').textContent.startsWith('Showing')");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .graph .state')].map(value => value.dataset.state).sort()"), matched);
    await filter('B.');
    await until("return document.querySelector('#bench .filter p').dataset.tone === 'error'");
    await evaluate("[...document.querySelectorAll('#bench .preset button')].find(value => value.textContent === 'Scope').click(); return true");
    assert.equal(await evaluate("return document.querySelector('#bench .editor textarea').value"), await evaluate('return book.record.example.brew.source'));
    assert.equal(await evaluate("return document.querySelectorAll('#bench .graph .state').length"), await evaluate('return book.record.example.brew.result.execution.state.length'));
    await evaluate("document.querySelector('#bench .filter .tool').click(); return true");
    await evaluate("[...document.querySelectorAll('#bench .preset button')].find(value => value.textContent === 'Conjunction').click(); return true");
    await evaluate("document.querySelector('#bench button.hyperedge.inferred').click(); return true");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .chain > code')].map(value => value.textContent)"), ['[True] Boolean', '[False] Boolean']);
    assert.equal(await evaluate("return document.querySelectorAll('#bench .graph .link.deduction').length"), 2);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .graph .state[data-deduction]')].map(value => value.dataset.state)"), ['3']);
    const inferred = `[...${figure('conjunction')}.querySelectorAll('.departure .event')].find(value => value.querySelector('.deduction'))`;
    assert.equal(await evaluate(`const row = ${inferred}; row.dispatchEvent(new Event('mouseenter')); return row.querySelector('.deduction').textContent`), 'matches s3 after [True] Boolean, [False] Boolean');
    assert.equal(await evaluate(`return ${figure('conjunction')}.querySelectorAll('.link.deduction, .state[data-deduction]').length`), 3);
    await evaluate(`${inferred}.dispatchEvent(new Event('mouseleave')); return true`);
    assert.equal(await evaluate(`return ${figure('conjunction')}.querySelectorAll('.link.deduction, .state[data-deduction]').length`), 0);
    await evaluate("[...document.querySelectorAll('#bench .preset button')].find(value => value.textContent === 'Parallel').click(); return true");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .symmetry .class button')].map(value => value.textContent)"), ['Global A and B; C and D']);
    await evaluate(`${figure('first')}.querySelector('.bar button:not(.run)').click(); return true`);
    await until("return document.querySelector('#bench .editor textarea').value === 'A.X,\\n[A] B'");
    assert.match(await evaluate("return document.querySelector('#bench .bar a').getAttribute('href')"), /^lightbox\.html\?source=/);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.bar a').getAttribute('href')`), 'lightbox.html?source=A.X%2C%0A%5BA%5D+B');
    assert.equal(await evaluate("return document.querySelector('.rail .launch').getAttribute('href')"), 'lightbox.html');
    const connection = "document.querySelector('.connection')";
    assert.equal(await evaluate(`return ${connection}.querySelector('.badge').textContent`), 'recorded run');
    assert.equal(await evaluate(`return ${connection}.querySelector('.verdict').textContent`), 'One shape: Arithmetic, Logic and Geometry differ only in the names of their atoms.');
    assert.deepEqual(await evaluate(`return [...${connection}.querySelectorAll('.output tbody tr')].map(row => [...row.cells].map(cell => cell.textContent))`), [['A', 'Add', 'Xor', 'Compose'], ['B', '0', 'False', 'Keep'], ['C', '1', 'True', 'Flip']]);
    await evaluate(`${connection}.querySelectorAll('.output tbody tr')[1].dispatchEvent(new PointerEvent('pointerenter')); return true`);
    assert.deepEqual(await evaluate(`return [...new Set([...${connection}.querySelectorAll('.program .atom[data-lit]')].map(node => node.textContent))].sort()`), ['0', 'False', 'Keep']);
    await evaluate(`[...${connection}.querySelectorAll('.output .option button')].find(value => value.textContent === 'Geometry').click(); return true`);
    assert.equal(await evaluate(`return ${connection}.querySelector('.output .code').textContent`), '[Compose.Keep.Keep] Keep,\n[Compose.Keep.Flip] Flip,\n[Compose.Flip.Flip] Keep');
    for (const name of await evaluate(`return [...${connection}.querySelectorAll('.preset button')].map(value => value.textContent)`)) {
        await evaluate(`[...${connection}.querySelectorAll('.preset button')].find(value => value.textContent === ${JSON.stringify(name)}).click(); return true`);
        assert.equal(await evaluate(`return ${connection}.querySelector('.message').textContent`), '', name);
        assert.equal(await evaluate(`return ${connection}.querySelectorAll('.output .panel').length`), await evaluate(`return book.record.connection[${JSON.stringify(name)}].result.shape.length`), name);
    }
    assert.match(await evaluate(`return ${connection}.querySelector('.verdict').textContent`), /^One shape: lattice\.converse and lattice\.order/);
    await narrow();
    console.log('The recorded book renders every example, verdict, lens, expression, workbench view, filter and connection from a local file.');

    await open(pathToFileURL(join(root, 'lightbox.html')).href, "return document.querySelectorAll('.lightbox .graph .state').length > 0");
    assert.equal(await evaluate("return document.querySelector('.lightbox .run').hidden"), true);
    assert.equal(await evaluate("return document.querySelector('.lightbox .notice').hidden"), false);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .graph .state').length"), await evaluate('return book.record.example.light.result.execution.state.length'));
    const chip = "document.querySelector('.lightbox .symmetry .class button')";
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .symmetry .class button')].map(value => value.textContent)"), ['Global Red, Green and Blue']);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .graph .token[data-symmetry]')].map(value => `${value.textContent} ${value.dataset.symmetry}`).sort()"), ['Blue global', 'Green global', 'Red global']);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .graph .link[data-symmetry=\"global\"]').length"), 3);
    await evaluate(`${chip}.dispatchEvent(new PointerEvent('pointerenter')); return true`);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .graph .token[data-glow=\"global\"]').length"), 3);
    await evaluate(`${chip}.dispatchEvent(new PointerEvent('pointerleave')); return true`);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox [data-glow]').length"), 0);
    await evaluate("document.querySelector('.lightbox .symmetry input').click(); return true");
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox [data-tint]').length"), 0);
    await evaluate("document.querySelector('.lightbox .symmetry input').click(); return true");
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox [data-tint]').length"), 2);
    await evaluate("[...document.querySelectorAll('.lightbox .preset button')].find(value => value.textContent === 'Negation').click(); return true");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .symmetry .class button')].map(value => value.textContent)"), ['Local False ⇄ True', 'Block Boolean.Not']);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .option button[aria-pressed=\"true\"]')].map(value => value.title)"), ['library/function/invoke.particle', 'library/boolean/not.particle']);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .verdict .badge')].map(value => value.textContent)"), ['reached']);
    await narrow();
    console.log('The recorded Lightbox shows every example with its libraries, targets and symmetries from a local file.');

    await open(`${origin}/index.html`, `${ready} && book.engine.state === 'live'`);
    assert.equal(await evaluate("return document.getElementById('status').textContent"), 'Live engine');
    await evaluate(`const state = ${figure('first')}.querySelector('.state[data-state="1"]'); state.click(); return true`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.state[data-state="1"]').getAttribute('aria-current')`), 'true');
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.departure').textContent`), /no rule applies here/);
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = 'A.X,\\n[A] C';
        area.dispatchEvent(new Event('input'));
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return [...${figure('first')}.querySelectorAll('.state .token')].some(value => value.textContent === 'C')`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.editor pre').textContent`), 'A.X,\n[A] C\n');
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = '[A';
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return ${figure('first')}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.message').textContent`), /at character/);
    await evaluate(`[...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.editor textarea').value`), 'A.X,\n[A] B');
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = '人, [A';
        area.dispatchEvent(new Event('input'));
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return ${figure('first')}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.message').textContent`), /at character 6\)/);
    assert.equal(await evaluate(`return [...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').hidden`), false);
    await evaluate(`[...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const goal = ${figure('check')}.querySelector('.field textarea');
        goal.value = 'B.X';
        goal.dispatchEvent(new Event('input'));
        [...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Workbench').click();
        return true`);
    await until("return document.querySelectorAll('#bench .verdict .claim').length === 3");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .verdict .claim code')].map(value => value.textContent)"), ['B.X, [A] B', 'B.X', 'A.X, [A] B']);
    await evaluate(`[...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const goal = ${figure('check')}.querySelector('.field textarea');
        goal.value = 'B.X, [A] B\\nC.X, [A] B';
        ${figure('check')}.querySelector('.run').click();
        return true`);
    await until(`return [...${figure('check')}.querySelectorAll('.verdict .badge')].map(value => value.textContent).join() === 'reached,unreachable'`);
    await until(`return !${figure('check')}.querySelector('.run').hasAttribute('aria-disabled')`);
    const submit = (edit = '') => evaluate(`
        [...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click();
        const goal = ${figure('check')}.querySelector('.field textarea');
        goal.value = 'B.X, [A] B\\n  C.[';
        goal.focus();
        goal.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', metaKey: true, bubbles: true }));
        ${edit}
        return true`);
    const settled = `return ${figure('check')}.querySelector('.message').dataset.tone === 'error'`;
    await submit();
    await until(settled);
    assert.match(await evaluate(`return ${figure('check')}.querySelector('.message').textContent`), /^Target 2: .*\(at character 3\)$/);
    assert.deepEqual(await evaluate(`const goal = ${figure('check')}.querySelector('.field textarea'); return [document.activeElement === goal, goal.selectionStart, goal.selectionEnd]`), [true, 15, 16]);
    await submit("document.querySelector('#value .lens input').focus();");
    await until(settled);
    assert.match(await evaluate(`return ${figure('check')}.querySelector('.message').textContent`), /^Target 2: /);
    assert.equal(await evaluate("return document.activeElement === document.querySelector('#value .lens input')"), true);
    await submit("goal.value += ' D'; goal.setSelectionRange(goal.value.length, goal.value.length);");
    await until(settled);
    assert.deepEqual(await evaluate(`const goal = ${figure('check')}.querySelector('.field textarea'); return [document.activeElement === goal, goal.selectionStart, goal.selectionEnd, goal.value.length]`), [true, 18, 18, 18]);
    await evaluate(`[...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = '(A, B).(C, D)'; input.dispatchEvent(new Event('input')); return true");
    await until("return /4 coherences/.test(document.querySelector('#value .lens .lowered').textContent)");
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = 'constructor'; input.dispatchEvent(new Event('input')); return true");
    await until("return /1 coherence/.test(document.querySelector('#value .lens .lowered').textContent)");
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = '人, [B'; input.dispatchEvent(new Event('input')); return true");
    await until("return /at character 6\\)/.test(document.querySelector('#value .lens .message').textContent)");
    await evaluate("[...document.querySelectorAll('#calculator .preset button')].find(value => value.textContent === '12 + 2').click(); return true");
    await until("return /21₃= 7 in decimal/.test(document.querySelector('#calculator .result').textContent) && document.querySelector('#calculator .stepper')");
    await evaluate("[...document.querySelectorAll('#calculator .preset button')].find(value => value.textContent === '1 / 0').click(); return true");
    await until("return /Division by zero/.test(document.querySelector('#calculator .message').textContent)");
    await evaluate(`
        window.settled = [];
        const follow = book.engine.path();
        const track = (name, promise) => promise.then(value => settled.push(name + ' ' + value.answer.ternary), error => settled.push(name + ' ' + error.message));
        const queued = new AbortController();
        const running = new AbortController();
        track('first', follow('expression', { source: '12+2' })).then(() => running.abort());
        track('queued', follow('expression', { source: '1+1' }, { signal: queued.signal }));
        track('running', follow('expression', { source: '2*2' }, { signal: running.signal }));
        track('resent', follow('expression', { source: '2+1' }));
        queued.abort();
        return true`);
    await until('return window.settled.length === 4');
    assert.deepEqual(await evaluate('return window.settled'), ['queued Stopped.', 'first 21', 'running Stopped.', 'resent 10']);
    await evaluate(`${figure('involution')}.querySelector('.run').click(); return true`);
    await until(`return !${figure('involution')}.querySelector('.run').hasAttribute('aria-disabled') && ${figure('involution')}.querySelector('.stepper .badge')?.textContent === 'reached' && ${figure('involution')}.querySelector('.pair .state')`);
    await evaluate(`${figure('involution')}.querySelector('[aria-label="Next event"]').click(); return true`);
    await until(`return /^event 2 of/.test(${figure('involution')}.querySelector('.stepper .arrow').textContent) && ${figure('involution')}.querySelector('.pair .name').textContent === 's' + book.record.example.involution.result.step[1].source`);
    await evaluate(`
        const area = ${figure('involution')}.querySelector('.editor textarea');
        area.value = area.value.replace('[Claim]', '[Claim');
        area.dispatchEvent(new Event('input'));
        ${figure('involution')}.querySelector('.run').click();
        return true`);
    await until(`return ${figure('involution')}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${figure('involution')}.querySelector('.message').textContent`), /at character/);
    await evaluate(`${figure('involution')}.querySelector('[aria-label="Last event"]').click(); return true`);
    await until(`return ${figure('involution')}.querySelector('.pair .state') && !/Run the program again/.test(${figure('involution')}.querySelector('.stepper pre').textContent)`);
    await evaluate(`
        const area = ${figure('involution')}.querySelector('.editor textarea');
        area.value = 'A,\\n[A] A.A';
        area.dispatchEvent(new Event('input'));
        ${figure('involution')}.querySelector('.run').click();
        [...${figure('involution')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Stop').click();
        return true`);
    await until(`return ${figure('involution')}.querySelector('.message').textContent === 'Stopped.'`);
    assert.equal(await evaluate(`return ${figure('involution')}.querySelector('.run').hasAttribute('aria-disabled')`), false);
    await evaluate(`[...${figure('involution')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const area = document.querySelector('#bench .editor textarea');
        area.value = 'A, B,\\n[A] C,\\n[C, B] D';
        area.dispatchEvent(new Event('input'));
        document.querySelector('#bench .run').click();
        return true`);
    await until("return document.querySelectorAll('#bench .graph .state').length === 3 && document.querySelectorAll('#bench button.hyperedge').length === 2");
    await evaluate(`
        const area = document.querySelector('#bench .editor textarea');
        area.value = 'X.([A] B),\\n[X.([A] B)] (Y, [Y] Z)';
        area.dispatchEvent(new Event('input'));
        document.querySelector('#bench .run').click();
        return true`);
    await until("return document.querySelectorAll('#bench .graph .held .token').length === 2");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .graph .held .token')].map(value => [value.className, value.textContent, value.querySelectorAll('.atom').length]).sort()"), [['token rule', '[A] B', 2], ['token', 'X', 0]]);
    await evaluate(`
        const area = ${connection}.querySelectorAll('.program textarea')[1];
        area.value = area.value.replace('[Xor.True.True] False', '[Xor.True.True] True');
        area.dispatchEvent(new Event('input'));
        return true`);
    await until(`return ${connection}.querySelector('.verdict').textContent === '2 shapes: Arithmetic and Geometry share one; Logic has its own.'`);
    await evaluate(`
        const area = ${connection}.querySelectorAll('.program textarea')[2];
        area.value = '[Compose';
        area.dispatchEvent(new Event('input'));
        return true`);
    await until(`return ${connection}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${connection}.querySelector('.message').textContent`), /^Geometry: .*at character/);
    assert.equal(await evaluate(`return document.activeElement === ${connection}.querySelectorAll('.program textarea')[2]`), false);
    await capture('book');
    console.log('The live book edits and reruns programs, targets, lenses, expressions, proofs, workbench programs and comparisons in WebAssembly.');

    await open(`${origin}/lightbox.html`, "return book.engine.state === 'live' && document.querySelectorAll('.lightbox .graph .state').length > 0");
    await evaluate(`
        const area = document.querySelector('.lightbox .editor textarea');
        area.value = 'A, B,\\n[A] C,\\n[B] D';
        area.dispatchEvent(new Event('input'));
        document.querySelector('.lightbox .run').click();
        return true`);
    await until("return document.querySelectorAll('.lightbox .graph .state').length === 4 && document.querySelectorAll('.lightbox button.hyperedge').length === 2");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .symmetry .class button')].map(value => value.textContent)"), ['Global A and B; C and D']);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox button.hyperedge[data-symmetry=\"global\"]').length"), 2);
    assert.match(await evaluate('return new URLSearchParams(location.search).get("source")'), /\[B\] D$/);
    await open(await evaluate('return location.href'), "return book.engine.state === 'live' && document.querySelectorAll('.lightbox .graph .state').length === 4");
    await open(`${origin}/lightbox.html?source=Q`, "return book.engine.state === 'live' && document.querySelectorAll('.lightbox .graph .state').length === 1");
    await evaluate("[...document.querySelectorAll('.lightbox .preset button')].find(value => value.textContent === 'Cycle').click(); return true");
    assert.match(await evaluate('return location.search'), /^\?source=/);
    await open(`${origin}/lightbox.html`, "return document.querySelector('.lightbox .editor textarea').value.endsWith('[B] D') && document.querySelectorAll('.lightbox .graph .state').length === 4");
    await evaluate("[...document.querySelectorAll('.lightbox .bar button')].find(value => value.textContent === 'New').click(); return true");
    assert.equal(await evaluate("return document.querySelector('.lightbox .editor textarea').value"), '');
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .blank').length"), 1);
    assert.equal(await evaluate("return document.querySelector('.lightbox .symmetry').hidden"), true);
    await narrow();
    console.log('The live Lightbox runs new programs, restores shared links, keeps the reader’s draft through links and examples, and starts afresh.');

    await evaluate("document.getElementById('theme').click(); return true");
    assert.equal(await evaluate('return document.documentElement.dataset.theme'), 'light');
    await evaluate("document.getElementById('theme').click(); return true");
    assert.equal(await evaluate('return document.documentElement.dataset.theme'), 'dark');
    await open(`${origin}/index.html`, ready);
    assert.equal(await evaluate('return document.documentElement.dataset.theme'), 'dark');
    await evaluate("document.getElementById('theme').click(); return true");
    assert.equal(await evaluate("return document.documentElement.dataset.theme ?? 'system'"), 'system');
    assert.deepEqual(await evaluate(`
        const identity = [...document.querySelectorAll('[id]')].map(value => value.id);
        return identity.filter((value, index) => identity.indexOf(value) !== index);
    `), []);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('a[href^=\"#\"]')].filter(link => link.hash.length > 1 && !document.getElementById(decodeURIComponent(link.hash.slice(1)))).map(link => link.hash)"), []);
    assert.equal(await evaluate("return [...document.querySelectorAll('#outline a')].every(link => document.getElementById(link.hash.slice(1))?.classList.contains('chapter'))"), true);
    assert.equal(await evaluate("return document.querySelectorAll('#outline a').length === document.querySelectorAll('.chapter').length"), true);
    assert.deepEqual(await evaluate(`
        const pre = document.createElement('pre');
        book.syntax.highlight(pre, '<img src=x onerror=alert(1)> [A] B');
        return [pre.textContent, pre.querySelectorAll('img').length];
    `), ['<img src=x onerror=alert(1)> [A] B', 0]);
    await narrow();
    const message = await command(`/session/${session}/log`, { type: 'browser' });
    assert.deepEqual(message.filter(value => value.level === 'SEVERE'), []);
    assert.deepEqual(missing, []);
    console.log('The book keeps its theme, anchors, safe highlighting and narrow layout, with no console errors or missing assets.');
} finally {
    if (session) await fetch(`${endpoint}/session/${session}`, { method: 'DELETE', signal: AbortSignal.timeout(5000) }).catch(() => {});
    driver.kill();
    server.close();
    server.closeAllConnections();
}
