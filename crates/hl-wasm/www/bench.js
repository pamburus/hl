// In-browser benchmark for the hl WASM renderer.
//
// Mirrors what the native `hl-bench.sh` does for the CLI: time a tight loop that runs
// format_line on every record of a log file, repeat N times, and report best/median/worst
// wall time alongside throughput. The work runs on the main thread, not in the viewer's
// worker, so the numbers reflect pure WASM/JS throughput without scroll, DOM, or
// postMessage overhead getting in the way.

import init, { init as wasmInit, format_line } from "./pkg/hl_wasm.js";

const urlInput = document.getElementById("url-input");
const fileInput = document.getElementById("file-input");
const itersInput = document.getElementById("iters");
const runBtn = document.getElementById("run-btn");
const status = document.getElementById("status");
const results = document.getElementById("bench-results");

let wasmReady = false;
let loadedBuf = null;
let loadedLabel = null;
let lineStarts = null; // Int32Array of byte offsets

(async () => {
  await init();
  wasmInit();
  wasmReady = true;
  status.textContent = "Ready";
})();

runBtn.addEventListener("click", async () => {
  if (!wasmReady) {
    status.textContent = "WASM still loading…";
    return;
  }
  const iters = Math.max(1, Math.min(50, Number(itersInput.value) || 1));
  try {
    if (fileInput.files && fileInput.files[0]) {
      await loadFromFile(fileInput.files[0]);
    } else if (urlInput.value.trim()) {
      await loadFromUrl(urlInput.value.trim());
    } else {
      status.textContent = "Provide a URL or a file.";
      return;
    }
    await runIterations(iters);
  } catch (e) {
    status.textContent = `error: ${e.message ?? e}`;
    console.error(e);
  }
});

async function loadFromFile(file) {
  if (loadedLabel === `file:${file.name}:${file.size}`) return; // cached
  status.textContent = `Reading ${file.name} (${formatBytes(file.size)})…`;
  const t0 = performance.now();
  loadedBuf = new Uint8Array(await file.arrayBuffer());
  loadedLabel = `file:${file.name}:${file.size}`;
  const t1 = performance.now();
  status.textContent = `Read ${formatBytes(loadedBuf.length)} in ${((t1 - t0) / 1000).toFixed(2)}s — indexing lines…`;
  await nextFrame();
  indexLines();
}

async function loadFromUrl(url) {
  if (loadedLabel === `url:${url}`) return;
  status.textContent = `Fetching ${url}…`;
  const t0 = performance.now();
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url} returned ${res.status}`);
  loadedBuf = new Uint8Array(await res.arrayBuffer());
  loadedLabel = `url:${url}`;
  const t1 = performance.now();
  status.textContent = `Loaded ${formatBytes(loadedBuf.length)} in ${((t1 - t0) / 1000).toFixed(2)}s — indexing lines…`;
  await nextFrame();
  indexLines();
}

// Build a sorted list of line-start byte offsets so the bench loop can slice the file
// in O(1) per line without scanning for newlines on every iteration.
function indexLines() {
  const buf = loadedBuf;
  const starts = [0];
  for (let i = 0; i < buf.length; i++) {
    if (buf[i] === 0x0a && i + 1 < buf.length) starts.push(i + 1);
  }
  lineStarts = new Int32Array(starts);
}

async function runIterations(iters) {
  const buf = loadedBuf;
  const totalLines = lineStarts.length;
  status.textContent = `Warming up… (${totalLines.toLocaleString()} lines, ${iters} iters)`;
  await nextFrame();

  // Warmup pass (not measured): primes the JIT, populates icaches, etc.
  await oneIteration(buf, lineStarts);

  const samples = [];
  let lastOutputChars = 0;
  for (let k = 0; k < iters; k++) {
    status.textContent = `Iteration ${k + 1} / ${iters}…`;
    await nextFrame();
    const { elapsed, outputChars } = await oneIteration(buf, lineStarts);
    samples.push(elapsed);
    lastOutputChars = outputChars;
  }
  samples.sort((a, b) => a - b);
  const best = samples[0];
  const median = samples[(samples.length / 2) | 0];
  const worst = samples[samples.length - 1];
  appendResult({
    label: loadedLabel,
    bytes: buf.length,
    lines: totalLines,
    outputChars: lastOutputChars,
    iters,
    best,
    median,
    worst,
    samples,
  });
  status.textContent = "Done.";
}

// One pass of format_line over every line. Returns wall time and a side-effect sum
// (output character total) to prevent the optimizer from eliding the loop body.
function oneIteration(buf, starts) {
  return new Promise((resolve) => {
    // Resolve via microtask so the previous status repaint actually paints first.
    queueMicrotask(() => {
      const total = buf.length;
      const count = starts.length;
      let outputChars = 0;
      const t0 = performance.now();
      for (let i = 0; i < count; i++) {
        const start = starts[i];
        const end = i + 1 < count ? starts[i + 1] - 1 : total;
        if (end <= start) continue;
        const slice = buf.subarray(start, end);
        const html = format_line(slice);
        outputChars += html.length;
      }
      const t1 = performance.now();
      resolve({ elapsed: (t1 - t0) / 1000, outputChars });
    });
  });
}

function appendResult(r) {
  const div = document.createElement("div");
  div.className = "bench-run";
  const mibPerSec = r.bytes / 1024 / 1024 / r.best;
  const linesPerSec = r.lines / r.best;
  const tableRows = [
    ["source", r.label],
    ["bytes", `${formatBytes(r.bytes)} (${r.bytes.toLocaleString()})`],
    ["lines", r.lines.toLocaleString()],
    ["html output", `${formatBytes(r.outputChars)} chars`],
    ["iterations", `${r.iters}`],
    ["best", `${r.best.toFixed(3)} s`],
    ["median", `${r.median.toFixed(3)} s`],
    ["worst", `${r.worst.toFixed(3)} s`],
    ["throughput", `${mibPerSec.toFixed(1)} MiB/s · ${formatNum(linesPerSec)} lines/s`],
    ["all samples (s)", r.samples.map((s) => s.toFixed(3)).join("  ")],
  ];
  const inner = tableRows
    .map(([k, v]) => `<div class="bench-row"><span class="k">${k}</span><span>${escapeHtml(String(v))}</span></div>`)
    .join("");
  div.innerHTML = `<h3>format_line bench</h3>${inner}`;
  results.insertBefore(div, results.firstChild);
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MiB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}

function formatNum(n) {
  if (n >= 1e6) return `${(n / 1e6).toFixed(2)} M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(2)} k`;
  return n.toFixed(0);
}

function escapeHtml(s) {
  return s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

function nextFrame() {
  return new Promise((r) => requestAnimationFrame(() => r()));
}
