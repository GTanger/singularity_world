// PWA Service Worker：最小實作，支援「加入主畫面」
// 改前端後必須 bump 此 CACHE 版號，瀏覽器才會偵測 SW 更新 → install/activate → claim
// （比 index.html 的 ?v= 更深一層：即使 HTML ?v= 變了，若 SW 本身內容沒變，舊 SW 續命）
const CACHE = 'singularity-world-v52';

self.addEventListener('install', function (e) {
  self.skipWaiting();
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
  var dest = e.request.destination;
  if (e.request.mode === 'navigate' || dest === 'document' || dest === 'script' || dest === 'style') {
    e.respondWith(fetch(e.request, { cache: 'reload' }));
    return;
  }
});
