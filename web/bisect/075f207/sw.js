// Offline cache for repeat visits. Every file a deploy stamps with ?v=<build> never changes under that name, so
// it is served from the cache straight away; when a newer build of the same file arrives, the older copy is
// dropped. The page itself and the version files always ask the network first (falling back to the cache when
// offline), so a new deploy is seen at once. Unstamped files are served from the cache and refreshed behind it.
const CACHE = 'scalia-files-1';

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => {
  e.waitUntil((async () => {
    for (const k of await caches.keys()) if (k !== CACHE) await caches.delete(k);
    await self.clients.claim();
  })());
});

const fresh = (url) => url.pathname.endsWith('/') || /\.(html|json)$/.test(url.pathname);

self.addEventListener('fetch', (e) => {
  const req = e.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== location.origin) return;
  // Long streamed sounds (music, wind) are left to the browser: range requests do not cache well.
  if (req.headers.has('range')) return;
  e.respondWith((async () => {
    const cache = await caches.open(CACHE);
    if (fresh(url)) {
      try {
        const r = await fetch(req, { cache: 'no-cache' });
        if (r.ok) cache.put(req, r.clone());
        return r;
      } catch (_) {
        return (await cache.match(req)) || Response.error();
      }
    }
    const hit = await cache.match(req);
    const stamped = url.searchParams.has('v');
    const update = fetch(req).then(async (r) => {
      if (r.ok && r.status === 200) {
        await cache.put(req, r.clone());
        // A new build of a file replaces the old one: drop other builds of the same path.
        if (stamped) {
          for (const k of await cache.keys()) {
            const u = new URL(k.url);
            if (u.pathname === url.pathname && u.search !== url.search) await cache.delete(k);
          }
        }
      }
      return r;
    }).catch(() => null);
    if (hit) {
      if (!stamped) e.waitUntil(update);
      return hit;
    }
    return (await update) || Response.error();
  })());
});
