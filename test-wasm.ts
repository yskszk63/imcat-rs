import WASI from "https://deno.land/std@0.106.0/wasi/snapshot_preview1.ts";

//deno run --unstable --allow-run --allow-read ./test-wasm.ts < assets/unko.png
const proc = Deno.run({
    cmd: ['cargo', '+nightly', 'wasi', 'build', '--lib'],
});
try {
    const status = await proc.status();
    if (!status.success) {
        throw new Error("failed to run cargo.");
    }
} finally {
    proc.close();
}

const wasi = new WASI({});
const binary = await Deno.readFile("./target/wasm32-wasi/debug/imcat_rs.wasm");
const module = await WebAssembly.compile(binary);
const instance = await WebAssembly.instantiate(module, {
  wasi_snapshot_preview1: wasi.exports,
});
wasi.initialize(instance);
const { columns, rows } = await Deno.consoleSize(Deno.stdout.rid);
(instance.exports['wasi_imcat'] as Function)(0, 1, columns, rows);
