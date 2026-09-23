import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const [binary, javascript, numeral, input = '2*2*2*2*2*2', sample = '5'] = process.argv.slice(2);
const engine = await import(pathToFileURL(javascript));
const { decode } = await import(pathToFileURL(numeral));
engine.initSync({ module: await readFile(binary) });

function evaluate() {
    const start = performance.now();
    const session = new engine.Evaluation(input);
    const initialized = performance.now();
    const result = JSON.parse(session.run());
    const executed = performance.now();
    const value = decode(result.state).ternary;
    const inspecting = performance.now();
    const transition = JSON.parse(session.inspect(result.event - 1));
    if (transition.error) throw new Error(transition.error.message);
    const inspected = performance.now();
    session.free();
    return {
        initialization: (initialized - start) / 1000,
        execution: (executed - initialized) / 1000,
        inspection: (inspected - inspecting) / 1000,
        release: (performance.now() - inspected) / 1000,
        value,
        event: result.event,
        work: result.work,
    };
}

evaluate();
const measurement = Array.from({ length: Number(sample) }, evaluate);
console.log(JSON.stringify({ input, measurement }, null, 2));
