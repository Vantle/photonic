import initialize, { Evaluation } from '../toolchain/browser/module/runtime.js';
import { decode } from './numeral.js';
let session;
self.onmessage = async ({ data }) => {
    try {
        await initialize();
        if (data.kind === 'inspect') {
            if (!session) throw Error('Run an expression first.');
            const reply = JSON.parse(session.inspect(data.index));
            if (reply.error) throw Error(reply.error.message);
            self.postMessage({ kind: 'inspect', ...reply });
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
        self.postMessage({ kind: data.kind, error: error.message ?? String(error) });
    }
};
