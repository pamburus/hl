// Static viewer. Thin client over the hl-server HTTP API:
//   GET /api/probe?url=…          -> source metadata (content_length, etc.)
//   GET /api/render?url=…&start&end -> rendered, structured lines for a byte range
//
// The browser never fetches the source URL itself — everything is proxied through
// the backend. Heavy work (parse, format) happens server-side. The client only:
//   - maintains a (sparse) byte<->line index estimated from observed bytes/line
//   - drives a virtualised scroll over a recycled row pool
//   - paints `Segment`s as styled <span>s using the CSS palette
//   - caches rendered lines (capped) and prefetches a couple of chunks around the
//     visible window
//
// All values are configurable as constants here so we can tune from one place.

// --- constants ---

const ROW_HEIGHT_PX = 18;
// Bytes per /api/render request. Bigger = fewer round trips but more work
// server-side per request; matches the server's MAX_RENDER_BYTES (16 MiB) by being
// well below it.
const CHUNK_SIZE = 256 * 1024;
// Prefetch this many chunks beyond the visible window in each direction.
const READ_AHEAD_CHUNKS = 2;
// Soft cap on the rendered-line cache. When exceeded, we evict the oldest insertions
// (Map iteration order); cheap, no real LRU bookkeeping.
const RENDER_CACHE_LINES = 50_000;
// Browser scroll heights cap somewhere around 17–33 M px depending on the engine;
// pick a safe ceiling and we'll silently scale beyond it.
const MAX_SPACER_HEIGHT_PX = 16_000_000;
// Initial guess until we observe the first chunk's actual bytes-per-line.
const INITIAL_BYTES_PER_LINE = 120;

// --- DOM refs ---

const urlInput = document.getElementById("url-input");
const openBtn = document.getElementById("open-btn");
const statusEl = document.getElementById("status");
const viewport = document.getElementById("viewport");
const spacer = document.getElementById("spacer");
const rowsHost = document.getElementById("rows");

// --- bootstrap ---

injectPalette();

const params = new URLSearchParams(window.location.search);
const initialSrc = params.get("src");
if (initialSrc) {
  urlInput.value = initialSrc;
  openLog(initialSrc);
}

openBtn.addEventListener("click", () => {
  const url = urlInput.value.trim();
  if (!url) return;
  const p = new URLSearchParams(window.location.search);
  p.set("src", url);
  history.replaceState(null, "", `?${p.toString()}`);
  openLog(url);
});

urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    openBtn.click();
  }
});

let currentViewer = null;

async function openLog(url) {
  if (currentViewer) {
    currentViewer.stop();
    currentViewer = null;
  }
  setStatus("Probing…");
  try {
    const meta = await probe(url);
    if (!meta.content_length) {
      throw new Error("source did not return Content-Length");
    }
    setStatus(`Connected. ${formatBytes(meta.content_length)}`);
    currentViewer = new Viewer(url, meta.content_length);
    currentViewer.start();
  } catch (e) {
    setStatus(`error: ${e.message ?? e}`);
    console.error(e);
  }
}

// --- API client ---

async function probe(url) {
  const res = await fetch(`/api/probe?url=${encodeURIComponent(url)}`);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error ?? `probe failed: ${res.status}`);
  }
  return res.json();
}

async function renderRange(url, start, end) {
  const res = await fetch(
    `/api/render?url=${encodeURIComponent(url)}&start=${start}&end=${end}`,
  );
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error ?? `render failed: ${res.status}`);
  }
  return res.json();
}

// --- line index ---

// Sparse byte<->line map built from anchors observed in successive chunks. Between
// anchors, positions are extrapolated using a running bytes-per-line average. Exact
// for chunks fetched contiguously from byte 0; approximate for chunks fetched after
// a scroll jump (which is good enough — we use it to drive the next fetch, not to
// label rows).
class LineIndex {
  constructor(totalSize) {
    this.totalSize = totalSize;
    // sorted-by-byte; anchors[0] is always {byte:0, line:0}
    this.anchors = [{ byte: 0, line: 0 }];
    this.avgBytesPerLine = INITIAL_BYTES_PER_LINE;
  }

  estimatedTotalLines() {
    return Math.max(1, Math.ceil(this.totalSize / this.avgBytesPerLine));
  }

  /// Nearest anchor with byte <= b.
  floor(b) {
    let lo = 0,
      hi = this.anchors.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (this.anchors[mid].byte <= b) lo = mid;
      else hi = mid - 1;
    }
    return this.anchors[lo];
  }

  /// Nearest anchor with line <= n.
  floorLine(n) {
    // Anchors are sorted by byte, but also by line (since chunks are non-overlapping
    // and we tag in arrival order). Linear scan is fine — we never accumulate more
    // than a few hundred anchors.
    let best = this.anchors[0];
    for (const a of this.anchors) {
      if (a.line <= n && a.line >= best.line) best = a;
    }
    return best;
  }

  lineAt(byte) {
    const a = this.floor(byte);
    return a.line + Math.round((byte - a.byte) / this.avgBytesPerLine);
  }

  byteAt(line) {
    const a = this.floorLine(line);
    const b = a.byte + (line - a.line) * this.avgBytesPerLine;
    return Math.min(this.totalSize, Math.max(0, b));
  }

  /// Register a chunk we just received. Returns the line number we assigned to its
  /// first line so the caller can stuff the cache.
  ingest(chunk) {
    if (!chunk.lines || chunk.lines.length === 0) {
      return null;
    }
    const baseLine = this.lineAt(chunk.first_byte);
    this.upsertAnchor(chunk.first_byte, baseLine);
    // Refine the running average from the observed span.
    const span = chunk.last_byte - chunk.first_byte;
    if (span > 0) {
      const observed = span / chunk.lines.length;
      this.avgBytesPerLine = this.avgBytesPerLine * 0.7 + observed * 0.3;
    }
    return baseLine;
  }

  upsertAnchor(byte, line) {
    // Replace if we already have an anchor at this byte; otherwise insert sorted.
    for (let i = 0; i < this.anchors.length; i++) {
      if (this.anchors[i].byte === byte) {
        this.anchors[i].line = line;
        return;
      }
      if (this.anchors[i].byte > byte) {
        this.anchors.splice(i, 0, { byte, line });
        return;
      }
    }
    this.anchors.push({ byte, line });
  }
}

// --- viewer ---

class Viewer {
  constructor(srcUrl, totalSize) {
    this.srcUrl = srcUrl;
    this.index = new LineIndex(totalSize);
    this.lineCache = new Map(); // line -> { start, segments }
    this.inflight = new Map(); // chunkStart -> Promise
    this.fetched = new Set(); // chunkStart that completed
    this.rowPool = [];
    this.poolSize = 0;
    this.rafPending = false;
    this.lastRange = { first: -1, last: -1 };
    this.scrollListener = null;
    this.resizeListener = null;
    this.stopped = false;
  }

  start() {
    this.updateSpacerHeight();
    this.allocatePool();
    this.scrollListener = () => this.requestRepaint();
    this.resizeListener = () => this.onResize();
    viewport.addEventListener("scroll", this.scrollListener, { passive: true });
    window.addEventListener("resize", this.resizeListener);
    // Bootstrap from byte 0 so we get an exact line-0 anchor and tune avgBpl quickly.
    this.fetchChunkAt(0);
    this.updateVisible();
  }

  stop() {
    this.stopped = true;
    if (this.scrollListener) viewport.removeEventListener("scroll", this.scrollListener);
    if (this.resizeListener) window.removeEventListener("resize", this.resizeListener);
    rowsHost.replaceChildren();
    this.rowPool = [];
    this.lineCache.clear();
    this.fetched.clear();
    this.inflight.clear();
  }

  allocatePool() {
    const visibleRows = Math.max(1, Math.ceil(viewport.clientHeight / ROW_HEIGHT_PX) + 4);
    this.poolSize = visibleRows;
    rowsHost.replaceChildren();
    this.rowPool = [];
    for (let i = 0; i < this.poolSize; i++) {
      const div = document.createElement("div");
      div.className = "row placeholder";
      div.textContent = "…";
      rowsHost.appendChild(div);
      this.rowPool.push(div);
    }
  }

  onResize() {
    // Reallocate the pool to match the new viewport.
    this.allocatePool();
    this.lastRange = { first: -1, last: -1 };
    this.updateVisible();
  }

  updateSpacerHeight() {
    const ideal = this.index.estimatedTotalLines() * ROW_HEIGHT_PX;
    spacer.style.height = `${Math.min(MAX_SPACER_HEIGHT_PX, ideal)}px`;
  }

  requestRepaint() {
    if (this.rafPending || this.stopped) return;
    this.rafPending = true;
    requestAnimationFrame(() => {
      this.rafPending = false;
      if (this.stopped) return;
      this.updateVisible();
    });
  }

  updateVisible() {
    const scrollTop = viewport.scrollTop;
    const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT_PX) - 2);
    const last = first + this.poolSize - 1;

    // Translate the visible line window into a byte range, then dispatch the chunks
    // that cover it plus the configured read-ahead in each direction.
    const firstByte = this.index.byteAt(first);
    const lastByte = this.index.byteAt(last);
    const firstChunk = Math.max(
      0,
      Math.floor(firstByte / CHUNK_SIZE) * CHUNK_SIZE - READ_AHEAD_CHUNKS * CHUNK_SIZE,
    );
    const lastChunk =
      Math.floor(lastByte / CHUNK_SIZE) * CHUNK_SIZE + READ_AHEAD_CHUNKS * CHUNK_SIZE;
    for (let c = firstChunk; c <= lastChunk; c += CHUNK_SIZE) {
      this.fetchChunkAt(c);
    }

    if (first !== this.lastRange.first || last !== this.lastRange.last) {
      this.lastRange = { first, last };
    }
    this.paintRows(first, last);
  }

  async fetchChunkAt(chunkStart) {
    if (this.stopped) return;
    if (chunkStart >= this.index.totalSize) return;
    if (this.fetched.has(chunkStart) || this.inflight.has(chunkStart)) return;
    const p = (async () => {
      try {
        const end = Math.min(this.index.totalSize, chunkStart + CHUNK_SIZE);
        const chunk = await renderRange(this.srcUrl, chunkStart, end);
        if (this.stopped) return;
        const baseLine = this.index.ingest(chunk);
        if (baseLine !== null) {
          for (let i = 0; i < chunk.lines.length; i++) {
            this.lineCache.set(baseLine + i, chunk.lines[i]);
          }
          this.evictIfNeeded();
        }
        this.fetched.add(chunkStart);
        this.updateSpacerHeight();
        this.updateVisible();
      } catch (e) {
        setStatus(`fetch error: ${e.message ?? e}`);
        console.error("chunk fetch failed", chunkStart, e);
      } finally {
        this.inflight.delete(chunkStart);
      }
    })();
    this.inflight.set(chunkStart, p);
  }

  evictIfNeeded() {
    if (this.lineCache.size <= RENDER_CACHE_LINES) return;
    const drop = this.lineCache.size - RENDER_CACHE_LINES;
    let i = 0;
    for (const key of this.lineCache.keys()) {
      if (i >= drop) break;
      this.lineCache.delete(key);
      i++;
    }
  }

  paintRows(first, last) {
    for (let i = 0; i < this.poolSize; i++) {
      const line = first + i;
      const row = this.rowPool[i];
      row.style.top = `${line * ROW_HEIGHT_PX}px`;
      const data = this.lineCache.get(line);
      if (data) {
        // Use a stable signature so we don't rebuild the row's DOM when the line
        // hasn't changed.
        const sig = `r${line}`;
        if (row.dataset.sig !== sig) {
          renderSegments(row, data.segments);
          row.className = "row rendered";
          row.dataset.sig = sig;
        }
      } else {
        const sig = `p${line}`;
        if (row.dataset.sig !== sig) {
          row.replaceChildren();
          row.textContent = "…";
          row.className = "row placeholder";
          row.dataset.sig = sig;
        }
      }
    }
  }
}

// --- segment painter ---

function renderSegments(row, segments) {
  row.replaceChildren();
  for (const seg of segments) {
    if (!seg.class && !seg.style) {
      // Plain text segment — append as a text node to avoid wrapping in a span.
      row.appendChild(document.createTextNode(seg.text));
      continue;
    }
    const span = document.createElement("span");
    if (seg.class) span.className = seg.class;
    if (seg.style) span.style.cssText = seg.style;
    span.textContent = seg.text;
    row.appendChild(span);
  }
}

// --- xterm 256-color palette ---

// Inject a baseline palette so the segments coming back from the server land on
// reasonable colors out of the box. Theme stylesheets in /themes/*.css can override
// any of these classes for a custom look. We do this in JS rather than ship a
// 1500-line CSS file because the palette is fully deterministic.
function injectPalette() {
  const palette = xterm256();
  const parts = [];
  for (let i = 0; i < palette.length; i++) {
    parts.push(`.hl-fg-${i}{color:${palette[i]}}.hl-bg-${i}{background-color:${palette[i]}}`);
  }
  const style = document.createElement("style");
  // Inject as the first stylesheet so theme CSS files (loaded later in the HTML)
  // win on equal-specificity ties.
  style.textContent = parts.join("\n");
  document.head.insertBefore(style, document.head.firstChild);
}

function xterm256() {
  const out = new Array(256);
  // 0–15: classic system colors (xterm defaults; many terminals override these).
  const sys = [
    "#000000", "#cd0000", "#00cd00", "#cdcd00", "#0000ee", "#cd00cd",
    "#00cdcd", "#e5e5e5", "#7f7f7f", "#ff0000", "#00ff00", "#ffff00",
    "#5c5cff", "#ff00ff", "#00ffff", "#ffffff",
  ];
  for (let i = 0; i < 16; i++) out[i] = sys[i];
  // 16–231: 6×6×6 color cube. Component values per xterm spec.
  const levels = [0, 95, 135, 175, 215, 255];
  for (let r = 0; r < 6; r++) {
    for (let g = 0; g < 6; g++) {
      for (let b = 0; b < 6; b++) {
        out[16 + r * 36 + g * 6 + b] = `#${hex(levels[r])}${hex(levels[g])}${hex(levels[b])}`;
      }
    }
  }
  // 232–255: 24-step grayscale ramp.
  for (let i = 0; i < 24; i++) {
    const v = 8 + i * 10;
    const h = hex(v);
    out[232 + i] = `#${h}${h}${h}`;
  }
  return out;
}

function hex(n) {
  return n.toString(16).padStart(2, "0");
}

// --- misc ---

function setStatus(text) {
  statusEl.textContent = text;
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MiB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}
