import React, { RefObject } from "react";
import { Terminal } from "xterm";
import FontFaceObserver from 'fontfaceobserver';

interface Props {
    input?: ReadableStreamDefaultReader,
    cols?: number,
    rows?: number,
}

interface State {
    div: RefObject<HTMLDivElement>;
    term: Terminal,
    input?: ReadableStreamDefaultReader,
}

class TerminalComponent extends React.Component<Props, State> {
    constructor(props: Props) {
        super(props);

        const term = new Terminal({
            cols: props.cols || 80,
            rows: props.rows || 20,
            fontFamily: 'IBM Plex Mono',
            convertEol: true,
        });
        this.state = {
            div: React.createRef(),
            term,
            input: props.input,
        }
    }

    async componentDidMount() {
        const { term, input, div } = this.state as State;
        const element = div.current;
        if (element === null) {
            throw new Error("no element found.");
        }
        if (input == null) {
            throw new Error("no input initialized.");
        }

        // ref https://github.com/CoderPad/xterm-webfont/blob/master/src/index.js
        const font = term.getOption("fontFamily");
        const regular = new FontFaceObserver(font).load();
        const bold = new FontFaceObserver(font, { weight: 'bold' }).load();
        await Promise.all([regular, bold]);

        term.open(element);
        term.focus();

        (async () => {
            while (true) {
                const { value, done } = await input.read();
                if (done) {
                    break;
                }
                if (value) {
                    term.write(value);
                }
            }
        })();
    }

    render() {
        return <div ref={this.state.div}></div>;
    }
}

export default TerminalComponent;
