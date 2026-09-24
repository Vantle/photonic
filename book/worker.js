import initialize, { lower, explore, Path } from '../toolchain/browser/module/runtime.js';
import { decode } from './numeral.js';

const ready = initialize();
let path;

const value = state => {
    try {
        return decode(state);
    } catch (error) {
        return { error: error.message };
    }
};

const follow = (next, numeric) => {
    const progress = JSON.parse(next.run());
    if (progress.error) {
        next.free();
        return progress;
    }
    path?.free();
    path = next;
    if (numeric) progress.value = value(progress.state);
    return progress;
};

const refuse = message => ({ version: 1, error: { code: 'request', message } });

const answer = data => {
    if (data.kind === 'lower') return JSON.parse(lower(data.source));
    if (data.kind === 'explore') return JSON.parse(explore(JSON.stringify(data.request)));
    if (data.kind === 'path') return follow(new Path(JSON.stringify(data.request)), false);
    if (data.kind === 'expression') return follow(Path.expression(data.input), true);
    if (data.kind !== 'inspect') return refuse(`The engine does not know the request kind ${data.kind}.`);
    if (!path) return refuse('The engine restarted. Run the program again to step through it.');
    return JSON.parse(path.inspect(data.index));
};

self.onmessage = async ({ data }) => {
    try {
        await ready;
    } catch (error) {
        self.postMessage({ serial: data.serial, version: 1, error: { code: 'engine', message: error.message } });
        return;
    }
    try {
        self.postMessage({ serial: data.serial, ...answer(data) });
    } catch (error) {
        self.postMessage({ serial: data.serial, version: 1, error: { code: 'crash', message: error.message } });
    }
};
