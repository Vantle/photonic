(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element, tally } = book.render;

    const enhance = widget => {
        const sample = JSON.parse(widget.dataset.preset);
        const bar = element('div', 'bar');
        const badge = element('span', 'badge', 'recorded runs');
        bar.append(element('span', 'title', 'Expression evaluator'), badge);
        const body = element('div', 'body');
        const choice = book.render.preset(sample, index => evaluate(sample[index]));
        const form = element('form', 'row');
        const field = element('label', 'field');
        field.append('Base-three expression: digits 0 1 2, + − × ÷ (or - * /), parentheses');
        const input = element('input');
        input.spellcheck = false;
        input.autocomplete = 'off';
        field.append(input);
        const run = element('button', 'button primary', 'Evaluate');
        run.type = 'submit';
        const stop = element('button', 'button', 'Stop');
        stop.type = 'button';
        stop.hidden = true;
        form.append(field, run, stop);
        const result = element('div', 'result');
        const message = book.render.message();
        const tape = element('details');
        const summary = element('summary', undefined, 'The Photonic program this expression becomes');
        const code = element('pre', 'code');
        tape.append(summary, code);
        const trace = element('div');
        body.append(choice.element, form, result, message.element, tape, trace);
        widget.replaceChildren(bar, body);
        const follow = book.engine.path();
        const cycle = book.run.create({ trigger: run, stop, message, text: 'Evaluating in WebAssembly…' });
        const clear = () => {
            result.replaceChildren();
            code.replaceChildren();
            trace.replaceChildren();
        };
        const draw = (outcome, live) => {
            clear();
            if (outcome.answer.error) {
                message.say(outcome.answer.error, 'error');
            } else {
                message.say();
                const sign = outcome.answer.ternary.startsWith('-') ? '−' : '';
                result.append(element('strong', undefined, `${sign}${outcome.answer.ternary.replace('-', '')}₃`));
                result.append(element('span', undefined, `= ${sign}${outcome.answer.decimal.replace('-', '')} in decimal`));
            }
            const count = element('span', 'summary');
            count.append(tally(outcome.event, 'event'), ' · ', tally(outcome.work, 'work step'));
            result.append(count);
            book.syntax.highlight(code, outcome.source);
            if (!live) return;
            book.trace.draw(trace, outcome, { quiet: true, start: Math.max(0, outcome.event - 1) });
        };
        const choose = value => {
            input.value = value;
            choice.press(value);
        };
        const recorded = value => {
            const outcome = book.record?.expression?.[value];
            if (!outcome) return false;
            draw(outcome, false);
            return true;
        };
        const evaluate = value => {
            choose(value);
            if (book.engine.state !== 'live') {
                if (!recorded(value)) message.say('Serve the book to evaluate new expressions: bazel run -c opt //book:serve');
                return;
            }
            if (cycle.busy) cycle.cancel();
            cycle.start(signal => follow('expression', { source: value }, { timeout: 30000, signal }), outcome => draw(outcome, true), error => {
                clear();
                message.say(error.message, 'error');
            });
        };
        form.addEventListener('submit', event => {
            event.preventDefault();
            if (!cycle.busy) evaluate(input.value);
        });
        book.engine.watch(state => {
            const on = state === 'live';
            badge.textContent = on ? 'live · Photonic in WebAssembly' : 'recorded runs';
            input.readOnly = !on;
            run.hidden = !on;
        });
        choose(sample.at(-1));
        recorded(sample.at(-1));
    };

    document.querySelectorAll('.calculator[data-preset]').forEach(enhance);
})();
