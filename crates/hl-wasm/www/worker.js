// Web worker that owns the hl WASM renderer.
//
// Two message types from the main thread:
//   - format_chunk: format every line in `bytes`, return `[line, html, ...]` flat array.
//   - search_chunk: format every line, derive plain text, find substring matches, return
//                   `[{line, ranges:[s,e,s,e,...]}, ...]` plus the chunk's line count.
// Doing both off the main thread keeps scroll smooth and lets search progress in parallel
// with paint (browser will round-robin tasks, but at least scroll handlers run unblocked).
import init, { init as wasmInit, format_line } from "./pkg/hl_wasm.js";

const ready = (async () => {
  await init();
  wasmInit();
  self.postMessage({ type: "ready" });
})();

function indexOf(buf, byte, from) {
  for (let i = from; i < buf.length; i++) if (buf[i] === byte) return i;
  return -1;
}

const decoder = new TextDecoder("utf-8", { fatal: false });

function renderLine(slice) {
  let html;
  try {
    html = format_line(slice);
  } catch (e) {
    html = `<span class="error">parse error: ${escapeHtml(e.message ?? String(e))}</span>`;
  }
  if (html.length === 0) {
    html = escapeHtml(decoder.decode(slice));
  }
  return html;
}

// Strip tags and decode common HTML entities so a substring search sees what the user sees.
// We do not rely on DOMParser (not available in workers); the renderer's entity set is small.
function htmlToPlain(html) {
  let s = html.replace(/<[^>]*>/g, "");
  s = s
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&nbsp;/g, " ");
  return s;
}

self.onmessage = async (ev) => {
  await ready;
  const msg = ev.data;
  if (msg.type === "format_chunk") {
    const { id, firstLine, bytes } = msg;
    const out = [];
    let cursor = 0;
    let line = firstLine;
    while (cursor < bytes.length) {
      const nl = indexOf(bytes, 0x0a, cursor);
      const end = nl < 0 ? bytes.length : nl;
      out.push(line, renderLine(bytes.subarray(cursor, end)));
      cursor = nl < 0 ? bytes.length : nl + 1;
      line += 1;
    }
    self.postMessage({ id, type: "format_chunk_result", out });
    return;
  }
  if (msg.type === "search_chunk") {
    const { id, firstLine, bytes, needle, caseInsensitive } = msg;
    const needleKey = caseInsensitive ? needle.toLowerCase() : needle;
    const needleLen = needle.length;
    const hits = [];
    let cursor = 0;
    let line = firstLine;
    while (cursor < bytes.length) {
      const nl = indexOf(bytes, 0x0a, cursor);
      const end = nl < 0 ? bytes.length : nl;
      const html = renderLine(bytes.subarray(cursor, end));
      const plain = htmlToPlain(html);
      const hay = caseInsensitive ? plain.toLowerCase() : plain;
      let p = 0;
      const ranges = [];
      while (true) {
        const idx = hay.indexOf(needleKey, p);
        if (idx < 0) break;
        ranges.push(idx, idx + needleLen);
        p = idx + Math.max(1, needleLen);
      }
      if (ranges.length > 0) hits.push({ line, ranges });
      cursor = nl < 0 ? bytes.length : nl + 1;
      line += 1;
    }
    self.postMessage({ id, type: "search_chunk_result", hits, lineCount: line - firstLine });
    return;
  }
};

function escapeHtml(s) {
  return s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}
