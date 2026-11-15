// Preloading: the engine and every short sound are downloaded while the prototype notice is up, with real
// progress, so nothing waits for its file the first time it plays. Music and wind are long and stream on their
// own. Sounds are kept as bytes here and decoded once audio is allowed (the first click).

const bytes = new Map(); // path (no query) -> Promise<ArrayBuffer>
const key = (url) => new URL(url, location.href).pathname;

/// The bytes of a file, from the preload when it has it, else fetched now (and kept).
export function fileBytes(url) {
  const k = key(url);
  if (!bytes.has(k)) bytes.set(k, fetch(url).then((r) => { if (!r.ok) throw new Error(`${url}: ${r.status}`); return r.arrayBuffer(); }));
  const p = bytes.get(k);
  p.catch(() => bytes.delete(k));
  return p;
}

/// Downloads `urls` in parallel; `onProgress(done bytes, total bytes, files done, files)` as they come in.
/// Resolves when all are in (a failed file is skipped: it is fetched again when first played).
export function preload(urls, onProgress = () => {}) {
  const sizes = new Map(), got = new Map();
  let files = 0;
  const report = () => {
    let total = 0, done = 0;
    for (const u of urls) { total += sizes.get(u) || 0; done += got.get(u) || 0; }
    onProgress(done, total, files, urls.length);
  };
  return Promise.all(urls.map(async (u) => {
    const k = key(u);
    try {
      const r = await fetch(u);
      if (!r.ok) throw new Error(String(r.status));
      sizes.set(u, Number(r.headers.get('content-length')) || 0);
      const reader = r.body && r.body.getReader();
      let buf;
      if (!reader) buf = await r.arrayBuffer();
      else {
        const parts = [];
        let n = 0;
        for (;;) {
          const { done, value } = await reader.read();
          if (done) break;
          parts.push(value); n += value.length;
          got.set(u, n); report();
        }
        const all = new Uint8Array(n);
        let at = 0;
        for (const p of parts) { all.set(p, at); at += p.length; }
        buf = all.buffer;
      }
      if (!sizes.get(u)) sizes.set(u, buf.byteLength);
      got.set(u, buf.byteLength);
      bytes.set(k, Promise.resolve(buf));
      return buf;
    } catch (_) {
      return null;
    } finally {
      files++; report();
    }
  }));
}
