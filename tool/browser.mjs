import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, resolve, sep } from 'node:path';

import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const missing = [];
const type = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
const server = createServer(async (request, response) => {
    const path = resolve(root, `.${new URL(request.url, 'http://localhost').pathname}`);
    if (!path.startsWith(root + sep)) {
        response.writeHead(403).end();
        return;
    }
    try {
        const content = await readFile(path === resolve(root, "browser/module/runtime_bg.wasm") ? process.argv[4] : path === resolve(root, "browser/module/runtime.js") ? process.argv[5] : path);
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
    await navigate('/document/reference.html', "return document.getElementById('state')?.children.length > 0");
    assert.ok(await evaluate("return document.getElementById('example').options.length > 0"), JSON.stringify(await command(`/session/${session}/log`, { type: 'browser' })));
    await evaluate("document.getElementById('explore').click()");
    assert.ok(await evaluate("return document.getElementById('event').children.length > 0"));
    await evaluate("document.getElementById('arrival').click(); document.getElementById('second').click(); document.getElementById('second').click()");
    assert.match(await evaluate("return document.getElementById('pulse').textContent"), /3 arrivals · 1 completed bindings/);
    await evaluate("document.getElementById('clear').click()");
    assert.match(await evaluate("return document.getElementById('pulse').textContent"), /0 arrivals · 0 completed bindings/);
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
