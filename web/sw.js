// PWA Service Worker：最小實作，支援「加入主畫面」
// 改前端後必須 bump 此 CACHE 版號，瀏覽器才會偵測 SW 更新 → install/activate → claim
// （比 index.html 的 ?v= 更深一層：即使 HTML ?v= 變了，若 SW 本身內容沒變，舊 SW 續命）
const CACHE = 'sw-shell-v20261118a25';
const CORE_ASSETS = [
  '/',
  '/index.html',
  '/style.css?v=20261118a25',
  '/rim-world.css?v=20261118a25',
  '/rim-sky-time.js?v=20261118a25',
  '/rim-portrait.js?v=20261118a25',
  '/rim-world.js?v=20261118a25',
  '/assets/pawn/m1_1_hat.png?v=20261118a25',
  '/assets/pawn/m1_2_head.png?v=20261118a25',
  '/assets/pawn/m1_4_pack.png?v=20261118a25',
  '/assets/pawn/m2_5_boot_top.png?v=20261118a25',
  '/assets/pawn/m1_8_bamboo.png?v=20261118a25',
  '/assets/pawn/m1_9_rock.png?v=20261118a25',
  '/assets/pawn/obj_campfire.png?v=20261118a25',
  '/assets/pawn/obj_workbench.png?v=20261118a25',
  '/assets/pawn/obj_stockpile.png?v=20261118a25',
  '/assets/pawn/obj_bridge.png?v=20261118a25',
  '/assets/pawn/obj_post.png?v=20261118a25',
  '/assets/pawn/obj_shoot.png?v=20261118a25',
  '/assets/pawn/item_shoot.png?v=20261118a25',
  '/assets/pawn/item_bamboo.png?v=20261118a25',
  '/assets/pawn/item_meat.png?v=20261118a25',
  '/assets/pawn/item_roast_shoot.png?v=20261118a25',
  '/assets/pawn/item_roast_meat.png?v=20261118a25',
  '/assets/pawn/item_blade.png?v=20261118a25',
  '/assets/pawn/item_pick.png?v=20261118a25',
  '/assets/pawn/beast_boar.png?v=20261118a25',
  '/assets/pawn/beast_pheasant.png?v=20261118a25',
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
    return;
  }
});
