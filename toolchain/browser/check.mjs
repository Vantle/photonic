import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
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
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.message')].filter(value => !value.hidden).map(value => value.textContent)"), []);
    assert.equal(await evaluate(`return ${figure('light')}.querySelectorAll('.state').length`), 4);
    assert.deepEqual(await evaluate(`return [...${figure('check')}.querySelectorAll('.verdict .badge')].map(value => value.textContent)`), ['reached', 'unreachable', 'reached']);
    assert.equal(await evaluate(`return ${figure('involution')}.querySelector('.stepper .badge').textContent`), 'reached');
    assert.equal(await evaluate(`return Number(${figure('involution')}.querySelector('input[type=range]').max)`), await evaluate('return book.record.example.involution.result.event'));
    await evaluate("document.querySelector('#field .lens .preset button:nth-child(3)').click()");
    assert.match(await evaluate("return document.querySelector('#field .lens .lowered').textContent"), /2 coherences/);
    assert.deepEqual(await evaluate(`return [...${figure('forever')}.querySelectorAll('.state .badge')].map(value => value.textContent)`), ['start']);
    await evaluate(`${figure('forever')}.querySelector('.state[data-state="16"]').click(); return true`);
    assert.match(await evaluate(`return ${figure('forever')}.querySelector('.departure').textContent`), /no events recorded before the budget ran out/);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.state[data-state="1"] .name').textContent`), 's1end');
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
    await evaluate("[...document.querySelectorAll('#bench .preset button')].find(value => value.textContent === 'Parallel').click(); return true");
    await evaluate(`${figure('first')}.querySelector('.bar button:not(.run)').click(); return true`);
    await until("return document.querySelector('#bench .editor textarea').value === 'A.X\\n[A] B'");
    assert.match(await evaluate("return document.querySelector('#bench .bar a').getAttribute('href')"), /^lightbox\.html\?source=/);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.bar a').getAttribute('href')`), 'lightbox.html?source=A.X%0A%5BA%5D+B');
    assert.equal(await evaluate("return document.querySelector('.rail .launch').getAttribute('href')"), 'lightbox.html');
    await narrow();
    console.log('The recorded book renders every example, verdict, lens, expression, workbench view and filter from a local file.');

    await open(pathToFileURL(join(root, 'lightbox.html')).href, "return document.querySelectorAll('.lightbox .graph .state').length > 0");
    assert.equal(await evaluate("return document.querySelector('.lightbox .run').hidden"), true);
    assert.equal(await evaluate("return document.querySelector('.lightbox .notice').hidden"), false);
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .graph .state').length"), await evaluate('return book.record.example.light.result.execution.state.length'));
    await evaluate("[...document.querySelectorAll('.lightbox .preset button')].find(value => value.textContent === 'Negation').click(); return true");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .option button[aria-pressed=\"true\"]')].map(value => value.title)"), ['library/function/invoke.particle', 'library/boolean/not.particle']);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('.lightbox .verdict .badge')].map(value => value.textContent)"), ['reached']);
    await narrow();
    console.log('The recorded Lightbox shows every example with its libraries and targets from a local file.');

    await open(`${origin}/index.html`, `${ready} && book.engine.state === 'live'`);
    assert.equal(await evaluate("return document.getElementById('status').textContent"), 'Live engine');
    await evaluate(`const state = ${figure('first')}.querySelector('.state[data-state="1"]'); state.click(); return true`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.state[data-state="1"]').getAttribute('aria-pressed')`), 'true');
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.departure').textContent`), /no rule applies here/);
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = 'A.X\\n[A] C';
        area.dispatchEvent(new Event('input'));
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return [...${figure('first')}.querySelectorAll('.state .token')].some(value => value.textContent === 'C')`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.editor pre').textContent`), 'A.X\n[A] C\n');
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = '[A';
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return ${figure('first')}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.message').textContent`), /at character/);
    await evaluate(`[...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    assert.equal(await evaluate(`return ${figure('first')}.querySelector('.editor textarea').value`), 'A.X\n[A] B');
    await evaluate(`
        const area = ${figure('first')}.querySelector('.editor textarea');
        area.value = '人 [A';
        area.dispatchEvent(new Event('input'));
        ${figure('first')}.querySelector('.run').click();
        return true`);
    await until(`return ${figure('first')}.querySelector('.message').dataset.tone === 'error'`);
    assert.match(await evaluate(`return ${figure('first')}.querySelector('.message').textContent`), /at character 5\)/);
    assert.equal(await evaluate(`return [...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').hidden`), false);
    await evaluate(`[...${figure('first')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const goal = ${figure('check')}.querySelector('.field textarea');
        goal.value = 'B.X';
        goal.dispatchEvent(new Event('input'));
        [...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Workbench').click();
        return true`);
    await until("return document.querySelectorAll('#bench .verdict .claim').length === 3");
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#bench .verdict .claim code')].map(value => value.textContent)"), ['B.X [A] B', 'B.X', 'A.X [A] B']);
    await evaluate(`[...${figure('check')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const goal = ${figure('check')}.querySelector('.field textarea');
        goal.value = 'B.X [A] B\\nC.X [A] B';
        ${figure('check')}.querySelector('.run').click();
        return true`);
    await until(`return [...${figure('check')}.querySelectorAll('.verdict .badge')].map(value => value.textContent).join() === 'reached,unreachable'`);
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = '(A, B).(C, D)'; input.dispatchEvent(new Event('input')); return true");
    await until("return /4 coherences/.test(document.querySelector('#value .lens .lowered').textContent)");
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = 'constructor'; input.dispatchEvent(new Event('input')); return true");
    await until("return /1 coherence/.test(document.querySelector('#value .lens .lowered').textContent)");
    await evaluate("const input = document.querySelector('#value .lens input'); input.value = '人 [B'; input.dispatchEvent(new Event('input')); return true");
    await until("return /at character 5\\)/.test(document.querySelector('#value .lens .message').textContent)");
    await evaluate("[...document.querySelectorAll('#calculator .preset button')].find(value => value.textContent === '12 + 2').click(); return true");
    await until("return /21₃= 7 in decimal/.test(document.querySelector('#calculator .result').textContent) && document.querySelector('#calculator .stepper')");
    await evaluate("[...document.querySelectorAll('#calculator .preset button')].find(value => value.textContent === '1 / 0').click(); return true");
    await until("return /Division by zero/.test(document.querySelector('#calculator .message').textContent)");
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
        area.value = 'A\\n[A] A.A';
        area.dispatchEvent(new Event('input'));
        ${figure('involution')}.querySelector('.run').click();
        [...${figure('involution')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Stop').click();
        return true`);
    await until(`return ${figure('involution')}.querySelector('.message').textContent === 'Stopped.'`);
    assert.equal(await evaluate(`return ${figure('involution')}.querySelector('.run').hasAttribute('aria-disabled')`), false);
    await evaluate(`[...${figure('involution')}.querySelectorAll('.bar button')].find(value => value.textContent === 'Reset').click(); return true`);
    await evaluate(`
        const area = document.querySelector('#bench .editor textarea');
        area.value = 'A, B\\n[A] C\\n[C, B] D';
        area.dispatchEvent(new Event('input'));
        document.querySelector('#bench .run').click();
        return true`);
    await until("return document.querySelectorAll('#bench .graph .state').length === 3 && document.querySelectorAll('#bench button.hyperedge').length === 2");
    await capture('book');
    console.log('The live book edits and reruns programs, targets, lenses, expressions, proofs and workbench programs in WebAssembly.');

    await open(`${origin}/lightbox.html`, "return book.engine.state === 'live' && document.querySelectorAll('.lightbox .graph .state').length > 0");
    await evaluate(`
        const area = document.querySelector('.lightbox .editor textarea');
        area.value = 'A, B\\n[A] C\\n[B] D';
        area.dispatchEvent(new Event('input'));
        document.querySelector('.lightbox .run').click();
        return true`);
    await until("return document.querySelectorAll('.lightbox .graph .state').length === 4 && document.querySelectorAll('.lightbox button.hyperedge').length === 2");
    assert.match(await evaluate('return new URLSearchParams(location.search).get("source")'), /\[B\] D$/);
    await open(await evaluate('return location.href'), "return book.engine.state === 'live' && document.querySelectorAll('.lightbox .graph .state').length === 4");
    await open(`${origin}/lightbox.html`, "return document.querySelector('.lightbox .editor textarea').value.endsWith('[B] D') && document.querySelectorAll('.lightbox .graph .state').length === 4");
    await evaluate("[...document.querySelectorAll('.lightbox .bar button')].find(value => value.textContent === 'New').click(); return true");
    assert.equal(await evaluate("return document.querySelector('.lightbox .editor textarea').value"), '');
    assert.equal(await evaluate("return document.querySelectorAll('.lightbox .blank').length"), 1);
    await narrow();
    console.log('The live Lightbox runs new programs, restores shared links and drafts, and starts afresh.');

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
