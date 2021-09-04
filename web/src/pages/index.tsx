import Head from 'next/head'
import styles from '../styles/Home.module.css'
import dynamic from "next/dynamic";
import { useCallback, useEffect, useMemo, useState } from 'react';
import "xterm/css/xterm.css";
import { WASI } from "@wasmer/wasi"
import { WasmFs } from "@wasmer/wasmfs";

const DynamicTerminal = dynamic(() => import("../lib/terminal"), {
  ssr: false
});

export default function Home() {
  const Terminal = useMemo(() => DynamicTerminal, []);

  const [rxtx, setRxtx] = useState<{rx?: ReadableStreamDefaultReader, tx?: Function}>({});
  const [wasm, setWasm] = useState<WebAssembly.Module|undefined>();
  const [[cols, rows], ] = [[88, 30], ];
  const prompt = "$ ";

  useEffect(() => {
    let controller: ReadableStreamDefaultController | null = null;
    const rx = new ReadableStream({
      start(c) {
        controller = c;
      }
    });
    const tx = (chunk: string) => {
        if (controller !== null) {
          controller.enqueue(chunk);
        }
      };
    setRxtx({rx: rx.getReader(), tx, });
    tx(prompt);
  }, []);

  useEffect(() => {
    (async () => {
        const module = await WebAssembly.compileStreaming(fetch("imcat_rs.wasm"));
        setWasm(module);
    })();
  }, []);

  const oninput = useCallback(async (evt) => {
    if (evt.target.files.length && rxtx && rxtx.tx && wasm) {
        const [file] = evt.target.files;

        const fs = new WasmFs();
        const wasi = new WASI({
            args: [],
            env: {},
            bindings: {
                ...WASI.defaultBindings,
                fs: fs.fs,
            }
        });
        const instance = await WebAssembly.instantiate(wasm, {
            ...wasi.getImports(wasm),
        });
        wasi.start(instance);

        const unkoResponse = await fetch(URL.createObjectURL(file));
        if (unkoResponse.status != 200) {
            throw new Error("failed to fetch unko");
        }
        const unkoPng = await unkoResponse.arrayBuffer();

        await new Promise(resolve => {
            fs.fs.writeFile("/dev/stdin", new Uint8Array(unkoPng), resolve);
        });
        (instance.exports.wasi_imcat as Function)(0, 1, cols, rows);
        rxtx.tx("imcat_rs -\r\n");

        const data = await fs.getStdOut();
        rxtx.tx(data);
        rxtx.tx("\r\n");
        const data2 = fs.fs.readFileSync("/dev/stderr", "utf8");
        if (data2) {
            rxtx.tx(data2);
        }
        rxtx.tx(prompt);
    }
  }, [rxtx, wasm, cols, rows]);

  return (
    <div className={styles.container}>
      <Head>
        <title>imcat-rs</title>
        <meta name="description" content="imcat-rs web demo." />
        <link rel="icon" href="data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text x=%2250%%22 y=%2250%%22 style=%22dominant-baseline:central;text-anchor:middle;font-size:90px;%22>○</text></svg>" />
        <meta property="og:description" content="imcat Clone. Implemented by Rust. " />
        <meta property="og:title" content="imcat-rs demo." />
      </Head>

      <main className={styles.main}>
        <label>$ imcat_rs <input type="file" onInput={oninput} accept="image/*"/></label>
        <Terminal input={rxtx.rx} cols={cols} rows={rows}/>
      </main>
    </div>
  )
}
