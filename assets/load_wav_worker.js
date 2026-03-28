import init, { run_load_wav_worker } from "./wavalyze.js";

let wasmReady;

async function ensureWasm() {
    if (!wasmReady) {
        wasmReady = init();
    }
    await wasmReady;
}

self.onmessage = async (event) => {
    try {
        await ensureWasm();
        const result = run_load_wav_worker(event.data);
        self.postMessage(result);
    } catch (error) {
        self.postMessage({
            kind: "failed",
            error: error instanceof Error ? error.message : String(error),
        });
    }
};
