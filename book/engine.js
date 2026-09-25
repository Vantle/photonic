(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const version = 2;
    const stale = 'This page and its engine come from different versions of the book. Reload the page.';
    const served = /^https?:$/.test(location.protocol);
    const listener = new Set();
    let state = served ? 'unknown' : 'recorded';

    const announce = value => {
        if (state === value) return;
        state = value;
        listener.forEach(callback => callback(value));
    };

    const open = () => {
        let worker;
        let flight;
        let serial = 0;
        let watchdog;
        const pending = new Map();
        const dispatch = () => {
            if (flight !== undefined) return;
            const [number] = pending.keys();
            if (number === undefined) return;
            const entry = pending.get(number);
            flight = number;
            worker ??= start();
            worker.postMessage({ serial: number, kind: entry.kind, request: entry.request });
            watchdog = setTimeout(() => halt(number, `Stopped after ${entry.timeout / 1000} seconds without a result.`), entry.timeout);
        };
        const land = () => {
            clearTimeout(watchdog);
            const entry = pending.get(flight);
            pending.delete(flight);
            flight = undefined;
            return entry;
        };
        const abandon = message => {
            clearTimeout(watchdog);
            worker?.terminate();
            worker = undefined;
            flight = undefined;
            const waiting = [...pending.values()];
            pending.clear();
            waiting.forEach(({ reject }) => reject(new Error(message)));
        };
        const halt = (number, message) => {
            if (number !== flight) return;
            const { reject } = land();
            worker.terminate();
            worker = undefined;
            dispatch();
            reject(new Error(message));
        };
        const withdraw = number => {
            if (number === flight) {
                halt(number, 'Stopped.');
                return;
            }
            const entry = pending.get(number);
            if (!entry) return;
            pending.delete(number);
            entry.reject(new Error('Stopped.'));
        };
        const start = () => {
            const current = new Worker('book/worker.js', { type: 'module' });
            current.onmessage = ({ data }) => {
                if (current !== worker || data.serial !== flight) return;
                if (data.failure?.code === 'engine') {
                    abandon(`The WebAssembly engine did not start: ${data.failure.message}`);
                    announce('recorded');
                    return;
                }
                if (data.failure?.code === 'crash') {
                    halt(flight, `The engine stopped with ${data.failure.message}, usually because the program ran out of memory. The next run starts a fresh engine.`);
                    return;
                }
                if (!data.failure && data.reply?.version !== version) {
                    abandon(stale);
                    announce('stale');
                    return;
                }
                const { resolve, reject } = land();
                dispatch();
                announce('live');
                if (data.failure) reject(new Error(data.failure.message));
                else if (data.reply.error) reject(Object.assign(new Error(data.reply.error.message), { detail: data.reply.error }));
                else resolve(data.reply);
            };
            current.onerror = event => {
                event.preventDefault();
                if (current !== worker) return;
                abandon('The WebAssembly engine did not load. Serve the book with bazel run -c opt //toolchain/browser:serve.');
                announce('recorded');
            };
            return current;
        };
        const send = (kind, body, { timeout = 20000, signal } = {}) => {
            if (!served) return Promise.reject(new Error('Live execution needs the local server: bazel run -c opt //toolchain/browser:serve'));
            if (signal?.aborted) return Promise.reject(new Error('Stopped.'));
            const number = ++serial;
            return new Promise((resolve, reject) => {
                pending.set(number, { kind, request: { version, ...body }, timeout, resolve, reject });
                signal?.addEventListener('abort', () => withdraw(number), { once: true });
                dispatch();
            });
        };
        return { send };
    };

    const watch = callback => {
        listener.add(callback);
        callback(state);
    };

    let shared;
    const send = (kind, body, option) => (shared ??= open()).send(kind, body, option);

    const request = (setting, source, target) => ({
        source,
        library: setting.library.map(name => {
            const text = book.record?.library?.[name];
            if (text === undefined) throw new Error(`${name}.particle is not recorded. Regenerate the records with bazel run -c opt //book:record.`);
            return { name: `${name}.particle`, source: text };
        }),
        target,
        preserve: setting.preserve,
    });

    const explore = async (setting, source, target, signal) => send('explore', request(setting, source, target), { signal });

    if (served) {
        const probe = () => send('lower', { source: 'A' }, { timeout: 30000 }).catch(() => {
            if (state === 'unknown') announce('recorded');
        });
        if ('requestIdleCallback' in globalThis) requestIdleCallback(probe, { timeout: 3000 });
        else setTimeout(probe, 1200);
    }

    book.engine = { open, send, request, explore, watch, get state() { return state; } };
})();
