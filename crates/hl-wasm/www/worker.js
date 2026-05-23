// Web worker that owns the hl WASM renderer.
//
// The main thread posts raw byte chunks (one per fetched range) along with the
// line index of the first line in the chunk. The worker line-splits the chunk,
// calls `format_line` once per record, and posts back a flat `[line, html, ...]`
// array. Doing this off the main thread keeps scroll smooth even when a chunk
// contains thousands of lines.
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

self.onmessage = async (ev) => {
  await ready;
  const msg = ev.data;
  if (msg.type !== "format_chunk") return;
  const { id, firstLine, bytes } = msg;
  const out = [];
  let cursor = 0;
  let line = firstLine;
  while (cursor < bytes.length) {
    const nl = indexOf(bytes, 0x0a, cursor);
    const end = nl < 0 ? bytes.length : nl;
    const slice = bytes.subarray(cursor, end);
    let html;
    try {
      html = format_line(slice);
    } catch (e) {
      html = `<span class="error">parse error: ${escapeHtml(e.message ?? String(e))}</span>`;
    }
    if (html.length === 0) {
      html = escapeHtml(new TextDecoder("utf-8", { fatal: false }).decode(slice));
    }
    out.push(line, html);
    cursor = nl < 0 ? bytes.length : nl + 1;
    line += 1;
  }
  self.postMessage({ id, type: "format_chunk_result", out });
};

function escapeHtml(s) {
  return s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}
