// PWA Service Worker（v20261128r10：全島拓撲地名校直×跨地塊真實徒步行旅尺度×去方塊雜亂清爽野誌排版）
const CACHE = 'sw-shell-v20261128r10';
const CORE_ASSETS = [
  '/',
  '/index.html',
  '/island-ui.css?v=20261128r10',
  '/town-basemaps.js?v=20261128r10',
  '/static-node-map.js?v=20261128r10',
  '/island-ui.js?v=20261128r10',
  '/map-stamps.json?v=20261128r8',
  '/icon.svg',
  '/manifest.webmanifest'
];

self.addEventListener('install', function (e) {
  e.waitUntil(
    caches.open(CACHE).then(function (cache) {
      return cache.addAll(CORE_ASSETS).catch(function () {});
    }).then(function () {
      return self.skipWaiting();
    })
  );
});

self.addEventListener('activate', function (e) {
  e.waitUntil(
    caches.keys().then(function (keys) {
      return Promise.all(
        keys.filter(function (k) { return k !== CACHE; }).map(function (k) { return caches.delete(k); })
      );
    }).then(function () { return self.clients.claim(); })
  );
});

self.addEventListener('fetch', function (e) {
  if (e.request.method !== 'GET') return;
  var dest = e.request.destination;
  if (e.request.mode === 'navigate' || dest === 'document' || dest === 'script' || dest === 'style') {
    e.respondWith(
      fetch(e.request, { cache: 'reload' }).then(function (resp) {
        if (resp && resp.ok) {
          var copy = resp.clone();
          caches.open(CACHE).then(function (cache) { cache.put(e.request, copy); }).catch(function () {});
        }
        return resp;
      }).catch(function () {
        return caches.match(e.request).then(function (cached) {
          if (cached) return cached;
          if (e.request.mode === 'navigate' || dest === 'document') {
            return caches.match('/index.html');
          }
          return fetch(e.request);
        });
      })
    );
  }
});
