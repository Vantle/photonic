import initialize, { lower, explore, shape, select, Path } from '../toolchain/browser/module/runtime.js';
import { answer } from './numeral.js';

let live = false;
const ready = initialize().then(() => {
    live = true;
    self.postMessage({ ready: true });
}, error => self.postMessage({ failure: { code: 'engine', message: error.message } }));
let path;

const follow = (next, numeric) => {
    const progress = JSON.parse(next.run());
    if (progress.error) {
        next.free();
        return progress;
    }
    path?.free();
    path = next;
    if (numeric) progress.answer = answer(progress.state, progress.definition);
    return progress;
};

const perform = ({ kind, request }) => {
    if (kind === 'inspect') {
        if (!path) return { failure: { code: 'request', message: 'The engine restarted. Run the program again to step through it.' } };
        return { reply: JSON.parse(path.inspect(request.index)) };
    }
    const input = JSON.stringify(request);
    if (kind === 'lower') return { reply: JSON.parse(lower(input)) };
    if (kind === 'explore') return { reply: JSON.parse(explore(input)) };
    if (kind === 'shape') return { reply: JSON.parse(shape(input)) };
    if (kind === 'select') return { reply: JSON.parse(select(input)) };
    if (kind === 'path') return { reply: follow(new Path(input), false) };
    if (kind === 'expression') return { reply: follow(Path.expression(input), true) };
    return { failure: { code: 'request', message: `The engine does not know the request kind ${kind}.` } };
};

self.onmessage = async ({ data }) => {
    await ready;
    if (!live) return;
    try {
        self.postMessage({ serial: data.serial, ...perform(data) });
    } catch (error) {
        self.postMessage({ serial: data.serial, failure: { code: 'crash', message: error.message } });
    }
};
