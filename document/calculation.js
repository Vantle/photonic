import initialize, { Evaluation } from '../browser/module/runtime.js';
import { decode } from './numeral.js';
let session;
self.onmessage = async ({ data }) => {
    try {
        await initialize();
        if (data.kind === 'inspect') {
            if (!session) throw Error('Run an expression first.');
            self.postMessage({ kind: 'inspect', ...JSON.parse(session.inspect(data.index)) });
            return;
        }
        session?.free();
        session = undefined;
        session = new Evaluation(data.input);
        const result = JSON.parse(session.run());
        let value;
        try { value = decode(result.state); } catch (error) { value = { error: error.message }; }
        self.postMessage({ kind: 'run', ...value, work: result.work, event: result.event, source: result.source });
    } catch (error) {
        let message = error.message ?? String(error);
        try { message = JSON.parse(message).message; } catch {}
        self.postMessage({ kind: data.kind, error: message });
    }
};
