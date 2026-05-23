// Browser entry point for the hl WASM log viewer.
//
// Architecture:
//   - LogSource: HTTP fetcher with byte-range support, line-boundary slop, and an LRU chunk cache.
//   - LineIndex: sparse (line -> byte offset) map built lazily as the user scrolls.
//   - VirtualList: tall spacer + recycled row pool with translateY positioning.
//   - Formatter: a Web Worker owns the WASM renderer so format work doesn't block the UI thread.

// Formatter routes line-format requests to a dedicated worker. The worker hosts the WASM
// module and runs `format_line` per record off the main thread. We keep one worker
// because the WASM module is single-threaded; the throughput win comes from not blocking
// scroll/paint, not from parallelism.
class Formatter {
  constructor() {
    this.worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
    this.pending = new Map();
    this.nextId = 1;
    this.ready = new Promise((resolve) => {
      this._readyResolve = resolve;
    });
    this.worker.onmessage = (ev) => {
      const data = ev.data;
      if (data.type === "ready") {
        this._readyResolve();
        return;
      }
      if (data.type === "format_chunk_result") {
        const resolver = this.pending.get(data.id);
        if (resolver) {
          this.pending.delete(data.id);
          resolver(data.out);
        }
        return;
      }
      if (data.type === "search_chunk_result") {
        const resolver = this.pending.get(data.id);
        if (resolver) {
          this.pending.delete(data.id);
          resolver(data);
        }
      }
    };
  }

  formatChunk(firstLine, bytes) {
    return new Promise((resolve) => {
      const id = this.nextId++;
      this.pending.set(id, resolve);
      this.worker.postMessage({ type: "format_chunk", id, firstLine, bytes });
    });
  }

  searchChunk(firstLine, bytes, needle, caseInsensitive) {
    return new Promise((resolve) => {
      const id = this.nextId++;
      this.pending.set(id, resolve);
      this.worker.postMessage({
        type: "search_chunk",
        id,
        firstLine,
        bytes,
        needle,
        caseInsensitive,
      });
    });
  }
}

const formatter = new Formatter();

const CHUNK_SIZE = 256 * 1024; // 256 KiB per range fetch
const SLOP = 64 * 1024; // bytes fetched beyond requested boundaries to absorb partial lines
const CACHE_BYTES = 128 * 1024 * 1024; // 128 MiB byte-budget LRU for fetched chunks
const ROW_HEIGHT_PX = 18;
const PREFETCH_LINES = 1024; // keep at least this many lines above and below the visible window
const RENDER_CACHE_LINES = 32_768; // formatted-HTML LRU cap; well exceeds the prefetch window so backtracking is instant

const status = document.getElementById("status");
const viewport = document.getElementById("viewport");
const spacer = document.getElementById("spacer");
const rowsHost = document.getElementById("rows");
const openBtn = document.getElementById("open-btn");
const urlInput = document.getElementById("url-input");
const searchBar = document.getElementById("search-bar");
const searchInput = document.getElementById("search-input");
const searchCount = document.getElementById("search-count");
const searchPrev = document.getElementById("search-prev");
const searchNext = document.getElementById("search-next");
const searchClose = document.getElementById("search-close");

function setStatus(text) {
  status.textContent = text;
}

// ---

class LogSource {
  constructor(url) {
    this.url = url;
    this.totalSize = null;
    this.supportsRange = false;
    this.cache = new Map(); // chunkStart -> {bytes, startOffset, endOffset}
    this.cacheBytes = 0; // total bytes resident in `cache`
    this.inflight = new Map(); // chunkStart -> Promise<chunk>
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

      // Byte-budget LRU: evict oldest entries until the new chunk fits.
      while (this.cacheBytes + result.bytes.length > CACHE_BYTES && this.cache.size > 0) {
        const oldestKey = this.cache.keys().next().value;
        const oldest = this.cache.get(oldestKey);
        this.cache.delete(oldestKey);
        this.cacheBytes -= oldest.bytes.length;
      }
      this.cache.set(chunkStart, result);
      this.cacheBytes += result.bytes.length;
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

// Wrap match ranges in `<mark>` tags inside an HTML fragment. `ranges` is a flat
// `[start, end, start, end, …]` array of plain-text character offsets (i.e., positions in
// the decoded textContent, which is what the worker uses to compute matches). We parse the
// HTML via `<template>` and walk text nodes so we correctly handle entity-decoded content
// and nested style spans without producing malformed markup.
function injectMarks(html, ranges, currentRangeIdx) {
  if (!ranges || ranges.length === 0) return html;
  const tpl = document.createElement("template");
  tpl.innerHTML = html;
  const walker = document.createTreeWalker(tpl.content, NodeFilter.SHOW_TEXT);
  const nodes = [];
  while (walker.nextNode()) nodes.push(walker.currentNode);

  let textPos = 0;
  let rangeIdx = 0;
  for (const node of nodes) {
    if (rangeIdx * 2 >= ranges.length) break;
    const text = node.nodeValue;
    const nodeStart = textPos;
    const nodeEnd = textPos + text.length;
    textPos = nodeEnd;

    if (ranges[rangeIdx * 2] >= nodeEnd) continue;

    const pieces = [];
    let p = 0;
    while (rangeIdx * 2 < ranges.length) {
      const rs = ranges[rangeIdx * 2];
      const re = ranges[rangeIdx * 2 + 1];
      if (rs >= nodeEnd) break;
      const ls = Math.max(0, rs - nodeStart);
      const le = Math.min(text.length, re - nodeStart);
      if (p < ls) pieces.push([p, ls, false, false]);
      pieces.push([ls, le, true, rangeIdx === currentRangeIdx]);
      p = le;
      if (re <= nodeEnd) rangeIdx++;
      else break;
    }
    if (p < text.length) pieces.push([p, text.length, false, false]);

    const frag = document.createDocumentFragment();
    for (const [a, b, mark, current] of pieces) {
      const piece = text.substring(a, b);
      if (piece.length === 0) continue;
      if (mark) {
        const m = document.createElement("mark");
        m.className = current ? "search-hit current" : "search-hit";
        m.textContent = piece;
        frag.appendChild(m);
      } else {
        frag.appendChild(document.createTextNode(piece));
      }
    }
    node.parentNode.replaceChild(frag, node);
  }

  return tpl.innerHTML;
}

// Drives a sequential scan of the document and maintains a sorted match list. Each entry
// is `{line, ranges, idxStart, idxEnd}` where `idxStart..idxEnd` is the global match-index
// range contributed by that line (so we can render "X of Y" without summing on every move).
// Cancellation: the current scan checks a per-scan `cancelled` flag at every await point.
const SEARCH_MATCH_CAP = 100_000;

class Search {
  constructor(viewer) {
    this.viewer = viewer;
    this.query = "";
    this.caseInsensitive = true;
    this.matches = [];
    this.matchesByLine = new Map();
    this.totalMatches = 0;
    this.currentIndex = -1; // global match index, -1 = no current selection
    this.scanController = null;
    this.truncated = false;
    this.scanning = false;
  }

  setQuery(q) {
    if (q === this.query) return;
    this.cancel();
    this.query = q;
    this.matches = [];
    this.matchesByLine = new Map();
    this.totalMatches = 0;
    this.currentIndex = -1;
    this.truncated = false;
    this.scanning = false;
    if (q.length === 0) {
      this.viewer.onSearchUpdate({ repaint: true });
      return;
    }
    this.scanning = true;
    const ctrl = { cancelled: false };
    this.scanController = ctrl;
    this.runScan(ctrl).catch((e) => {
      if (!ctrl.cancelled) {
        console.error("search scan failed", e);
      }
    });
  }

  cancel() {
    if (this.scanController) this.scanController.cancelled = true;
    this.scanController = null;
    this.scanning = false;
  }

  async runScan(ctrl) {
    const src = this.viewer.source;
    if (!src) return;
    const totalSize = src.totalSize;
    for (let chunkStart = 0; chunkStart < totalSize; chunkStart += CHUNK_SIZE) {
      if (ctrl.cancelled) return;
      let chunk;
      try {
        chunk = await src.fetchChunk(chunkStart);
      } catch (e) {
        console.warn("search fetch failed", e);
        continue;
      }
      if (ctrl.cancelled) return;
      // Keep the line index in sync so lineAtOffset for later chunks resolves correctly.
      const startLine = this.viewer.lineAtOffset(chunk.startOffset);
      this.viewer.index.ingest(startLine, chunk.startOffset, chunk.bytes);

      const result = await formatter.searchChunk(
        startLine,
        chunk.bytes,
        this.query,
        this.caseInsensitive,
      );
      if (ctrl.cancelled) return;

      let appended = false;
      for (const hit of result.hits) {
        if (this.matchesByLine.has(hit.line)) continue; // dedupe overlap from slop
        const count = hit.ranges.length / 2;
        const entry = {
          line: hit.line,
          ranges: hit.ranges,
          idxStart: this.totalMatches,
          idxEnd: this.totalMatches + count - 1,
        };
        this.matches.push(entry);
        this.matchesByLine.set(hit.line, entry);
        this.totalMatches += count;
        appended = true;
        if (this.totalMatches >= SEARCH_MATCH_CAP) {
          this.truncated = true;
          break;
        }
      }
      this.viewer.onSearchUpdate({ repaint: appended });
      if (this.truncated) {
        this.scanning = false;
        return;
      }
    }
    this.scanning = false;
    this.viewer.onSearchUpdate({ repaint: false });
  }

  // Locate the entry that contains the given global match index.
  entryForGlobalIdx(globalIdx) {
    let lo = 0,
      hi = this.matches.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      const e = this.matches[mid];
      if (globalIdx < e.idxStart) hi = mid - 1;
      else if (globalIdx > e.idxEnd) lo = mid + 1;
      else return e;
    }
    return null;
  }

  // Returns the global match index for "the first match at or after `line`" (or -1).
  firstAtOrAfterLine(line) {
    let lo = 0,
      hi = this.matches.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (this.matches[mid].line < line) lo = mid + 1;
      else hi = mid - 1;
    }
    if (lo >= this.matches.length) return -1;
    return this.matches[lo].idxStart;
  }

  lastBeforeLine(line) {
    let lo = 0,
      hi = this.matches.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (this.matches[mid].line >= line) hi = mid - 1;
      else lo = mid + 1;
    }
    if (hi < 0) return -1;
    return this.matches[hi].idxEnd;
  }

  next() {
    if (this.totalMatches === 0) return;
    if (this.currentIndex < 0) {
      const viewLine = Math.floor(viewport.scrollTop / ROW_HEIGHT_PX);
      const found = this.firstAtOrAfterLine(viewLine);
      this.currentIndex = found >= 0 ? found : 0;
    } else {
      this.currentIndex = (this.currentIndex + 1) % this.totalMatches;
    }
    this.gotoCurrent();
  }

  prev() {
    if (this.totalMatches === 0) return;
    if (this.currentIndex < 0) {
      const viewLine = Math.floor(viewport.scrollTop / ROW_HEIGHT_PX);
      const found = this.lastBeforeLine(viewLine);
      this.currentIndex = found >= 0 ? found : this.totalMatches - 1;
    } else {
      this.currentIndex = (this.currentIndex - 1 + this.totalMatches) % this.totalMatches;
    }
    this.gotoCurrent();
  }

  gotoCurrent() {
    const entry = this.entryForGlobalIdx(this.currentIndex);
    if (!entry) return;
    this.viewer.scrollToLine(entry.line);
    this.viewer.onSearchUpdate({ repaint: true });
  }

  // For the row painter: which range within this line, if any, is the current match.
  currentRangeIdxForLine(line) {
    if (this.currentIndex < 0) return -1;
    const entry = this.matchesByLine.get(line);
    if (!entry) return -1;
    if (this.currentIndex < entry.idxStart || this.currentIndex > entry.idxEnd) return -1;
    return this.currentIndex - entry.idxStart;
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
    this.search = new Search(this);
    this.onSearchStateChanged = null; // bootstrap-installed callback for count UI
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
    // Coalesce burst scroll events into one visible-window recompute per frame.
    this.rafPending = false;
    this.scrollListener = () => {
      if (this.rafPending) return;
      this.rafPending = true;
      requestAnimationFrame(() => {
        this.rafPending = false;
        this.updateVisible();
      });
    };
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
    if (this.search) this.search.cancel();
    rowsHost.innerHTML = "";
    this.rowPool = [];
  }

  // Center the given line in the viewport if it isn't already on-screen; bias toward the
  // top third when scrolling (mirrors browser Find behavior so the user has reading context
  // below the match).
  scrollToLine(line) {
    const target = line * ROW_HEIGHT_PX;
    const top = viewport.scrollTop;
    const bottom = top + viewport.clientHeight;
    if (target < top || target + ROW_HEIGHT_PX > bottom) {
      const desired = Math.max(0, target - viewport.clientHeight / 3);
      viewport.scrollTo({ top: desired });
    }
  }

  onSearchUpdate({ repaint }) {
    if (repaint && this.lastRange.first >= 0) {
      this.paintRows(this.lastRange.first, this.lastRange.last);
    }
    if (this.onSearchStateChanged) this.onSearchStateChanged();
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

    // Prefetch a fixed line window above/below so backwards/forwards motion stays instant.
    const wantFirst = Math.max(0, first - PREFETCH_LINES);
    const wantLast = Math.min(this.index.estimatedTotal() - 1, last + PREFETCH_LINES);
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
    const search = this.search;
    const searchActive = search && search.query.length > 0;
    for (let i = 0; i < this.poolSize; i++) {
      const line = first + i;
      const row = this.rowPool[i];
      row.style.transform = `translateY(${line * ROW_HEIGHT_PX}px)`;
      const html = this.renderCache.get(line);
      if (html !== undefined) {
        let displayHtml = html;
        let marker = "n";
        if (searchActive) {
          const entry = search.matchesByLine.get(line);
          if (entry) {
            const curIdx = search.currentRangeIdxForLine(line);
            displayHtml = injectMarks(html, entry.ranges, curIdx);
            marker = curIdx >= 0 ? `c${curIdx}` : "h";
          }
        }
        const sig = `r|${line}|${marker}`;
        if (row.dataset.sig !== sig) {
          row.innerHTML = displayHtml;
          row.className = "row rendered";
          row.dataset.line = String(line);
          row.dataset.sig = sig;
        }
      } else {
        const sig = `p|${line}`;
        if (row.dataset.sig !== sig) {
          row.textContent = `${line + 1}`;
          row.className = "row placeholder";
          row.dataset.line = String(line);
          row.dataset.sig = sig;
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

  async absorbChunk(chunk) {
    const startLine = this.lineAtOffset(chunk.startOffset);
    this.index.ingest(startLine, chunk.startOffset, chunk.bytes);
    spacer.style.height = `${this.index.estimatedTotal() * ROW_HEIGHT_PX}px`;
    this.pendingChunks.delete(Math.floor(chunk.startOffset / CHUNK_SIZE) * CHUNK_SIZE);
    // Repaint immediately so placeholders reflect the new total line count;
    // the actual line HTML lands once the worker responds.
    this.updateVisible();
    setStatus(
      `Indexed ${this.index.knownLines.toLocaleString()} of ~${this.index.estimatedTotal().toLocaleString()} lines (${formatBytes(this.source.totalSize)})`,
    );

    const out = await formatter.formatChunk(startLine, chunk.bytes);
    for (let i = 0; i < out.length; i += 2) {
      const line = out[i];
      const html = out[i + 1];
      this.renderCache.set(line, html);
      if (this.renderCache.size > RENDER_CACHE_LINES) {
        const oldest = this.renderCache.keys().next().value;
        this.renderCache.delete(oldest);
      }
    }
    this.updateVisible();
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

}

// ---

let currentViewer = null;

async function openLog(url) {
  if (currentViewer) {
    currentViewer.stop();
    currentViewer = null;
  }
  closeSearchBar();
  try {
    const source = new LogSource(url);
    await source.probe();
    const viewer = new Viewer(source);
    viewer.onSearchStateChanged = updateSearchCountUI;
    viewer.start();
    currentViewer = viewer;
  } catch (e) {
    setStatus(`error: ${e.message ?? e}`);
    console.error(e);
  }
}

function updateSearchCountUI() {
  const s = currentViewer && currentViewer.search;
  if (!s || s.query.length === 0) {
    searchCount.textContent = "0 / 0";
    searchPrev.disabled = true;
    searchNext.disabled = true;
    return;
  }
  const total = s.totalMatches;
  const cur = s.currentIndex < 0 ? 0 : s.currentIndex + 1;
  let label = `${cur.toLocaleString()} / ${total.toLocaleString()}`;
  if (s.truncated) label += "+";
  else if (s.scanning) label += "…";
  searchCount.textContent = label;
  searchPrev.disabled = total === 0;
  searchNext.disabled = total === 0;
}

function openSearchBar() {
  if (!searchBar.hidden) {
    searchInput.focus();
    searchInput.select();
    return;
  }
  searchBar.hidden = false;
  searchInput.focus();
  searchInput.select();
  updateSearchCountUI();
}

function closeSearchBar() {
  searchBar.hidden = true;
  if (currentViewer && currentViewer.search) {
    currentViewer.search.setQuery("");
  }
  updateSearchCountUI();
}

let searchDebounceTimer = null;
function scheduleSearch(q) {
  if (searchDebounceTimer !== null) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    searchDebounceTimer = null;
    if (!currentViewer) return;
    currentViewer.search.setQuery(q);
    updateSearchCountUI();
  }, 120);
}

async function bootstrap() {
  setStatus("Loading WASM…");
  await formatter.ready;
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

  // Cmd/Ctrl+F opens the in-page search bar; we intercept so the browser's native Find
  // (which can only see the handful of virtualized rows currently in the DOM) doesn't win.
  document.addEventListener("keydown", (e) => {
    const meta = e.metaKey || e.ctrlKey;
    if (meta && (e.key === "f" || e.key === "F")) {
      if (!currentViewer) return;
      e.preventDefault();
      openSearchBar();
      return;
    }
    if (e.key === "F3") {
      if (!currentViewer) return;
      e.preventDefault();
      openSearchBar();
      if (e.shiftKey) currentViewer.search.prev();
      else currentViewer.search.next();
      updateSearchCountUI();
      return;
    }
    if (e.key === "Escape" && !searchBar.hidden) {
      e.preventDefault();
      closeSearchBar();
      viewport.focus();
    }
  });

  searchInput.addEventListener("input", () => {
    scheduleSearch(searchInput.value);
  });
  searchInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (!currentViewer) return;
      // Flush any pending debounce so the search reflects what's typed before navigating.
      if (searchDebounceTimer !== null) {
        clearTimeout(searchDebounceTimer);
        searchDebounceTimer = null;
        currentViewer.search.setQuery(searchInput.value);
      }
      if (e.shiftKey) currentViewer.search.prev();
      else currentViewer.search.next();
      updateSearchCountUI();
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeSearchBar();
      viewport.focus();
    }
  });
  searchPrev.addEventListener("click", () => {
    if (!currentViewer) return;
    currentViewer.search.prev();
    updateSearchCountUI();
  });
  searchNext.addEventListener("click", () => {
    if (!currentViewer) return;
    currentViewer.search.next();
    updateSearchCountUI();
  });
  searchClose.addEventListener("click", () => {
    closeSearchBar();
    viewport.focus();
  });
}

bootstrap().catch((e) => {
  setStatus(`bootstrap failed: ${e.message ?? e}`);
  console.error(e);
});
