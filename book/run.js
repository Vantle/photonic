(() => {
    'use strict';
    const book = globalThis.book ??= {};
    const { element } = book.render;

    const button = () => {
        const node = element('button', 'run', 'Run');
        node.type = 'button';
        node.hidden = true;
        node.title = 'Run (⌘ or Ctrl + Enter)';
        return node;
    };

    const shortcut = (area, action) => area.addEventListener('keydown', event => {
        if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
        event.preventDefault();
        action();
    });

    const create = ({ trigger, stop, message, text = 'Running in WebAssembly…' }) => {
        let ticket = 0;
        let busy = false;
        let control = new AbortController();
        const settle = () => {
            busy = false;
            trigger.removeAttribute('aria-disabled');
            if (stop) stop.hidden = true;
        };
        const cancel = () => {
            ticket++;
            control.abort();
            settle();
        };
        const start = async (task, show, fail) => {
            if (trigger.hidden || busy) return;
            const mine = ++ticket;
            control = new AbortController();
            busy = true;
            trigger.setAttribute('aria-disabled', 'true');
            if (stop) stop.hidden = false;
            message.wait(text);
            try {
                const result = await task(control.signal);
                if (mine !== ticket) return;
                message.say();
                show(result);
            } catch (error) {
                if (mine === ticket) fail(error);
            } finally {
                if (mine === ticket) settle();
            }
        };
        stop?.addEventListener('click', () => control.abort());
        return {
            start,
            cancel,
            get busy() {
                return busy;
            },
        };
    };

    book.run = { button, shortcut, create };
})();
