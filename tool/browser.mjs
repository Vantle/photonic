import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, resolve, sep } from 'node:path';

import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const missing = [];
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' };
const server = createServer(async (request, response) => {
    const path = resolve(root, `.${new URL(request.url, 'http://localhost').pathname}`);
    if (!path.startsWith(root + sep)) {
        response.writeHead(403).end();
        return;
    }
    try {
        const content = await readFile(path);
        response.writeHead(200, { 'Content-Type': mime[extname(path)] ?? 'text/plain' }).end(content);
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
    await navigate('/document/plan.html', "return document.getElementById('state')?.children.length > 0");
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
