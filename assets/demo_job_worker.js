const MASK = (1n << 64n) - 1n;

function rotl64(value, shift) {
    const bits = BigInt(Number(shift % 64n));
    if (bits === 0n) {
        return value & MASK;
    }
    return ((value << bits) & MASK) | (value >> (64n - bits));
}

function demoStageName(stageIx, total) {
    if (stageIx === 0) {
        return "preparing";
    }
    if (stageIx + 1 === total) {
        return "finalizing";
    }
    return `processing ${stageIx + 1}`;
}

function computeDemoStage(stageIx, workUnits, seed) {
    const iterations = Math.max(workUnits, 10_000);
    let value = (BigInt(seed) ^ (BigInt(stageIx) << 32n)) & MASK;
    for (let ix = 0; ix < iterations; ix += 1) {
        const step = BigInt(ix);
        value = (value * 1664525n + 1013904223n + rotl64(step, BigInt((stageIx % 13) + 1))) & MASK;
        value ^= value >> 17n;
        value = rotl64(value, BigInt((ix % 23) + 1));
    }
    return value & MASK;
}

self.onmessage = (event) => {
    const { jobId, stageCount, workUnits } = event.data;

    try {
        let checksum = 0n;
        for (let stageIx = 0; stageIx < stageCount; stageIx += 1) {
            const stageLabel = demoStageName(stageIx, stageCount);
            self.postMessage({
                kind: "progress",
                stageLabel,
                current: stageIx,
                total: stageCount,
                message: `running ${stageLabel}`,
            });

            checksum ^= computeDemoStage(stageIx, workUnits, jobId);

            self.postMessage({
                kind: "progress",
                stageLabel,
                current: stageIx + 1,
                total: stageCount,
                message: `completed stage ${stageIx + 1}`,
            });
        }

        self.postMessage({
            kind: "completed",
            stageCount,
            checksum: checksum.toString(),
        });
    } catch (error) {
        self.postMessage({
            kind: "failed",
            error: error instanceof Error ? error.message : String(error),
        });
    }
};
