(() => {
    'use strict';
    const book = globalThis.book ??= {};
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
        let serial = 0;
        let watchdog;
        const pending = new Map();
        const arm = () => {
            clearTimeout(watchdog);
            const [head] = pending.values();
            if (head) watchdog = setTimeout(() => halt(`Stopped after ${head.timeout / 1000} seconds without a result.`), head.timeout);
        };
        const halt = message => {
            clearTimeout(watchdog);
            worker?.terminate();
            worker = undefined;
            const waiting = [...pending.values()];
            pending.clear();
            waiting.forEach(({ reject }) => reject(new Error(message)));
        };
        const start = () => {
            worker = new Worker('book/worker.js', { type: 'module' });
            worker.onmessage = ({ data }) => {
                const waiting = pending.get(data.serial);
                if (!waiting) return;
                if (data.error?.code === 'engine') {
                    halt(`The WebAssembly engine did not start: ${data.error.message}`);
                    announce('recorded');
                    return;
                }
                if (data.error?.code === 'crash') {
                    halt(`The engine stopped with ${data.error.message}, usually because the program ran out of memory. The next run starts a fresh engine.`);
                    return;
                }
                pending.delete(data.serial);
                arm();
                announce('live');
                if (data.error) waiting.reject(Object.assign(new Error(data.error.message), { detail: data.error }));
                else waiting.resolve(data);
            };
            worker.onerror = event => {
                event.preventDefault();
                halt('The WebAssembly engine did not load. Serve the book with bazel run -c opt //toolchain/browser:serve.');
                announce('recorded');
            };
        };
        const send = (message, timeout = 20000) => {
            if (!served) return Promise.reject(new Error('Live execution needs the local server: bazel run -c opt //toolchain/browser:serve'));
            if (!worker) start();
            const number = ++serial;
            return new Promise((resolve, reject) => {
                pending.set(number, { resolve, reject, timeout });
                if (pending.size === 1) arm();
                worker.postMessage({ ...message, serial: number });
            });
        };
        return { send, stop: () => halt('Stopped.') };
    };

    const watch = callback => {
        listener.add(callback);
        callback(state);
    };

    let shared;
    const send = (message, timeout) => (shared ??= open()).send(message, timeout);

    const request = (setting, source, target = setting.target) => ({
        version: 1,
        source,
        library: setting.library.map(name => {
            const text = book.record?.library?.[name];
            if (text === undefined) throw new Error(`${name}.particle is not recorded. Regenerate the records with bazel run -c opt //book:record.`);
            return { name: `${name}.particle`, source: text };
        }),
        target,
        preserve: setting.preserve,
    });

    const explore = async (setting, source, target) => send({ kind: 'explore', request: request(setting, source, target) });

    if (served) {
        const probe = () => send({ kind: 'lower', source: 'A' }, 30000).catch(() => announce('recorded'));
        if ('requestIdleCallback' in globalThis) requestIdleCallback(probe, { timeout: 3000 });
        else setTimeout(probe, 1200);
    }

    book.engine = { open, send, request, explore, watch, get state() { return state; } };
})();
