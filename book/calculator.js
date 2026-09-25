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
        const preset = element('div', 'preset');
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
        body.append(preset, form, result, message.element, tape, trace);
        widget.replaceChildren(bar, body);
        const channel = book.engine.open();
        const clear = () => {
            result.replaceChildren();
            code.replaceChildren();
            trace.replaceChildren();
        };
        const draw = (outcome, live) => {
            clear();
            if (outcome.value?.error) {
                message.say(outcome.value.error, 'error');
            } else {
                message.say();
                const sign = outcome.value.ternary.startsWith('-') ? '−' : '';
                result.append(element('strong', undefined, `${sign}${outcome.value.ternary.replace('-', '')}₃`));
                result.append(element('span', undefined, `= ${sign}${outcome.value.decimal.replace('-', '')} in decimal`));
            }
            const count = element('span', 'summary');
            count.append(tally(outcome.event, 'event'), ' · ', tally(outcome.work, 'work step'));
            result.append(count);
            book.syntax.highlight(code, outcome.source);
            if (!live) return;
            book.trace.draw(trace, {
                quiet: true,
                count: outcome.event,
                work: outcome.work,
                definition: outcome.definition,
                start: Math.max(0, outcome.event - 1),
                get: index => channel.send({ kind: 'inspect', index }),
            });
        };
        const choose = value => {
            input.value = value;
            preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === value)));
        };
        const recorded = value => {
            const outcome = book.record?.expression?.[value];
            if (!outcome) return false;
            draw(outcome, false);
            return true;
        };
        let ticket = 0;
        let busy = false;
        const evaluate = async value => {
            choose(value);
            if (book.engine.state !== 'live') {
                if (!recorded(value)) message.say('Serve the book to evaluate new expressions: bazel run -c opt //toolchain/browser:serve');
                return;
            }
            const mine = ++ticket;
            if (busy) channel.stop();
            busy = true;
            run.setAttribute('aria-disabled', 'true');
            stop.hidden = false;
            message.wait('Evaluating in WebAssembly…');
            try {
                const outcome = await channel.send({ kind: 'expression', input: value }, 30000);
                if (mine === ticket) draw(outcome, true);
            } catch (error) {
                if (mine !== ticket) return;
                clear();
                message.say(error.message, 'error');
            } finally {
                if (mine === ticket) {
                    busy = false;
                    run.removeAttribute('aria-disabled');
                    stop.hidden = true;
                }
            }
        };
        sample.forEach(value => {
            const button = element('button', undefined, value);
            button.type = 'button';
            button.addEventListener('click', () => evaluate(value));
            preset.append(button);
        });
        form.addEventListener('submit', event => {
            event.preventDefault();
            if (!busy) evaluate(input.value);
        });
        stop.addEventListener('click', () => channel.stop());
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
