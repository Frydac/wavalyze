// Trunk generates a small JavaScript loader and the WebAssembly application binary.
// Their filenames contain a content hash, so a changed build gets a new URL while unchanged
// files can safely stay cached. This worker keeps the latest page available offline and caches
// those generated files after their first controlled online load.
const CACHE_NAME = 'wavalyze-v2';
const LEGACY_CACHE_PREFIX = 'egui-template-pwa';

// Activate this worker as soon as installation finishes instead of waiting for old tabs to close.
self.addEventListener('install', event => {
  event.waitUntil(self.skipWaiting());
});

// Remove caches created by the previous worker, then take control of open pages.
self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys()
      .then(names => Promise.all(
        names
          .filter(name => name.startsWith(LEGACY_CACHE_PREFIX))
          .map(name => caches.delete(name))
      ))
      .then(() => self.clients.claim())
  );
});

// HTML must come from the network when possible so it references the newest hashed JS and WASM.
// When offline, fall back to the last HTML response saved in this cache.
async function networkFirst(request) {
  const cache = await caches.open(CACHE_NAME);

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const response = await cache.match(request);
    if (response) {
      return response;
    }
    throw error;
  }
}

// Hashed files are immutable: changed contents produce a different URL. Reusing a cached response
// is therefore safe and avoids downloading the usually large WASM binary on every page load.
async function cacheFirst(request) {
  const cache = await caches.open(CACHE_NAME);
  const cachedResponse = await cache.match(request);
  if (cachedResponse) {
    return cachedResponse;
  }

  const response = await fetch(request);
  if (response.ok) {
    await cache.put(request, response.clone());
  }
  return response;
}

self.addEventListener('fetch', event => {
  // Never intercept uploads or other requests which may change server state.
  if (event.request.method !== 'GET') {
    return;
  }

  const url = new URL(event.request.url);

  // Browser navigations may request either the site root or index.html.
  const isIndex = url.origin === self.location.origin && url.pathname.endsWith('/index.html');
  if (event.request.mode === 'navigate' || isIndex) {
    event.respondWith(networkFirst(event.request));
    return;
  }

  // This also matches wasm-bindgen-rayon's worker script because its parent directory is hashed.
  const isCode = url.pathname.endsWith('.js') || url.pathname.endsWith('.wasm');
  const hasContentHash = /-[0-9a-f]{16,}(?:[._/]|$)/.test(url.pathname);
  if (url.origin === self.location.origin && isCode && hasContentHash) {
    event.respondWith(cacheFirst(event.request));
  }

  // No respondWith call means icons, manifests, and unrelated requests use normal browser loading.
});
