// Startup asset loader. Every required asset is fetched and checked; failures are returned, never hidden.
const bytes = new Map();
const key = (url) => new URL(url, location.href).pathname;
export function fileBytes(url) {
  const k = key(url);
  if (!bytes.has(k)) bytes.set(k, fetch(url).then((r) => {
    if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`);
    return r.arrayBuffer();
  }).then((buf) => {
    if (!buf.byteLength) throw new Error(`${url}: empty response`);
    return buf;
  }));
  const p = bytes.get(k);
  p.catch(() => bytes.delete(k));
  return p;
}
export async function preload(urls, onProgress = () => {}) {
  let completed = 0;
  const results = await Promise.all(urls.map(async (url) => {
    let result;
    try { result = { url, buffer: await fileBytes(url), ok: true, error: null }; }
    catch (error) { result = { url, buffer: null, ok: false, error: error?.message || String(error) }; }
    finally { completed++; onProgress({ completed, total: urls.length, url }); }
    return result;
  }));
  return { results, failures: results.filter((r) => !r.ok) };
}
export async function verifyFiles(urls, onProgress = () => {}) {
  const failures = [];
  let next = 0, completed = 0;
  const worker = async () => {
    while (next < urls.length) {
      const url = urls[next++];
      try {
        const response = await fetch(url, { method: 'HEAD', cache: 'no-cache' });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        if (response.headers.get('content-length') === '0') throw new Error('Empty asset');
      } catch (error) { failures.push({ url, error: error?.message || String(error) }); }
      finally { completed++; onProgress({ completed, total: urls.length, url }); }
    }
  };
  await Promise.all(Array.from({ length: Math.min(8, urls.length) }, worker));
  return failures;
}
