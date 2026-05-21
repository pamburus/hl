// Browser entry point for the hl WASM log viewer.
//
// Architecture:
//   - LogSource: HTTP fetcher with byte-range support, line-boundary slop, and an LRU chunk cache.
//   - LineIndex: sparse (line -> byte offset) map built lazily as the user scrolls.
//   - VirtualList: tall spacer + recycled row pool with translateY positioning.
//   - Renderer is the WASM module; format_line(bytes) returns ready-to-insert HTML.

import init, { init as wasmInit, format_line } from "./pkg/hl_wasm.js";

const CHUNK_SIZE = 256 * 1024; // 256 KiB per range fetch
const SLOP = 64 * 1024; // bytes fetched beyond requested boundaries to absorb partial lines
const CACHE_CHUNKS = 32; // simple LRU bound on cached chunks
const ROW_HEIGHT_PX = 18;
const PREFETCH_VIEWPORTS = 2; // load this many pages above/below the visible window

const status = document.getElementById("status");
const viewport = document.getElementById("viewport");
const spacer = document.getElementById("spacer");
const rowsHost = document.getElementById("rows");
const openBtn = document.getElementById("open-btn");
const urlInput = document.getElementById("url-input");

function setStatus(text) {
  status.textContent = text;
}

// ---

class LogSource {
  constructor(url) {
    this.url = url;
    this.totalSize = null;
    this.supportsRange = false;
    this.cache = new Map(); // chunkStart -> Uint8Array
    this.inflight = new Map(); // chunkStart -> Promise<Uint8Array>
  }

  async probe() {
    setStatus("Probing…");
    let res;
    try {
      res = await fetch(this.url, { method: "HEAD" });
    } catch (e) {
      throw new Error(`failed to reach URL: ${e.message ?? e}`);
    }
    if (!res.ok) {
      throw new Error(`HEAD ${this.url} returned ${res.status}`);
    }
    const len = res.headers.get("content-length");
    this.totalSize = len !== null ? Number(len) : null;
    this.supportsRange = (res.headers.get("accept-ranges") ?? "").includes("bytes");
    if (this.totalSize === null || Number.isNaN(this.totalSize)) {
      throw new Error("server did not return Content-Length; cannot virtualize");
    }
    if (!this.supportsRange) {
      setStatus("Range requests not supported — falling back to full GET");
    } else {
      setStatus(`Connected. ${formatBytes(this.totalSize)}`);
    }
  }

  // Fetch a chunk of bytes spanning [chunkStart, chunkStart + CHUNK_SIZE), trimmed to whole lines.
  // Returns { bytes, startOffset, endOffset } where the offsets are byte positions in the file.
  async fetchChunk(chunkStart) {
    if (this.cache.has(chunkStart)) {
      // refresh LRU
      const b = this.cache.get(chunkStart);
      this.cache.delete(chunkStart);
      this.cache.set(chunkStart, b);
      return b;
    }
    if (this.inflight.has(chunkStart)) return this.inflight.get(chunkStart);

    const p = (async () => {
      const fetchStart = Math.max(0, chunkStart - SLOP);
      const fetchEnd = Math.min(this.totalSize, chunkStart + CHUNK_SIZE + SLOP);
      const headers = this.supportsRange
        ? { Range: `bytes=${fetchStart}-${fetchEnd - 1}` }
        : {};
      const res = await fetch(this.url, { headers });
      if (!res.ok && !(this.supportsRange && res.status === 206)) {
        throw new Error(`GET ${this.url} returned ${res.status}`);
      }
      const buf = new Uint8Array(await res.arrayBuffer());

      // Trim leading partial line (unless we're at file start) and trailing partial line (unless EOF).
      let trimStart = 0;
      if (fetchStart > 0) {
        trimStart = indexOf(buf, 0x0a, 0); // first \n
        if (trimStart < 0) trimStart = buf.length; // no newline at all — discard everything
        else trimStart += 1;
      }
      let trimEnd = buf.length;
      if (fetchEnd < this.totalSize) {
        trimEnd = lastIndexOf(buf, 0x0a, buf.length - 1);
        if (trimEnd < 0) trimEnd = trimStart; // discard if no newline found
        else trimEnd += 1; // include the trailing newline
      }
      const trimmed = buf.subarray(trimStart, trimEnd);
      const startOffset = fetchStart + trimStart;
      const result = {
        bytes: trimmed,
        startOffset,
        endOffset: fetchStart + trimEnd,
      };

      // LRU eviction
      if (this.cache.size >= CACHE_CHUNKS) {
        const oldest = this.cache.keys().next().value;
        this.cache.delete(oldest);
      }
      this.cache.set(chunkStart, result);
      return result;
    })();
    this.inflight.set(chunkStart, p);
    try {
      return await p;
    } finally {
      this.inflight.delete(chunkStart);
    }
  }
}

function indexOf(buf, byte, from) {
  for (let i = from; i < buf.length; i++) if (buf[i] === byte) return i;
  return -1;
}

function lastIndexOf(buf, byte, from) {
  for (let i = from; i >= 0; i--) if (buf[i] === byte) return i;
  return -1;
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MiB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}

// ---

// A sparse map from line index to (byteOffset, lineByteLength). Built lazily as chunks arrive.
class LineIndex {
  constructor(totalSize) {
    this.totalSize = totalSize;
    // line -> { offset, length }
    this.entries = new Map();
    this.entries.set(0, { offset: 0, length: 0 });
    this.knownLines = 1;
    // Running estimate of bytes per line. Starts pessimistic; refined as we observe data.
    this.avgBytesPerLine = 120;
  }

  // Ingest a chunk that starts exactly at file byte `startOffset`. Adds entries for each line
  // boundary found in `bytes`. Records the line index of the first line in the chunk via
  // `firstLine` (caller responsibility — typically resolved from prior chunk overlap).
  ingest(firstLine, startOffset, bytes) {
    let cursor = 0;
    let line = firstLine;
    while (cursor < bytes.length) {
      const nl = indexOf(bytes, 0x0a, cursor);
      const next = nl < 0 ? bytes.length : nl + 1;
      this.entries.set(line, { offset: startOffset + cursor, length: next - cursor });
      line += 1;
      cursor = next;
      if (nl < 0) break;
    }
    if (line > this.knownLines) this.knownLines = line;
    // refine running average
    if (bytes.length > 0 && line > firstLine) {
      const observed = bytes.length / (line - firstLine);
      this.avgBytesPerLine = this.avgBytesPerLine * 0.7 + observed * 0.3;
    }
  }

  estimatedTotal() {
    if (this.totalSize === 0) return 1;
    return Math.max(this.knownLines, Math.ceil(this.totalSize / this.avgBytesPerLine));
  }

  get(line) {
    return this.entries.get(line);
  }

  // Nearest known line at or before `line`.
  floor(line) {
    let best = null;
    // Linear scan is fine because the index stays small (one entry per line we've seen).
    // For larger logs we'd switch to a sorted array + binary search.
    for (const [k, v] of this.entries) {
      if (k <= line && (best === null || k > best.line)) {
        best = { line: k, ...v };
      }
    }
    return best;
  }
}

// ---

class Viewer {
  constructor(source) {
    this.source = source;
    this.index = new LineIndex(source.totalSize);
    this.pendingChunks = new Set();
    this.rowPool = []; // DOM nodes
    this.poolSize = 0;
    this.scrollListener = null;
    this.resizeListener = null;
    this.lastRange = { first: -1, last: -1 };
    this.renderCache = new Map(); // line index -> HTML string (small LRU)
  }

  start() {
    spacer.style.height = `${this.index.estimatedTotal() * ROW_HEIGHT_PX}px`;
    const visibleRows = Math.ceil(viewport.clientHeight / ROW_HEIGHT_PX) + 4;
    this.poolSize = visibleRows;
    for (let i = 0; i < this.poolSize; i++) {
      const div = document.createElement("div");
      div.className = "row placeholder";
      div.textContent = "…";
      rowsHost.appendChild(div);
      this.rowPool.push(div);
    }
    this.scrollListener = () => this.updateVisible();
    viewport.addEventListener("scroll", this.scrollListener, { passive: true });
    this.resizeListener = () => this.resize();
    window.addEventListener("resize", this.resizeListener);
    this.updateVisible();
    // Bootstrap: fetch the first chunk so we have real data + a real line average.
    this.ensureChunkContaining(0);
  }

  stop() {
    if (this.scrollListener) viewport.removeEventListener("scroll", this.scrollListener);
    if (this.resizeListener) window.removeEventListener("resize", this.resizeListener);
    rowsHost.innerHTML = "";
    this.rowPool = [];
  }

  resize() {
    const visibleRows = Math.ceil(viewport.clientHeight / ROW_HEIGHT_PX) + 4;
    while (this.rowPool.length < visibleRows) {
      const div = document.createElement("div");
      div.className = "row placeholder";
      rowsHost.appendChild(div);
      this.rowPool.push(div);
    }
    this.poolSize = visibleRows;
    this.updateVisible();
  }

  // Compute the visible window of rows, then ensure the necessary chunks are loaded and
  // assign rows from the pool to the visible lines.
  updateVisible() {
    const scrollTop = viewport.scrollTop;
    const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT_PX) - 2);
    const last = first + this.poolSize - 1;

    // Prefetch 2 viewports above/below.
    const reach = this.poolSize * PREFETCH_VIEWPORTS;
    const wantFirst = Math.max(0, first - reach);
    const wantLast = Math.min(this.index.estimatedTotal() - 1, last + reach);
    this.ensureLinesLoaded(wantFirst, wantLast);

    if (first === this.lastRange.first && last === this.lastRange.last) {
      // even if we didn't move, content might have arrived — refresh visible row content.
      this.paintRows(first, last);
      return;
    }
    this.lastRange = { first, last };
    this.paintRows(first, last);
  }

  paintRows(first, last) {
    for (let i = 0; i < this.poolSize; i++) {
      const line = first + i;
      const row = this.rowPool[i];
      row.style.transform = `translateY(${line * ROW_HEIGHT_PX}px)`;
      const html = this.renderCache.get(line);
      if (html !== undefined) {
        if (row.dataset.line !== String(line) || !row.classList.contains("rendered")) {
          row.innerHTML = html;
          row.className = "row rendered";
          row.dataset.line = String(line);
        }
      } else {
        if (row.dataset.line !== String(line) || !row.classList.contains("placeholder")) {
          row.textContent = `${line + 1}`;
          row.className = "row placeholder";
          row.dataset.line = String(line);
        }
      }
    }
  }

  // Make sure that any chunk needed to render lines in [firstLine, lastLine] is being fetched.
  ensureLinesLoaded(firstLine, lastLine) {
    // We only know byte offsets for lines we've already indexed. For lines beyond that, estimate
    // a byte position and fetch the chunk that contains it. The newly fetched chunk's line indexes
    // are then absorbed into the index.
    const firstByte = this.estimateByteForLine(firstLine);
    const lastByte = this.estimateByteForLine(lastLine);
    const firstChunk = Math.floor(firstByte / CHUNK_SIZE) * CHUNK_SIZE;
    const lastChunk = Math.floor(lastByte / CHUNK_SIZE) * CHUNK_SIZE;
    for (let c = firstChunk; c <= lastChunk; c += CHUNK_SIZE) {
      this.ensureChunkContaining(c);
    }
  }

  estimateByteForLine(line) {
    const known = this.index.floor(line);
    if (!known) return 0;
    const delta = line - known.line;
    return Math.min(this.source.totalSize - 1, known.offset + delta * this.index.avgBytesPerLine);
  }

  ensureChunkContaining(byteOffset) {
    if (byteOffset >= this.source.totalSize) return;
    const chunkStart = Math.floor(byteOffset / CHUNK_SIZE) * CHUNK_SIZE;
    if (this.pendingChunks.has(chunkStart)) return;
    if (this.source.cache.has(chunkStart)) return;
    this.pendingChunks.add(chunkStart);
    this.source.fetchChunk(chunkStart).then(
      (chunk) => this.absorbChunk(chunk),
      (err) => {
        console.error("chunk fetch failed", err);
        setStatus(`fetch error: ${err.message ?? err}`);
        this.pendingChunks.delete(chunkStart);
      },
    );
  }

  absorbChunk(chunk) {
    const startLine = this.lineAtOffset(chunk.startOffset);
    this.index.ingest(startLine, chunk.startOffset, chunk.bytes);
    // Re-render visible rows whose lines now have data.
    this.renderLinesInChunk(startLine, chunk);
    spacer.style.height = `${this.index.estimatedTotal() * ROW_HEIGHT_PX}px`;
    this.pendingChunks.delete(Math.floor(chunk.startOffset / CHUNK_SIZE) * CHUNK_SIZE);
    this.updateVisible();
    setStatus(
      `Indexed ${this.index.knownLines.toLocaleString()} of ~${this.index.estimatedTotal().toLocaleString()} lines (${formatBytes(this.source.totalSize)})`,
    );
  }

  lineAtOffset(byteOffset) {
    // Find the largest known offset <= byteOffset, then count newlines in the gap if any.
    // For chunks we fetch in chunkStart-aligned order, this is normally the previous chunk's last line.
    let best = { line: 0, offset: 0 };
    for (const [line, v] of this.index.entries) {
      if (v.offset <= byteOffset && v.offset > best.offset) {
        best = { line, offset: v.offset };
      }
    }
    if (best.offset === byteOffset) return best.line;
    // We don't have the bytes between best.offset and byteOffset, but for the first chunk we always start at 0.
    // Subsequent chunks should start at a previously-indexed line boundary because of slop trimming.
    // For correctness in pathological cases we fall back to estimation by avgBytesPerLine.
    const gap = byteOffset - best.offset;
    return best.line + Math.round(gap / this.index.avgBytesPerLine);
  }

  renderLinesInChunk(firstLine, chunk) {
    let cursor = 0;
    let line = firstLine;
    while (cursor < chunk.bytes.length) {
      const nl = indexOf(chunk.bytes, 0x0a, cursor);
      const end = nl < 0 ? chunk.bytes.length : nl;
      const slice = chunk.bytes.subarray(cursor, end);
      let html;
      try {
        html = format_line(slice);
      } catch (e) {
        html = `<span class="row error">parse error: ${escapeHtml(e.message ?? String(e))}</span>`;
      }
      if (html.length === 0) {
        html = escapeHtml(decodeUtf8(slice));
      }
      this.renderCache.set(line, html);
      // simple cache cap
      if (this.renderCache.size > 4096) {
        const oldest = this.renderCache.keys().next().value;
        this.renderCache.delete(oldest);
      }
      cursor = nl < 0 ? chunk.bytes.length : nl + 1;
      line += 1;
    }
  }
}

function escapeHtml(s) {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function decodeUtf8(bytes) {
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

// ---

let currentViewer = null;

async function openLog(url) {
  if (currentViewer) {
    currentViewer.stop();
    currentViewer = null;
  }
  try {
    const source = new LogSource(url);
    await source.probe();
    const viewer = new Viewer(source);
    viewer.start();
    currentViewer = viewer;
  } catch (e) {
    setStatus(`error: ${e.message ?? e}`);
    console.error(e);
  }
}

async function bootstrap() {
  setStatus("Loading WASM…");
  await init();
  wasmInit();
  setStatus("Ready");

  const params = new URLSearchParams(window.location.search);
  const src = params.get("src");
  if (src) {
    urlInput.value = src;
    openLog(src);
  }

  openBtn.addEventListener("click", () => {
    const url = urlInput.value.trim();
    if (url) {
      const params = new URLSearchParams(window.location.search);
      params.set("src", url);
      history.replaceState(null, "", `?${params.toString()}`);
      openLog(url);
    }
  });
  urlInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      openBtn.click();
    }
  });

  viewport.addEventListener("keydown", (e) => {
    const step = ROW_HEIGHT_PX;
    const page = viewport.clientHeight - step;
    switch (e.key) {
      case "ArrowDown":
        viewport.scrollBy({ top: step });
        break;
      case "ArrowUp":
        viewport.scrollBy({ top: -step });
        break;
      case "PageDown":
        viewport.scrollBy({ top: page });
        break;
      case "PageUp":
        viewport.scrollBy({ top: -page });
        break;
      case "Home":
        viewport.scrollTo({ top: 0 });
        break;
      case "End":
        viewport.scrollTo({ top: spacer.offsetHeight });
        break;
      default:
        return;
    }
    e.preventDefault();
  });
}

bootstrap().catch((e) => {
  setStatus(`bootstrap failed: ${e.message ?? e}`);
  console.error(e);
});
