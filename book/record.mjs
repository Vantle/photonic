import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [javascript, webassembly, numeral, index, lightbox, output, mode] = process.argv.slice(2);
assert.ok(mode === 'write' || mode === 'check', 'record.mjs runs in write or check mode');
const workspace = process.env.BUILD_WORKSPACE_DIRECTORY;
assert.ok(mode === 'check' || workspace, 'Write the record with bazel run -c opt //book:record.');
const root = mode === 'write' ? workspace : dirname(index);
const engine = await import(pathToFileURL(javascript));
const { decode } = await import(pathToFileURL(numeral));
engine.initSync({ module: await readFile(webassembly) });

const entity = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", '#39': "'" };
const unescape = text => text.replace(/&(amp|lt|gt|quot|apos|#39);/g, (_, name) => entity[name]);
const page = await readFile(mode === 'write' ? join(workspace, 'index.html') : index, 'utf8');
const pair = /([^\s=>/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
const parse = text => [...text.matchAll(/<([a-z][a-z0-9]*)((?:\s+[^\s=>/]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'=<>`]+))?)*)\s*\/?>/g)].map(match => ({
    name: match[1],
    attribute: new Map([...match[2].matchAll(pair)].map(value => [value[1], unescape(value[2] ?? value[3] ?? value[4] ?? '')])),
    start: match.index,
    end: match.index + match[0].length,
}));
const tag = parse(page);
const style = (item, name) => (item.attribute.get('class') ?? '').split(/\s+/).includes(name);
const content = item => {
    const body = page.slice(item.end, page.indexOf(`</${item.name}>`, item.end));
    return unescape(body.startsWith('\n') ? body.slice(1) : body);
};
const file = async name => {
    try {
        return await readFile(join(root, name), 'utf8');
    } catch {
        throw new Error(`${name} is not available; add it to the data of //book:record.check.`);
    }
};
const invoke = (name, response) => {
    const value = JSON.parse(response);
    if (value.error) throw new Error(`${name}: ${value.error.code}: ${value.error.message}`);
    delete value.version;
    return value;
};

const library = Object.create(null);
const example = Object.create(null);
const lower = Object.create(null);
const expression = Object.create(null);
const workbench = Object.create(null);
const connection = Object.create(null);
const preset = new Set();

const setting = async item => {
    const value = {
        library: (item.attribute.get('data-library') ?? '').split(/\s+/).filter(Boolean),
        target: JSON.parse(item.attribute.get('data-target') ?? '[]'),
        preserve: item.attribute.has('data-preserve'),
    };
    for (const name of value.library) library[name] ??= await file(`${name}.particle`);
    return value;
};
const request = (value, source) => JSON.stringify({
    version: 1,
    source,
    library: value.library.map(name => ({ name: `${name}.particle`, source: library[name] })),
    target: value.target,
    preserve: value.preserve,
});

for (const item of tag.filter(value => value.attribute.has('data-example'))) {
    const name = item.attribute.get('data-example');
    assert.ok(item.name === 'figure' && style(item, 'example'), `example ${name} must be a figure with the example class`);
    assert.ok(!(name in example), `duplicate example ${name}`);
    const close = page.indexOf('</figure>', item.end);
    const block = tag.find(value => value.name === 'pre' && value.start > item.end && value.start < close);
    assert.ok(block, `example ${name} needs a pre block`);
    const source = content(block);
    const origin = item.attribute.get('data-source');
    if (origin) assert.equal(source.trimEnd(), (await file(origin)).trimEnd(), `example ${name} must show ${origin} verbatim`);
    const value = await setting(item);
    const expect = JSON.parse(item.attribute.get('data-expect') ?? '[]');
    if (item.attribute.get('data-mode') === 'path') {
        const path = new engine.Path(request(value, source));
        const progress = invoke(name, path.run());
        const step = [];
        const state = [];
        for (let position = 0; position < progress.event; position++) {
            const inspection = invoke(name, path.inspect(position));
            step.push(inspection.event);
            state[inspection.event.source] = inspection.before;
            state[inspection.event.target] = inspection.after;
        }
        path.free();
        if (expect.length) assert.deepEqual([progress.outcome], expect, `example ${name} outcome`);
        example[name] = { source, ...value, result: { outcome: progress.outcome, event: progress.event, work: progress.work, definition: progress.definition, step, state } };
        continue;
    }
    const result = invoke(name, engine.explore(request(value, source)));
    if (expect.length) assert.deepEqual(result.verdict.map(verdict => verdict.outcome), expect, `example ${name} verdicts`);
    example[name] = { source, ...value, result };
}

for (const item of tag.filter(value => value.name === 'pre')) {
    const listing = content(item);
    const origin = item.attribute.get('data-source');
    if (origin) assert.equal(listing.trimEnd(), (await file(origin)).trimEnd(), `the listing of ${origin} must match it verbatim`);
    const excerpt = item.attribute.get('data-excerpt');
    if (excerpt) assert.ok((await file(excerpt)).includes(listing), `the listing from ${excerpt} must appear in it verbatim`);
}

for (const item of tag.filter(value => style(value, 'lens') && value.attribute.has('data-lens'))) {
    for (const source of JSON.parse(item.attribute.get('data-lens'))) {
        lower[source] = { program: invoke(`lens ${source}`, engine.lower(source)).program };
    }
}

for (const item of tag.filter(value => style(value, 'calculator') && value.attribute.has('data-preset'))) {
    for (const input of JSON.parse(item.attribute.get('data-preset'))) {
        const path = engine.Path.expression(input);
        const progress = invoke(`expression ${input}`, path.run());
        path.free();
        let value;
        try {
            value = decode(progress.state);
        } catch (error) {
            value = { error: error.message };
        }
        expression[input] = { value, event: progress.event, work: progress.work, source: progress.source };
    }
}

for (const item of tag.filter(value => style(value, 'workbench') && value.attribute.has('data-preset'))) {
    for (const entry of JSON.parse(item.attribute.get('data-preset'))) {
        assert.ok(!preset.has(entry.name), `duplicate workbench preset ${entry.name}`);
        preset.add(entry.name);
        if (entry.example) {
            assert.ok(example[entry.example]?.result.execution, `workbench preset ${entry.name} must name an explored example`);
            continue;
        }
        const value = { library: entry.library ?? [], target: entry.target ?? [], preserve: entry.preserve ?? false };
        for (const name of value.library) library[name] ??= await file(`${name}.particle`);
        const result = invoke(`workbench ${entry.name}`, engine.explore(request(value, entry.source)));
        assert.ok(result.verdict.every(verdict => verdict.outcome === 'reached'), `workbench preset ${entry.name} must reach its targets`);
        workbench[entry.name] = { source: entry.source, ...value, result };
    }
}

for (const item of tag.filter(value => style(value, 'connection') && value.attribute.has('data-preset'))) {
    for (const entry of JSON.parse(item.attribute.get('data-preset'))) {
        assert.ok(!(entry.name in connection), `duplicate connection preset ${entry.name}`);
        const program = [];
        for (const member of entry.program) program.push({ field: member.field, source: member.file ? await file(member.file) : member.source });
        const result = invoke(`connection ${entry.name}`, engine.compare(JSON.stringify({ version: 1, program: program.map(member => member.source) })));
        assert.equal(result.shape.length, entry.shape, `connection preset ${entry.name} must find ${entry.shape} shapes`);
        connection[entry.name] = { program, result };
    }
}

const playground = parse(await readFile(mode === 'write' ? join(workspace, 'lightbox.html') : lightbox, 'utf8'));
for (const item of playground.filter(value => style(value, 'lightbox') && value.attribute.has('data-preset'))) {
    for (const entry of JSON.parse(item.attribute.get('data-preset'))) {
        if (entry.example) assert.ok(example[entry.example]?.result.execution, `Lightbox preset ${entry.name} must name an explored example`);
        else assert.ok(entry.workbench in workbench, `Lightbox preset ${entry.name} must name a workbench preset`);
    }
}

const section = (name, value) => {
    assert.ok(!('__proto__' in value), `${name} cannot record a key named __proto__`);
    return `"${name}": {\n__proto__: null,\n${Object.entries(value).map(([key, item]) => `${JSON.stringify(key)}: ${JSON.stringify(item)}`).join(',\n')}\n}`;
};
const record = `globalThis.book ??= {};\nglobalThis.book.record = {\n${[
    section('library', library),
    section('example', example),
    section('lower', lower),
    section('expression', expression),
    section('workbench', workbench),
    section('connection', connection),
].join(',\n')}\n};\n`;
const summary = `${Object.keys(example).length} examples, ${Object.keys(lower).length} lowerings, ${Object.keys(expression).length} expressions, ${Object.keys(workbench).length} workbench programs and ${Object.keys(connection).length} comparisons`;

if (mode === 'write') {
    await writeFile(join(workspace, 'book', 'record.js'), record);
    console.log(`Recorded ${summary} in book/record.js.`);
} else if (await readFile(output, 'utf8') !== record) {
    console.error('book/record.js does not match the book. Regenerate it with: bazel run -c opt //book:record');
    process.exitCode = 1;
} else {
    console.log(`book/record.js matches ${summary}.`);
}
