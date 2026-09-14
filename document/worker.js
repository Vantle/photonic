import initialize, { execute } from '../browser/module/runtime.js';

self.onmessage = async ({ data }) => {
    try {
        await initialize();
        const request = JSON.stringify({ version: 1, source: data.source, targets: data.targets });
        if (new TextEncoder().encode(request).length > 32768) throw new Error('Keep the source and targets together below 32 KiB.');
        self.postMessage(JSON.parse(execute(request)));
    } catch (error) {
        self.postMessage({ version: 1, error: { code: 'worker', message: error.message } });
    }
};
