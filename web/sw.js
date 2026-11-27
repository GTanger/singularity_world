// PWA Service Worker（v20261127r3：修正行動端鏡頭焦點 focusY 精準對齊碼頭與廣場）
const CACHE = 'sw-shell-v20261127r3';
const CORE_ASSETS = [
  '/',
  '/index.html',
  '/island-ui.css?v=20261127r3',
  '/town-basemaps.js?v=20261127r3',
  '/static-node-map.js?v=20261127r3',
  '/island-ui.js?v=20261127r3',
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
