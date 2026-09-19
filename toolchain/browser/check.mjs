import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, resolve, sep } from 'node:path';

import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const missing = [];
const type = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
const server = createServer(async (request, response) => {
    const path = resolve(root, `.${new URL(request.url, 'http://localhost').pathname}`);
    if (!path.startsWith(root + sep)) {
        response.writeHead(403).end();
        return;
    }
    try {
        const content = await readFile(path === resolve(root, "toolchain/browser/module/runtime_bg.wasm") ? process.argv[4] : path === resolve(root, "toolchain/browser/module/runtime.js") ? process.argv[5] : path);
        response.writeHead(200, { 'Content-Type': type[extname(path)] ?? 'text/plain' }).end(content);
    } catch {
        missing.push(request.url);
        response.writeHead(404).end();
    }
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const origin = `http://127.0.0.1:${server.address().port}`;
const chrome = process.argv[2];
const driver = spawn(process.argv[3], ['--port=0'], { stdio: ['ignore', 'pipe', 'pipe'] });
let log = '';
driver.stdout.on('data', value => { log += value; });
driver.stderr.on('data', value => { log += value; });
let session;
let endpoint;
async function command(path, body) {
    const response = await fetch(endpoint + path, {
        method: body === undefined ? 'GET' : 'POST',
        ...(body === undefined ? {} : { body: JSON.stringify(body), headers: { 'Content-Type': 'application/json' } }),
        signal: AbortSignal.timeout(30000),
    });
    const result = await response.json();
    assert.ok(response.ok, JSON.stringify(result));
    return result.value;
}
const evaluate = script => command(`/session/${session}/execute/sync`, { script, args: [] });
async function navigate(path, ready) {
    await command(`/session/${session}/url`, { url: origin + path });
    for (let attempt = 0; attempt < 200; attempt++) {
        if (await evaluate(ready)) return;
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.fail(JSON.stringify(await command(`/session/${session}/log`, { type: 'browser' })));
}
try {
    for (let attempt = 0; attempt < 200; attempt++) {
        const port = /started successfully on port (\d+)/.exec(log)?.[1];
        if (port) {
            endpoint = `http://127.0.0.1:${port}`;
            break;
        }
        assert.equal(driver.exitCode, null, log);
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.ok(endpoint, log);
    session = (await command('/session', { capabilities: { alwaysMatch: {
        browserName: 'chrome',
        'goog:chromeOptions': { binary: chrome, args: ['--headless', '--no-sandbox', '--disable-dev-shm-usage'] },
        'goog:loggingPrefs': { browser: 'ALL' },
    } } })).sessionId;
    await navigate('/index.html', "return document.getElementById('operation-result')?.textContent === '184,500'");
    assert.equal(await evaluate("return document.getElementById('operation-result').textContent.replaceAll(',', '')"), '184500');
    assert.ok(await evaluate("return document.getElementById('library').textContent.includes('photonic_test')"));
    assert.match(await evaluate("return document.getElementById('test-contract').textContent"), /Unknown never satisfies/);
    assert.match(await evaluate("return document.getElementById('function-interface').textContent"), /Invoke activates the supplied Function rule/);
    assert.match(await evaluate("return document.getElementById('packing').textContent"), /Without the library rules/);
    await evaluate("const select = document.getElementById('lowering-select'); select.selectedIndex = 2; select.dispatchEvent(new Event('change'));");
    assert.equal(await evaluate("return document.getElementById('lowering-output').textContent"), 'Pack.Position.0, Pack.Value.2');
    await evaluate("const select = document.getElementById('lowering-select'); select.selectedIndex = 3; select.dispatchEvent(new Event('change'));");
    assert.equal(await evaluate("return document.getElementById('lowering-output').textContent"), 'Invoke.Pack.([Position] 0).([Value] 2)');
    assert.match(await evaluate("return document.getElementById('lowering-description').textContent"), /does not execute/);
    await evaluate("document.getElementById('theme').click()");
    assert.equal(await evaluate('return document.documentElement.dataset.theme'), 'dark');
    await navigate('/index.html', "return document.getElementById('operation-result')?.textContent === '184,500'");
    assert.equal(await evaluate('return document.documentElement.dataset.theme'), 'dark');
    for (const [operation, answer, remainder] of [['add', '1623', ''], ['subtract', '1377', ''], ['multiply', '184500', ''], ['divide', '12', 'remainder 24']]) {
        await evaluate(`const select = document.getElementById('operation'); select.value = '${operation}'; select.dispatchEvent(new Event('change'));`);
        assert.equal(await evaluate("return document.getElementById('operation-result').textContent.replaceAll(',', '')"), answer);
        assert.equal(await evaluate("return document.getElementById('operation-remainder').textContent"), remainder);
        assert.equal(await evaluate("return document.getElementById('operation-back').disabled"), true);
        await evaluate("document.getElementById('operation-next').click()");
        assert.match(await evaluate("return document.getElementById('operation-position').textContent"), /^Event 2 /);
        await evaluate("document.getElementById('operation-back').click()");
        assert.equal(await evaluate("return document.getElementById('operation-back').disabled"), true);
    }
    assert.equal(await evaluate("return document.querySelectorAll('#evaluation-graph [data-state]').length"), 4);
    await evaluate("document.querySelector('#evaluation-graph [data-state=\"1\"]').dispatchEvent(new MouseEvent('click'))");
    assert.ok(await evaluate("return document.getElementById('evaluation-state').textContent.length > 0"));
    await evaluate("document.getElementById('evaluation-run').click()");
    for (let attempt = 0; attempt < 200; attempt++) {
        if (await evaluate("return !document.getElementById('evaluation-run').disabled")) break;
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.match(await evaluate("return document.getElementById('evaluation-status').textContent"), /Executed here/);
    assert.match(await evaluate("return document.getElementById('evaluation-verdict').textContent"), /D: reached/);
    assert.match(await evaluate("return document.getElementById('evaluation-verdict').textContent"), /C.D: reached/);
    await evaluate("document.getElementById('evaluation-source').value = 'A [A] B'; document.getElementById('evaluation-target').value = '[\"B\", \"C\"]'; document.getElementById('evaluation-run').click()");
    for (let attempt = 0; attempt < 200; attempt++) {
        if (await evaluate("return !document.getElementById('evaluation-run').disabled")) break;
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.match(await evaluate("return document.getElementById('evaluation-verdict').textContent"), /B: reached.*C: unreachable/);
    await evaluate("document.getElementById('evaluation-run').click(); document.getElementById('evaluation-stop').click()");
    assert.match(await evaluate("return document.getElementById('evaluation-status').textContent"), /^Stopped/);
    await command(`/session/${session}/window/rect`, { width: 1440, height: 1000 });
    await evaluate("document.documentElement.style.scrollBehavior = 'auto'; document.getElementById('evaluation-reset').click(); document.getElementById('evaluation-graph').scrollIntoView({block: 'center', behavior: 'instant'})");
    if (process.env.TEST_UNDECLARED_OUTPUTS_DIR) {
        const screenshot = await command(`/session/${session}/screenshot`);
        await writeFile(resolve(process.env.TEST_UNDECLARED_OUTPUTS_DIR, 'graph.png'), Buffer.from(screenshot, 'base64'));
    }
    assert.equal(await evaluate("return document.getElementById('product-equation').textContent"), '12 (base 3) × 21 (base 3) = 1022 (base 3) · 5 × 7 = 35');
    assert.equal(await evaluate("return document.querySelectorAll('#product-graph path').length"), 8);
    assert.equal(await evaluate("return document.querySelectorAll('#product-column tr').length"), 4);
    assert.deepEqual(await evaluate(`
        const failure = [];
        for (let left = 0; left < 9; left++) for (let right = 0; right < 9; right++) {
            document.getElementById('product-left').value = left;
            const select = document.getElementById('product-right');
            select.value = right;
            select.dispatchEvent(new Event('change'));
            const row = [...document.querySelectorAll('#product-column tr')].map(row => [...row.cells].slice(1).map(cell => Number(cell.textContent)));
            const value = row.reduce((sum, cell, position) => sum + cell[2] * 3 ** position, 0);
            if (value !== left * right || row.some(cell => cell[0] + cell[1] !== cell[2] + 3 * cell[3])) failure.push([left, right]);
        }
        document.getElementById('product-left').value = 5;
        document.getElementById('product-right').value = 7;
        document.getElementById('product-right').dispatchEvent(new Event('change'));
        return failure;
    `), []);
    await evaluate("document.getElementById('product').scrollIntoView({block: 'start', behavior: 'instant'})");
    if (process.env.TEST_UNDECLARED_OUTPUTS_DIR) {
        const screenshot = await command(`/session/${session}/screenshot`);
        await writeFile(resolve(process.env.TEST_UNDECLARED_OUTPUTS_DIR, 'product.png'), Buffer.from(screenshot, 'base64'));
    }
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#ternary h3[id]')].map(value => value.id)"), ['ternary-addition', 'ternary-subtraction', 'ternary-multiplication', 'ternary-division', 'expression']);
    assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), '12120 (base 3) = 150 (decimal)');
    assert.equal(await evaluate("return document.getElementById('expression-back').disabled"), true);
    for (const expected of ['2210 (base 3) = 75 (decimal)', '2221 (base 3) = 79 (decimal)', '2220 (base 3) = 78 (decimal)']) {
        await evaluate("document.getElementById('expression-next').click()");
        assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), expected);
    }
    assert.equal(await evaluate("return document.getElementById('expression-next').disabled"), true);
    assert.match(await evaluate("return document.getElementById('expression-rule').textContent"), /Evaluate.Pending.Subtract/);
    await evaluate("document.getElementById('expression-back').click()");
    assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), '2221 (base 3) = 79 (decimal)');
    assert.equal(await evaluate("return document.getElementById('expression-select').options.length"), 4);
    for (const [selection, expected, count] of [[1, '21 (base 3) = 7 (decimal)', 2], [2, '101 (base 3) = 10 (decimal)', 3], [3, '10 (base 3) = 3 (decimal)', 2]]) {
        await evaluate(`const select = document.getElementById('expression-select'); select.value = ${selection}; select.dispatchEvent(new Event('change'));`);
        assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), expected);
        assert.equal(await evaluate("return document.querySelectorAll('#expression-trit .trit-card').length"), count);
        assert.equal(await evaluate("return document.getElementById('expression-back').disabled"), true);
        assert.equal(await evaluate("return document.getElementById('expression-source-panel').hidden"), false);
        assert.ok(await evaluate("return document.getElementById('expression-source').textContent.includes('[Built,Stage.')"));
        assert.equal(await evaluate("return document.getElementById('expression-source').textContent.includes('Result.')"), false);
    }
    assert.match(await evaluate("return document.getElementById('expression-note').textContent"), /truncates toward zero/);
    assert.deepEqual(await evaluate("return [...document.querySelectorAll('#expression-tape .token')].map(value => value.textContent)"), ['2', '1', 'Divide', '2', 'Subtract', '1']);
    await evaluate("document.getElementById('expression-next').click()");
    assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), '2 (base 3) = 2 (decimal)');
    assert.equal(await evaluate("return document.getElementById('expression-next').disabled"), true);
    await evaluate("const select = document.getElementById('expression-select'); select.value = 0; select.dispatchEvent(new Event('change'));");
    assert.equal(await evaluate("return document.getElementById('expression-source-panel').hidden"), true);
    assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), '12120 (base 3) = 150 (decimal)');
    await evaluate("document.getElementById('expression-lab').scrollIntoView({block: 'start', behavior: 'instant'})");
    if (process.env.TEST_UNDECLARED_OUTPUTS_DIR) {
        const screenshot = await command(`/session/${session}/screenshot`);
        await writeFile(resolve(process.env.TEST_UNDECLARED_OUTPUTS_DIR, 'expression.png'), Buffer.from(screenshot, 'base64'));
    }
    await command(`/session/${session}/window/rect`, { width: 390, height: 844 });
    assert.ok(await evaluate("return document.documentElement.scrollWidth <= window.innerWidth + 1"));
    await evaluate("const select = document.getElementById('expression-select'); select.value = 3; select.dispatchEvent(new Event('change')); document.getElementById('expression-lab').scrollIntoView({block: 'start', behavior: 'instant'});");
    assert.equal(await evaluate("return document.getElementById('expression-result').textContent"), '10 (base 3) = 3 (decimal)');
    await command(`/session/${session}/window/rect`, { width: 1440, height: 1000 });
    for (const [input, expected] of [['12 + 2', '21 (base 3) = 7 (decimal)'], ['-(12 + 2) * 10', '-210 (base 3) = -21 (decimal)'], ['1212 * 10 / 2 + 11 - 1', '2220 (base 3) = 78 (decimal)'], ['2*2*2*2*2*2', '2101 (base 3) = 64 (decimal)'], ['2*2*2*2*2*2*2*2*2*2', '1101221 (base 3) = 1024 (decimal)']]) {
        await evaluate(`document.getElementById('sandbox-input').value = ${JSON.stringify(input)}; document.getElementById('sandbox-run').click();`);
        for (let attempt = 0; attempt < 450; attempt++) {
            if (await evaluate("return !document.getElementById('sandbox-run').disabled")) break;
            await new Promise(resolve => setTimeout(resolve, 50));
        }
        assert.equal(await evaluate("return document.getElementById('sandbox-result').textContent"), expected);
        assert.match(await evaluate("return document.getElementById('sandbox-status').textContent"), /Executed here in WebAssembly/);
        for (let attempt = 0; attempt < 100; attempt++) {
            if (await evaluate("return /^Event 1 of/.test(document.getElementById('sandbox-position').textContent)")) break;
            await new Promise(resolve => setTimeout(resolve, 20));
        }
        assert.equal(await evaluate("return document.getElementById('sandbox-trace').hidden"), false);
        assert.match(await evaluate("return document.getElementById('sandbox-before').textContent"), /Configuration 0/);
        assert.ok(await evaluate("return document.querySelectorAll('#sandbox-rule .syntax-group').length > 0"));
        assert.ok(await evaluate("return document.querySelectorAll('#sandbox-source .syntax-number').length > 0"));
        await evaluate("document.getElementById('sandbox-last').click()");
        for (let attempt = 0; attempt < 100; attempt++) {
            if (await evaluate("return document.getElementById('sandbox-next').disabled")) break;
            await new Promise(resolve => setTimeout(resolve, 20));
        }
        assert.match(await evaluate("return document.getElementById('sandbox-after').textContent"), /Expression/);
        assert.equal(await evaluate("return document.getElementById('sandbox-next').disabled"), true);
        await evaluate("document.getElementById('sandbox-step').value = 2; document.getElementById('sandbox-step').dispatchEvent(new Event('change'))");
        for (let attempt = 0; attempt < 100; attempt++) {
            if (await evaluate("return /^Event 2 of/.test(document.getElementById('sandbox-position').textContent)")) break;
            await new Promise(resolve => setTimeout(resolve, 20));
        }
        assert.match(await evaluate("return document.getElementById('sandbox-position').textContent"), /^Event 2 of/);
    }
    await command(`/session/${session}/window/rect`, { width: 390, height: 844 });
    await evaluate("document.querySelector('#sandbox-before details').open = true");
    assert.ok(await evaluate("return document.documentElement.scrollWidth <= window.innerWidth + 1"));
    await command(`/session/${session}/window/rect`, { width: 1440, height: 1000 });
    await evaluate("document.getElementById('sandbox-trace').scrollIntoView({block: 'start', behavior: 'instant'})");
    if (process.env.TEST_UNDECLARED_OUTPUTS_DIR) {
        const screenshot = await command(`/session/${session}/screenshot`);
        await writeFile(resolve(process.env.TEST_UNDECLARED_OUTPUTS_DIR, 'sandbox.png'), Buffer.from(screenshot, 'base64'));
    }
    assert.deepEqual(await evaluate(`
        const target = document.createElement('pre');
        const source = '<img src=x onerror=alert(1)> [A] B';
        globalThis.syntax.highlight(target, source);
        return [target.textContent === source, target.querySelector('img') === null];
    `), [true, true]);
    await evaluate("document.getElementById('sandbox-input').value = '1/0'; document.getElementById('sandbox-run').click()");
    for (let attempt = 0; attempt < 450; attempt++) {
        if (await evaluate("return !document.getElementById('sandbox-run').disabled")) break;
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.match(await evaluate("return document.getElementById('sandbox-status').textContent"), /Division by zero/);
    assert.equal(await evaluate("return document.getElementById('sandbox-trace').hidden"), false);
    assert.equal(await evaluate("return document.getElementById('sandbox-result').textContent"), '');
    await evaluate("document.getElementById('sandbox-run').click(); document.getElementById('sandbox-stop').click()");
    assert.match(await evaluate("return document.getElementById('sandbox-status').textContent"), /^Stopped/);
    await command(`/session/${session}/window/rect`, { width: 390, height: 844 });
    assert.ok(await evaluate("return document.documentElement.scrollWidth <= window.innerWidth + 1"));
    await command(`/session/${session}/window/rect`, { width: 1440, height: 1000 });
    assert.deepEqual(await evaluate(`
        const identity = [...document.querySelectorAll('[id]')].map(value => value.id);
        return identity.filter((value, index) => identity.indexOf(value) !== index);
    `), []);
    assert.deepEqual(await evaluate(`return [...document.querySelectorAll('a[href^="#"]')].filter(link => link.hash.length > 1 && !document.getElementById(decodeURIComponent(link.hash.slice(1)))).map(link => link.hash)`), []);
    assert.equal(await evaluate("return document.querySelectorAll('#contents a').length === document.querySelectorAll('.chapter').length"), true);
    await evaluate("location.hash = 'guide-language-syntax'");
    for (let attempt = 0; attempt < 20; attempt++) {
        if (await evaluate("return document.getElementById('guide-language').open")) break;
        await new Promise(resolve => setTimeout(resolve, 20));
    }
    assert.equal(await evaluate("return document.getElementById('guide-language').open"), true);
    await command(`/session/${session}/window/rect`, { width: 390, height: 844 });
    await evaluate("document.querySelectorAll('.guide').forEach(value => { value.open = true; })");
    assert.ok(await evaluate("return document.documentElement.scrollWidth <= window.innerWidth + 1"));
    const message = await command(`/session/${session}/log`, { type: 'browser' });
    assert.deepEqual(message.filter(value => value.level === 'SEVERE'), []);
    assert.deepEqual(missing, []);
    console.log('Browser checks passed: theme persistence, arithmetic results, event navigation, reference exploration, binding deduplication, and assets.');
} finally {
    if (session) await fetch(`${endpoint}/session/${session}`, { method: 'DELETE', signal: AbortSignal.timeout(5000) }).catch(() => {});
    driver.kill();
    server.close();
    server.closeAllConnections();
}
