(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const value = item => typeof item === 'string'
        ? book.render.token({ label: item })
        : book.render.token({}, item.rule.name);

    const particle = (content, style = 'coherence') => {
        const node = element('div', content.length ? style : `${style} empty`);
        content.forEach(item => node.append(value(item)));
        return node;
    };

    const rule = definition => {
        const box = element('div', 'shape');
        const title = element('pre', 'code');
        book.syntax.highlight(title, definition.name);
        box.append(title);
        const input = element('div', 'row');
        input.append(element('span', 'side', 'consumes'));
        if (!definition.input.length) input.append(element('span', 'summary', 'nothing: zero input positions'));
        definition.input.forEach(content => input.append(particle(content)));
        const output = element('div', 'row');
        output.append(element('span', 'side', 'produces'));
        if (!definition.output.length) output.append(element('span', 'summary', 'no coherence'));
        box.append(input, output);
        definition.output.forEach(item => {
            output.append(particle(item.particle));
            if (!item.body) return;
            const body = element('div', 'body');
            body.append(element('span', 'side', 'scope with rules'));
            item.body.forEach(inner => body.append(rule(inner)));
            box.append(body);
        });
        return box;
    };

    const draw = (host, program) => {
        const result = element('div', 'lowered');
        result.append(element('h4', undefined, program.initial.length === 1 ? '1 coherence' : `${program.initial.length} coherences`));
        const row = element('div', 'row');
        program.initial.forEach(content => row.append(particle(content)));
        if (!program.initial.length) row.append(element('span', 'summary', 'none'));
        result.append(row);
        result.append(element('h4', undefined, program.rule.length === 1 ? '1 rule' : `${program.rule.length} rules`));
        program.rule.forEach(definition => result.append(rule(definition)));
        if (!program.rule.length) result.append(element('span', 'summary', 'none'));
        host.replaceChildren(result);
    };

    const enhance = widget => {
        const sample = JSON.parse(widget.dataset.lens);
        const bar = element('div', 'bar');
        bar.append(element('span', 'title', 'Syntax lens'), element('span', 'badge', 'what the source means'));
        const body = element('div', 'body');
        const preset = element('div', 'preset');
        const field = element('label', 'field');
        field.append('Photonic source');
        const input = element('input');
        input.spellcheck = false;
        input.autocomplete = 'off';
        field.append(input);
        const message = book.render.message();
        const output = element('div');
        body.append(preset, field, message.element, output);
        widget.replaceChildren(bar, body);
        let ticket = 0;
        const show = async source => {
            const mine = ++ticket;
            input.value = source;
            preset.querySelectorAll('button').forEach(button => button.setAttribute('aria-pressed', String(button.textContent === source)));
            const recorded = book.record?.lower?.[source];
            if (recorded) {
                message.say();
                draw(output, recorded.program);
                return;
            }
            if (book.engine.state !== 'live') {
                message.say('Serve the book to lower new source: bazel run -c opt //toolchain/browser:serve');
                return;
            }
            try {
                const result = await book.engine.send({ kind: 'lower', source });
                if (mine !== ticket) return;
                message.say();
                draw(output, result.program);
            } catch (error) {
                if (mine !== ticket) return;
                message.say(book.editor.describe(error), 'error');
            }
        };
        sample.forEach(source => {
            const button = element('button', undefined, source);
            button.type = 'button';
            button.addEventListener('click', () => show(source));
            preset.append(button);
        });
        let timer;
        input.addEventListener('input', () => {
            clearTimeout(timer);
            timer = setTimeout(() => show(input.value), 180);
        });
        book.engine.watch(state => {
            input.readOnly = state !== 'live';
            input.title = state === 'live' ? '' : 'Serve the book locally to type your own source.';
        });
        show(sample[0]);
    };

    document.querySelectorAll('.lens[data-lens]').forEach(enhance);
})();
