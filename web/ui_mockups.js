/**
 * ui_mockups.js — 白塔半島「靜態文字中的活世界流動感」三種跳脫框架互動原型
 * 相容於同源 CSP (script-src 'self')，直接驅動：
 *   典範一：行跡水墨場（步進穿行、駐留微態呼吸、擦肩低語、離場餘韻、方位推移過渡）
 *   典範二：當下三息・活書卷（萬物皆在行間、原位墨變、新墨深舊墨淡無捲軸三息流）
 *   典範三：時光潮汐・因果漣漪（六時刻天光紙色流轉、出攤收攤、渡船靠離、感官水紋）
 */
(function () {
  "use strict";

  var paused = false;

  /* ══════════════════════════════════════════════════════════════════
     典範一：行跡水墨場（空間步進穿行 ＋ 駐留微態 ＋ 擦肩低語 ＋ 離場餘韻 ＋ 移步換景）
     ══════════════════════════════════════════════════════════════════ */
  var p1Rooms = {
    bridge: {
      phasePlace: "正晝 ｜ 中心石橋",
      title: "文曜・中心樞紐石橋",
      atmosphere: "三水在此匯流，正晝江風微潤，橋頭炭火正溫。",
      exits: {
        north: { target: "north_st", label: "北・三水北街" },
        west:  { target: "west_st",  label: "西・三水西街" },
        east:  { target: "east_st",  label: "東・三水東街" },
        south: { target: "south_st", label: "南・三水南街" }
      },
      residents: [
        {
          id: "railing",
          name: "雕花石欄",
          kind: "obj",
          x: 28,
          y: 22,
          states: ["青石微潤", "江風拂欄", "水光映石"],
          actions: ["端詳", "憑欄遠眺", "撫摸石刻"],
          desc: "青石欄杆上雕著三水匯流紋，欄下江水湍急，水氣撲面微涼。"
        },
        {
          id: "sundial",
          name: "日晷竹牌",
          kind: "obj",
          x: 72,
          y: 22,
          states: ["晷針指午", "日影微斜", "竹牌輕晃"],
          actions: ["核對時刻", "端詳路牌"],
          desc: "青銅晷針投影在正晝刻度上，旁側竹牌指向東南西北四條長街。"
        },
        {
          id: "scholar",
          name: "青衫旅人",
          kind: "npc",
          x: 32,
          y: 44,
          states: ["憑欄望水", "整理衣袖", "遠眺北岸商船"],
          actions: ["端詳", "搭話", "贈物"],
          desc: "旅人正憑著青石欄杆遠眺北岸水色，見你走近，微一頷首：「今日春水初漲，北街商船比往常來得早。」"
        },
        {
          id: "angler",
          name: "垂釣老叟",
          kind: "npc",
          x: 70,
          y: 45,
          states: ["甩竿入水", "靜候浮標", "浮標微沉", "提簍收魚"],
          actions: ["端詳", "搭話", "看魚簍"],
          desc: "老叟握著老青竹釣竿，目光緊盯橋下碧波：「橋墩回水灣裡有大白魚，急不得。」"
        },
        {
          id: "stall",
          name: "炭烤肉攤",
          kind: "stall",
          x: 64,
          y: 74,
          states: ["煽火起炭", "牛油滴落紅炭", "刷上濃醬", "焦香漫橋"],
          actions: ["聞焦香", "嚐肉串", "與攤主搭話", "買賣"],
          desc: "原塊牛油滴落紅炭，混著紅泥煨番薯的焦甜香氣撲面而來。攤主阿火抬頭笑道：「新刷上醬汁，趁熱來一串？」"
        }
      ],
      /* 穿行過客週期：東入 -> 橋心交會 -> 西出 -> 離場餘韻 */
      passerSequence: [
        {
          phase: "enter",
          name: "挑擔藥農",
          note: "自東街步入",
          x: 68,
          y: 59,
          whisper: null,
          trace: null,
          desc: "藥農肩挑兩簍新採下的山參與當歸，正從三水東街踏上石橋。"
        },
        {
          phase: "center",
          name: "挑擔藥農",
          note: "橋心歇肩",
          x: 48,
          y: 58,
          whisper: "「北街藥鋪今日收山參嗎？」",
          trace: null,
          desc: "藥農在橋心微喘歇肩，低聲向身旁的青衫旅人打聽北街藥鋪收貨的行情。"
        },
        {
          phase: "leave",
          name: "挑擔藥農",
          note: "往西街行去",
          x: 32,
          y: 59,
          whisper: null,
          trace: null,
          desc: "藥農重新挑起竹簍，邁步朝三水西街方向走去。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 36,
          y: 60,
          whisper: null,
          trace: "餘韻：微苦山參草氣往西散去",
          desc: "挑擔藥農正往三水西街走遠，空氣裡還留著一縷微苦的新鮮山參草氣。"
        }
      ]
    },
    west_st: {
      phasePlace: "正晝 ｜ 三水西街",
      title: "文曜・三水西街",
      atmosphere: "青石街面兩側藥鋪與茶屋相連，空氣裡帶著焙茶與草藥氣息。",
      exits: {
        north: { target: "bridge", label: "北・內環北道" },
        west:  { target: "bridge", label: "西・西門石道" },
        east:  { target: "bridge", label: "東・中心石橋" },
        south: { target: "bridge", label: "南・公議堂前" }
      },
      residents: [
        {
          id: "herb_shop",
          name: "百草藥鋪",
          kind: "stall",
          x: 34,
          y: 28,
          states: ["切製山參", "竹篩曬藥", "藥香滿堂"],
          actions: ["聞藥香", "與主事搭話", "買賣藥材"],
          desc: "沈主事正在櫃前撥弄算盤，學徒在廊下竹篩翻曬新收的當歸片。"
        },
        {
          id: "herbalist_arrived",
          name: "挑擔藥農",
          kind: "npc",
          x: 56,
          y: 48,
          states: ["卸下竹簍", "過秤交貨", "清點鎂幣"],
          actions: ["端詳", "搭話", "查看竹簍"],
          desc: "正是前刻從中心石橋往西走來的那位藥農！他已在藥鋪廊下卸下竹簍，擦著額頭汗水。"
        },
        {
          id: "tea_guest",
          name: "飲茶書生",
          kind: "npc",
          x: 68,
          y: 70,
          states: ["捧盞吹茶", "翻閱旬報", "凝神思索"],
          actions: ["端詳", "搭話"],
          desc: "書生坐在臨街茶座旁，手裡捧著一盞初沖開的春融毛尖。"
        }
      ],
      passerSequence: [
        {
          phase: "enter",
          name: "巡街巡守",
          note: "自西門步入",
          x: 32,
          y: 62,
          whisper: null,
          trace: null,
          desc: "巡守按著腰間木牌，沿著西街青石板緩步巡視。"
        },
        {
          phase: "center",
          name: "巡街巡守",
          note: "駐足巡看",
          x: 48,
          y: 64,
          whisper: "「今日西街貨擔頗多，留神車馬。」",
          trace: null,
          desc: "巡守在街心停步，向藥鋪主事點頭招呼了一聲。"
        },
        {
          phase: "leave",
          name: "巡街巡守",
          note: "往石橋行去",
          x: 68,
          y: 62,
          whisper: null,
          trace: null,
          desc: "巡守轉身朝東側中心石橋方向行去。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 72,
          y: 62,
          whisper: null,
          trace: "餘韻：靴踏青石的輕響往東遠去",
          desc: "巡街巡守正朝中心石橋方向走遠，石板上餘下幾道淺淺濕印。"
        }
      ]
    },
    north_st: {
      phasePlace: "正晝 ｜ 三水北街",
      title: "文曜・三水北街",
      atmosphere: "臨近北灣貨船大埠與早午市集，商號旗幌隨江風輕擺。",
      exits: {
        north: { target: "bridge", label: "北・北灣貨船大埠" },
        west:  { target: "bridge", label: "西・三水商行" },
        east:  { target: "bridge", label: "東・文曜學院" },
        south: { target: "bridge", label: "南・中心石橋" }
      },
      residents: [
        {
          id: "noodle_stall",
          name: "切仔麵攤",
          kind: "stall",
          x: 36,
          y: 34,
          states: ["大骨熬湯", "竹笊甩麵", "白霧蒸騰"],
          actions: ["聞湯香", "點一碗麵", "與罔市姊搭話"],
          desc: "大鍋裡豬骨高湯翻滾，罔市姊熟練地用竹笊將切仔麵甩乾入碗。"
        },
        {
          id: "merchant",
          name: "背簍行商",
          kind: "npc",
          x: 68,
          y: 52,
          states: ["清點貨單", "捆紮布匹", "歇腳飲水"],
          actions: ["端詳", "搭話", "買賣"],
          desc: "行商甫從北岸渡船下來，正對著貨單清點背簍裡的山貨。"
        }
      ],
      passerSequence: [
        {
          phase: "center",
          name: "抱卷學子",
          note: "往東院行去",
          x: 52,
          y: 68,
          whisper: "「午後講堂快開課了。」",
          trace: null,
          desc: "學子懷抱書卷，匆匆往東側文曜學院趕去。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 64,
          y: 68,
          whisper: null,
          trace: "餘韻：松煙墨香往學院方向散去",
          desc: "學子正轉入東側街口，留下淡淡松煙墨氣。"
        }
      ]
    },
    east_st: {
      phasePlace: "正晝 ｜ 三水東街",
      title: "文曜・三水東街",
      atmosphere: "東街連接鐵坊與集市，偶有鍛鐵清音順風傳來。",
      exits: {
        north: { target: "bridge", label: "北・文曜學院" },
        west:  { target: "bridge", label: "西・中心石橋" },
        east:  { target: "bridge", label: "東・東門大道" },
        south: { target: "bridge", label: "南・第一鐵坊" }
      },
      residents: [
        {
          id: "pepper_bun",
          name: "胡椒餅攤",
          kind: "stall",
          x: 64,
          y: 36,
          states: ["貼爐烘烤", "芝麻焦香", "敲爐出餅"],
          actions: ["聞餅香", "買胡椒餅", "搭話"],
          desc: "陶缸炭爐壁上貼滿金黃酥脆的蔥香胡椒餅，熱氣混著黑胡椒辛香撲鼻。"
        },
        {
          id: "smith_apprentice",
          name: "鐵坊學徒",
          kind: "npc",
          x: 34,
          y: 56,
          states: ["搬運生鐵", "擦拭汗水", "挑揀木炭"],
          actions: ["端詳", "搭話"],
          desc: "學徒挽著袖子，正把一筐木炭往南側第一鐵坊搬去。"
        }
      ],
      passerSequence: [
        {
          phase: "center",
          name: "挑擔藥農",
          note: "正往石橋去",
          x: 46,
          y: 68,
          whisper: null,
          trace: null,
          desc: "藥農挑著新採的山參，正沿東街往中心石橋方向走。"
        }
      ]
    },
    south_st: {
      phasePlace: "正晝 ｜ 三水南街",
      title: "文曜・三水南街",
      atmosphere: "南街通往公議堂與南岸水埠，街面寬闊整潔。",
      exits: {
        north: { target: "bridge", label: "北・中心石橋" },
        west:  { target: "bridge", label: "西・公議堂" },
        east:  { target: "bridge", label: "東・第一鐵坊" },
        south: { target: "bridge", label: "南・南岸水埠" }
      },
      residents: [
        {
          id: "herbal_tea",
          name: "青草茶攤",
          kind: "stall",
          x: 42,
          y: 38,
          states: ["冰鎮銅壺", "斟滿涼茶", "薄荷清香"],
          actions: ["飲涼茶", "與攤主搭話", "買賣"],
          desc: "銅壺裡盛著早先熬好放涼的百草薄荷茶，喝一口暑氣全消。"
        },
        {
          id: "clerk",
          name: "議堂文書",
          kind: "npc",
          x: 66,
          y: 58,
          states: ["夾著卷宗", "核對告示", "緩步思索"],
          actions: ["端詳", "搭話"],
          desc: "文書腋下夾著一疊田畝卷宗，正準備回西側公議堂歸檔。"
        }
      ],
      passerSequence: [
        {
          phase: "center",
          name: "挑水腳夫",
          note: "自水埠步入",
          x: 48,
          y: 72,
          whisper: null,
          trace: null,
          desc: "腳夫挑著兩桶清水自南岸水埠走來，扁擔吱呀作響。"
        }
      ]
    }
  };

  var p1State = {
    currentRoom: "bridge",
    prevRoomLabel: null,
    prevDirLabel: null,
    tick: 0,
    passerIdx: 0,
    activePopover: null
  };

  function renderP1Room(shiftClass) {
    var room = p1Rooms[p1State.currentRoom];
    var canvas = document.getElementById("p1-canvas");
    if (!canvas || !room) return;

    var phaseEl = document.getElementById("p1-phase-place");
    var titleEl = document.getElementById("p1-title");
    var atmoEl = document.getElementById("p1-atmo");
    var trailOriginEl = document.getElementById("p1-trail-origin");

    if (phaseEl) phaseEl.textContent = room.phasePlace;
    if (titleEl) titleEl.textContent = room.title;
    if (atmoEl) atmoEl.textContent = room.atmosphere;

    if (trailOriginEl) {
      if (p1State.prevRoomLabel) {
        trailOriginEl.textContent = "前刻自「" + p1State.prevRoomLabel + "」步入";
        trailOriginEl.hidden = false;
      } else {
        trailOriginEl.hidden = true;
      }
    }

    /* 更新四極邊籤 */
    ["north", "west", "east", "south"].forEach(function (dir) {
      var btn = document.getElementById("p1-exit-" + dir);
      if (btn && room.exits[dir]) {
        btn.textContent = room.exits[dir].label;
        btn.setAttribute("data-target", room.exits[dir].target);
        btn.setAttribute("data-dir", dir);
      }
    });

    /* 渲染在場常駐實體與其呼吸微態 */
    var stage = document.getElementById("p1-entities-layer");
    if (!stage) return;

    if (shiftClass) {
      stage.classList.remove("shift-from-west", "shift-from-east", "shift-from-north", "shift-from-south");
      void stage.offsetWidth;
      stage.classList.add(shiftClass);
    }

    stage.innerHTML = "";
    room.residents.forEach(function (ent, idx) {
      var stateText = ent.states[(p1State.tick + idx) % ent.states.length];
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "ink-entity is-" + ent.kind;
      btn.style.left = ent.x + "%";
      btn.style.top = ent.y + "%";

      var nameSpan = document.createElement("span");
      nameSpan.className = "ent-name";
      nameSpan.textContent = ent.name;

      var stateSpan = document.createElement("span");
      stateSpan.className = "ent-micro-state";
      stateSpan.textContent = stateText;

      btn.appendChild(nameSpan);
      btn.appendChild(stateSpan);

      btn.addEventListener("click", function () {
        openP1Popover(ent.name, stateText, ent.actions, ent.desc);
      });

      stage.appendChild(btn);
    });

    renderP1Passer();
  }

  function renderP1Passer() {
    var room = p1Rooms[p1State.currentRoom];
    var passerLayer = document.getElementById("p1-passer-layer");
    if (!passerLayer || !room) return;

    var seq = room.passerSequence;
    var step = seq[p1State.passerIdx % seq.length];
    passerLayer.innerHTML = "";

    if (step.name) {
      var pBtn = document.createElement("button");
      pBtn.type = "button";
      pBtn.className = "ink-entity is-npc is-passer phase-" + step.phase;
      pBtn.style.left = step.x + "%";
      pBtn.style.top = step.y + "%";

      var nSpan = document.createElement("span");
      nSpan.className = "ent-name";
      nSpan.textContent = step.name;

      var dirSpan = document.createElement("span");
      dirSpan.className = "ent-passer-cadence";
      dirSpan.textContent = step.note;

      pBtn.appendChild(nSpan);
      pBtn.appendChild(dirSpan);

      pBtn.addEventListener("click", function () {
        openP1Popover(
          step.name + "（駐足停步）",
          step.note,
          ["端詳", "攔住搭話", "詢問去向"],
          step.desc + "（因你上前搭話，對方暫時停下了腳步。）"
        );
      });
      passerLayer.appendChild(pBtn);

      if (step.whisper) {
        var wBtn = document.createElement("button");
        wBtn.type = "button";
        wBtn.className = "ink-whisper-thread";
        wBtn.style.left = "46%";
        wBtn.style.top = "51%";
        wBtn.textContent = step.whisper;
        wBtn.addEventListener("click", function () {
          openP1Popover(
            "橋心擦肩低語",
            "瞬態交會契機",
            ["湊近旁聽", "插話打聽"],
            "你及時湊近聽見兩人交談——青衫旅人低聲道：「北街百草藥鋪沈主事今日正急收三年野山參，出價比平日高出三成。」挑擔藥農聞言喜道：「那我這便趕往三水西街去！」"
          );
        });
        passerLayer.appendChild(wBtn);
      }
    } else if (step.trace) {
      var tBtn = document.createElement("button");
      tBtn.type = "button";
      tBtn.className = "ink-residual-trace";
      tBtn.style.left = step.x + "%";
      tBtn.style.top = step.y + "%";
      tBtn.textContent = "· " + step.trace + " ·";
      tBtn.addEventListener("click", function () {
        openP1Popover(
          "離場餘韻與痕跡",
          "前刻留存之跡",
          ["辨認氣息", "往西追上"],
          step.desc
        );
      });
      passerLayer.appendChild(tBtn);
    }
  }

  function openP1Popover(title, subtitle, actions, desc) {
    p1State.activePopover = title;
    var pop = document.getElementById("p1-popover");
    var canvas = document.getElementById("p1-canvas");
    if (!pop) return;

    document.getElementById("p1-pop-title").textContent = title;
    document.getElementById("p1-pop-sub").textContent = subtitle || "";
    document.getElementById("p1-pop-log").textContent = desc;

    var actWrap = document.getElementById("p1-pop-actions");
    actWrap.innerHTML = "";
    actions.forEach(function (act, idx) {
      var pill = document.createElement("button");
      pill.type = "button";
      pill.className = "pop-pill" + (idx === 0 ? " active" : "");
      pill.textContent = act;
      pill.addEventListener("click", function () {
        Array.from(actWrap.children).forEach(function (c) { c.classList.remove("active"); });
        pill.classList.add("active");
        if (act === "往西追上") {
          closeP1Popover();
          moveP1Room("west", "west_st");
        }
      });
      actWrap.appendChild(pill);
    });

    pop.classList.add("is-open");
    if (canvas) canvas.classList.add("has-popover");
  }

  function closeP1Popover() {
    p1State.activePopover = null;
    var pop = document.getElementById("p1-popover");
    var canvas = document.getElementById("p1-canvas");
    if (pop) pop.classList.remove("is-open");
    if (canvas) canvas.classList.remove("has-popover");
  }

  function moveP1Room(dir, targetRoom) {
    var oldRoom = p1Rooms[p1State.currentRoom];
    if (!oldRoom || !p1Rooms[targetRoom]) return;
    p1State.prevRoomLabel = oldRoom.title.replace("文曜・", "");
    p1State.currentRoom = targetRoom;
    p1State.passerIdx = 0;
    closeP1Popover();

    var shiftMap = {
      west: "shift-from-west",
      east: "shift-from-east",
      north: "shift-from-north",
      south: "shift-from-south"
    };
    renderP1Room(shiftMap[dir] || "shift-from-west");
  }

  /* ══════════════════════════════════════════════════════════════════
     典範二：當下三息・活書卷（萬物皆在行間、原位墨變、新墨深舊墨淡無捲軸）
     ══════════════════════════════════════════════════════════════════ */
  var p2Sentences = [
    {
      scholarVerb: "正憑欄遠眺北岸春水",
      anglerVerb: "揚手將青竹釣線甩入碧波",
      stallVerb: "煽火起炭，原塊牛油滴落紅炭"
    },
    {
      scholarVerb: "轉頭望向橋心來往行旅",
      anglerVerb: "屏息盯著水面微沉的浮標",
      stallVerb: "在滋滋作響的肉串上刷滿濃醬"
    },
    {
      scholarVerb: "微一頷首，拂去袖上江風水氣",
      anglerVerb: "猛地提竿，將一尾白魚收入竹簍",
      stallVerb: "翻動紅泥煨番薯，焦甜香氣四溢"
    }
  ];

  var p2BreathsPool = [
    {
      html: '一名 <button type="button" class="inline-ink is-npc" data-target="herbalist">挑擔藥農</button> 自東街踏上石橋，竹簍裡飄出微苦山參香。'
    },
    {
      html: '<button type="button" class="inline-ink is-npc" data-target="herbalist">挑擔藥農</button> 行經橋心，向 <button type="button" class="inline-ink is-npc" data-target="scholar">青衫旅人</button> 低聲打聽北街藥鋪行情。'
    },
    {
      html: '南側 <button type="button" class="inline-ink is-stall" data-target="stall">炭烤肉攤</button> 爆開一粒火星，牛油焦香隨江風漫過橋面。'
    },
    {
      html: '藥農挑擔往西街遠去，石階上餘下一縷淡淡山參草氣與新踏濕印。'
    },
    {
      html: '北灣貨船大埠傳來竹篙點水聲，<button type="button" class="inline-ink is-npc" data-target="angler">垂釣老叟</button> 身旁的魚簍微微晃動。'
    }
  ];

  var p2Entities = {
    railing: {
      title: "雕花石欄",
      sub: "橋北臨水",
      actions: ["端詳", "憑欄遠眺"],
      desc: "青石欄杆被三水川的水氣浸得微涼，低頭可見橋孔下碧波旋轉匯流。"
    },
    sundial: {
      title: "日晷竹牌",
      sub: "橋頭報時",
      actions: ["核對晷影", "看路牌"],
      desc: "銅針投影正移過正午 11:15，竹牌上以松煙墨寫著四方街廓去向。"
    },
    scholar: {
      title: "青衫旅人",
      sub: "行間在場人物",
      actions: ["端詳", "搭話", "贈物"],
      desc: "旅人見你目光投來，溫言笑道：「在橋上站了一刻鐘，來往商旅比昨日多了不少。」"
    },
    angler: {
      title: "垂釣老叟",
      sub: "行間在場人物",
      actions: ["端詳", "搭話", "看魚簍"],
      desc: "老叟掀開竹簍一角，裡面已有兩尾活蹦亂跳的三水白魚。"
    },
    stall: {
      title: "炭烤肉攤",
      sub: "橋南爐火",
      actions: ["聞焦香", "嚐肉串", "搭話", "買賣"],
      desc: "攤主阿火笑道：「初出爐的炭烤牛肉配紅泥煨番薯，熱騰騰最補氣力！」"
    },
    herbalist: {
      title: "挑擔藥農",
      sub: "穿橋過客（已駐足）",
      actions: ["端詳", "攔住搭話", "買山參"],
      desc: "藥農停下腳步，擦汗道：「新從北嶺腳採下的野山參，正要送去西街百草藥鋪。」"
    }
  };

  var p2State = {
    morphIdx: 0,
    breathCursor: 2
  };

  function updateP2Morph() {
    var s = p2Sentences[p2State.morphIdx % p2Sentences.length];
    var v1 = document.getElementById("p2-verb-scholar");
    var v2 = document.getElementById("p2-verb-angler");
    var v3 = document.getElementById("p2-verb-stall");

    [v1, v2, v3].forEach(function (el) {
      if (el) el.classList.add("ink-morphing");
    });

    setTimeout(function () {
      if (v1) v1.textContent = s.scholarVerb;
      if (v2) v2.textContent = s.anglerVerb;
      if (v3) v3.textContent = s.stallVerb;
      [v1, v2, v3].forEach(function (el) {
        if (el) el.classList.remove("ink-morphing");
      });
    }, 320);
  }

  function renderP2Breaths() {
    var box = document.getElementById("p2-breaths-list");
    if (!box) return;

    var b0 = p2BreathsPool[p2State.breathCursor % p2BreathsPool.length];
    var b1 = p2BreathsPool[(p2State.breathCursor - 1 + p2BreathsPool.length) % p2BreathsPool.length];
    var b2 = p2BreathsPool[(p2State.breathCursor - 2 + p2BreathsPool.length) % p2BreathsPool.length];

    box.innerHTML =
      '<p class="breath-line breath-fresh">' + b0.html + '</p>' +
      '<p class="breath-line breath-mid">' + b1.html + '</p>' +
      '<p class="breath-line breath-faint">' + b2.html + '</p>';

    bindP2InlineWords(box);
  }

  function bindP2InlineWords(root) {
    var btns = root.querySelectorAll(".inline-ink[data-target]");
    btns.forEach(function (btn) {
      btn.addEventListener("click", function () {
        var key = btn.getAttribute("data-target");
        var data = p2Entities[key];
        if (!data) return;
        var pop = document.getElementById("p2-popover");
        var page = document.getElementById("p2-page");
        document.getElementById("p2-pop-title").textContent = data.title;
        document.getElementById("p2-pop-sub").textContent = data.sub;
        document.getElementById("p2-pop-log").textContent = data.desc;
        var actWrap = document.getElementById("p2-pop-actions");
        actWrap.innerHTML = "";
        data.actions.forEach(function (act, i) {
          var p = document.createElement("span");
          p.className = "pop-pill" + (i === 0 ? " active" : "");
          p.textContent = act;
          actWrap.appendChild(p);
        });
        if (pop) pop.classList.add("is-open");
        if (page) page.classList.add("has-popover");
      });
    });
  }

  /* ══════════════════════════════════════════════════════════════════
     典範三：時光潮汐・因果漣漪（晝夜紙色流轉、出攤收攤、渡船週期、感官水紋）
     ══════════════════════════════════════════════════════════════════ */
  var p3Phases = [
    {
      key: "dawn",
      label: "晨曉",
      clock: "晨曉 06:15 ｜ 中心石橋",
      prose: "三水川籠著一層薄薄江霧，早起漁舟正解開纜繩，石欄上露水未乾。",
      ferry: { name: "北岸渡船", state: "解纜起航・往南岸渡江", x: 32, y: 20 },
      water: { name: "三水川面", state: "晨霧漫江・水波微涼", x: 70, y: 24 },
      mid1:  { name: "垂釣老叟", state: "破曉佔位・初甩第一竿", x: 34, y: 46 },
      mid2:  { name: "早市菜農", state: "挑筐趕往北街早市", x: 68, y: 48 },
      south: { name: "炭烤肉攤", state: "木炭未燃・正在備料", x: 56, y: 74, isStall: true },
      trace: "· 橋面青石還留著昨夜露水與早市車轍 ·"
    },
    {
      key: "noon",
      label: "正晝",
      clock: "正晝 12:30 ｜ 中心石橋",
      prose: "正午日影直落晷盤，三水碧波耀眼，橋南炭火正旺，人流絡繹不絕。",
      ferry: { name: "北岸渡船", state: "北岸靠泊・商旅登船", x: 30, y: 20 },
      water: { name: "雕花石欄", state: "正晝日暖・江風微潤", x: 70, y: 22 },
      mid1:  { name: "青衫旅人", state: "憑欄遠眺・與過客交談", x: 32, y: 45 },
      mid2:  { name: "垂釣老叟", state: "浮標急沉・提竿收魚", x: 68, y: 46 },
      south: { name: "炭烤肉攤", state: "紅炭正旺・焦香往北漫開", x: 58, y: 73, isStall: true },
      trace: "· 炭爐牛油焦香隨江風吹向橋北 ·"
    },
    {
      key: "dusk",
      label: "黃昏",
      clock: "黃昏 17:40 ｜ 中心石橋",
      prose: "夕陽將三水川染成碎金暮赭色，早市行商陸續歸家，南街夜市燈火初起。",
      ferry: { name: "北岸渡船", state: "滿載歸客・回航靠岸", x: 32, y: 20 },
      water: { name: "日晷竹牌", state: "暮影拉長・斜映橋階", x: 70, y: 22 },
      mid1:  { name: "垂釣老叟", state: "收捲釣線・準備歸家", x: 34, y: 46 },
      mid2:  { name: "收工鐵匠", state: "搭伴往南街食堂用飯", x: 68, y: 48 },
      south: { name: "炭烤肉攤", state: "肉串將盡・餘炭微紅", x: 58, y: 73, isStall: true },
      trace: "· 踏板前留著歸船旅客未乾的江水腳印 ·"
    },
    {
      key: "night",
      label: "入夜",
      clock: "入夜 21:50 ｜ 中心石橋",
      prose: "夜幕低垂，橋頭貝母路燈亮起溫潤微光，白日喧囂散去，唯聞橋下水聲潺潺。",
      ferry: { name: "北岸渡船", state: "夜泊收槳・隨波輕晃", x: 30, y: 20 },
      water: { name: "貝母路燈", state: "燈暈溫潤・映照夜水", x: 70, y: 22 },
      mid1:  { name: "守夜更夫", state: "提燈巡橋・梆聲清遠", x: 46, y: 48 },
      mid2:  null,
      south: { name: "橋南空位", state: "肉攤已收・餘一撮冷燼", x: 56, y: 74, isStall: false },
      trace: "· 炭烤肉攤已收攤歸家，石縫間猶存一絲淡煙 ·"
    }
  ];

  var p3State = {
    phaseIdx: 1,
    autoCycle: true
  };

  function renderP3Phase(idx) {
    p3State.phaseIdx = idx % p3Phases.length;
    var p = p3Phases[p3State.phaseIdx];
    var screen = document.getElementById("p3-screen");
    if (!screen || !p) return;

    screen.setAttribute("data-tide", p.key);
    var clockEl = document.getElementById("p3-clock");
    var proseEl = document.getElementById("p3-prose");
    var traceEl = document.getElementById("p3-trace");
    if (clockEl) clockEl.textContent = p.clock;
    if (proseEl) proseEl.textContent = p.prose;
    if (traceEl) traceEl.textContent = p.trace;

    /* 更新時刻撥盤高亮 */
    var pills = screen.querySelectorAll(".tide-pill");
    pills.forEach(function (btn, i) {
      if (i === p3State.phaseIdx) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    });

    var stage = document.getElementById("p3-stage-nodes");
    if (!stage) return;
    stage.innerHTML = "";

    [p.ferry, p.water, p.mid1, p.mid2, p.south].forEach(function (item) {
      if (!item) return;
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "ink-entity " + (item.isStall ? "is-stall has-ripple" : "is-npc");
      btn.style.left = item.x + "%";
      btn.style.top = item.y + "%";

      var n = document.createElement("span");
      n.className = "ent-name";
      n.textContent = item.name;

      var s = document.createElement("span");
      s.className = "ent-micro-state";
      s.textContent = item.state;

      btn.appendChild(n);
      btn.appendChild(s);

      btn.addEventListener("click", function () {
        var pop = document.getElementById("p3-popover");
        document.getElementById("p3-pop-title").textContent = item.name;
        document.getElementById("p3-pop-sub").textContent = p.label + "・" + item.state;
        document.getElementById("p3-pop-log").textContent =
          p.label + "實況 — " + item.name + "當前處於「" + item.state + "」。世界隨六時刻真實作息流轉，不同時刻前來將撞見截然不同的人事與後果。";
        if (pop) pop.classList.add("is-open");
      });

      stage.appendChild(btn);
    });
  }
  /* ══════════════════════════════════════════════════════════════════
     融匯定案：活水墨卷（取三案之優、逐三案之劣的終局主畫面）
     ══════════════════════════════════════════════════════════════════ */
  var fTides = [
    {
      key: "dawn",
      label: "晨曉",
      clockText: "晨曉 06:15",
      atmo: "三水川籠著薄青晨霧，早起漁舟解纜起航，石欄露水微涼。",
      stallState: "木炭未燃",
      stallDesc: "阿火正在搬運炭簍與調味陶罐，原塊牛肉尚在井水冰鎮備料。",
      trace: "· 橋面青石還留著昨夜露水與早市車轍 ·",
      breath: "晨曉薄霧漫過三水石橋，早市菜農挑筐匆匆過橋，石階露水未乾。"
    },
    {
      key: "noon",
      label: "正晝",
      clockText: "正晝 12:30",
      atmo: "三水在此匯流，正晝江風微潤，橋頭炭火正溫。",
      stallState: "煽火起炭",
      stallDesc: "原塊牛油滴落紅炭，混著紅泥煨番薯的焦甜香氣撲面而來。攤主阿火笑道：「新刷上醬汁，趁熱來一串？」",
      trace: "· 炭爐牛油焦香隨江風吹向橋北 ·",
      breath: "正晝江風微潤，炭烤肉攤前炭火正溫，原塊牛油香氣漫過橋欄。"
    },
    {
      key: "dusk",
      label: "黃昏",
      clockText: "黃昏 17:40",
      atmo: "夕陽染成碎金暮赭，歸帆徐徐靠岸，南街夜市燈火初起。",
      stallState: "肉串將盡",
      stallDesc: "鐵架上肉串已所剩無幾，炭火漸漸收斂，阿火正用鐵夾撥弄紅炭。",
      trace: "· 踏板前留著歸船旅客未乾的江水腳印 ·",
      breath: "黃昏暮赭如金，北岸渡船載著歸客靠岸，老叟收竿滿簍歸去。"
    },
    {
      key: "night",
      label: "入夜",
      clockText: "入夜 21:50",
      atmo: "夜幕低垂，貝母路燈亮起溫潤微光，喧囂散去，唯聞水聲潺潺。",
      stallState: "肉攤已收・餘一撮冷燼",
      stallDesc: "肉攤已收攤歸家，石板地上留著一撮尚帶微溫的白灰炭燼與竹籤木箱。",
      trace: "· 炭烤肉攤已收攤歸家，石縫間猶存一絲淡煙 ·",
      breath: "入夜更夫提燈巡橋，梆聲清遠穿透水霧，橋南肉攤已收餘一撮冷燼。"
    }
  ];

  var fRooms = {
    bridge: {
      title: "文曜・中心樞紐石橋",
      placeShort: "中心石橋",
      exits: {
        north: { target: "north_st", label: "三水北街" },
        west:  { target: "west_st",  label: "三水西街" },
        east:  { target: "east_st",  label: "三水東街" },
        south: { target: "south_st", label: "三水南街" }
      },
      getResidents: function (tideKey, tick) {
        var isNight = (tideKey === "night");
        var isDawn = (tideKey === "dawn");
        var isDusk = (tideKey === "dusk");

        var list = [];
        // 物件 1：北石欄
        list.push({
          id: "railing",
          name: "雕花石欄",
          kind: "obj",
          x: 28,
          y: 18,
          states: isNight ? ["燈影映水", "夜水微涼", "青石凝露"] : ["青石微潤", "江風拂欄", "水光映石"],
          actions: ["端詳", "憑欄遠眺", "撫摸石刻"],
          desc: "青石欄杆上雕著三水匯流紋，欄下江水湍急，水氣撲面微涼。"
        });

        // 物件 2：日晷竹牌 vs 貝母路燈（入夜更替）
        if (isNight) {
          list.push({
            id: "lantern",
            name: "貝母路燈",
            kind: "obj",
            x: 72,
            y: 18,
            states: ["燈暈溫潤", "映照夜水", "竹油防風"],
            actions: ["端詳", "借光看路"],
            desc: "以東海磨光貝母為罩的長明路燈，透出柔和微黃的光暈，照亮橋頭青石。"
          });
        } else {
          list.push({
            id: "sundial",
            name: "日晷竹牌",
            kind: "obj",
            x: 72,
            y: 18,
            states: isDawn ? ["晨曦微斜", "竹牌沾露"] : (isDusk ? ["暮影斜長", "竹牌輕晃"] : ["晷針指午", "日影微斜", "竹牌輕晃"]),
            actions: ["核對時刻", "端詳路牌"],
            desc: "青銅晷針投影在刻度盤上，旁側竹牌指向東南西北四條長街。"
          });
        }

        // 人物 1：青衫旅人 vs 守夜更夫（入夜更替）
        if (isNight) {
          list.push({
            id: "watchman",
            name: "守夜更夫",
            kind: "npc",
            x: 32,
            y: 38,
            states: ["提燈巡橋", "敲擊梆子", "留神水信"],
            actions: ["端詳", "搭話", "問詢時刻"],
            desc: "更夫手持防風油燈與青銅梆子：「天乾物燥，小心火燭。行客夜深還在走動，留神腳下濕滑。」"
          });
        } else {
          list.push({
            id: "scholar",
            name: "青衫旅人",
            kind: "npc",
            x: 32,
            y: 38,
            states: ["憑欄望水", "整理衣袖", "遠眺北岸商船"],
            actions: ["端詳", "搭話", "贈物"],
            desc: "旅人正憑著青石欄杆遠眺北岸水色，見你走近，微一頷首：「今日春水初漲，北街商船比往常來得早。」"
          });
        }

        // 人物 2：垂釣老叟（入夜歸家）
        if (!isNight) {
          list.push({
            id: "angler",
            name: "垂釣老叟",
            kind: "npc",
            x: 70,
            y: 38,
            states: isDusk ? ["收捲釣線", "清點魚簍", "準備歸家"] : ["甩竿入水", "靜候浮標", "提竿收魚"],
            actions: ["端詳", "搭話", "看魚簍"],
            desc: "老叟握著老青竹釣竿，目光緊盯橋下碧波：「橋墩回水灣裡有大白魚，急不得。」"
          });
        }

        // 攤位：炭烤肉攤（入夜收攤化為橋南空位・餘一撮冷燼）
        if (isNight) {
          list.push({
            id: "stall_night",
            name: "橋南空位",
            kind: "obj",
            isStall: false,
            hasRipple: false,
            x: 64,
            y: 80,
            states: ["肉攤已收", "餘一撮冷燼", "石縫淡煙"],
            actions: ["撥弄炭灰", "端詳空地"],
            desc: "炭烤肉攤已在暮色降臨時收攤歸家，青石板上只餘下一撮尚帶餘溫的白灰炭燼與淡淡木煙香氣。"
          });
        } else {
          list.push({
            id: "stall",
            name: "炭烤肉攤",
            kind: "stall",
            isStall: true,
            hasRipple: (tideKey === "noon"),
            x: 64,
            y: 80,
            states: isDawn ? ["擺設木架", "引燃引火木柴", "擺放調味罐"] : (isDusk ? ["肉串將盡", "餘炭微紅", "準備收攤"] : ["煽火起炭", "牛油滴落紅炭", "刷上濃醬", "焦香漫橋"]),
            actions: ["聞焦香", "嚐肉串", "與攤主搭話", "買賣"],
            desc: "原塊牛油滴落紅炭，混著紅泥煨番薯的焦甜香氣撲面而來。攤主阿火笑道：「新刷上醬汁，趁熱來一串？」"
          });
        }

        return list;
      },
      passerSequence: [
        {
          phase: "enter",
          name: "挑擔藥農",
          note: "自東街步入",
          x: 72,
          y: 62,
          whisper: null,
          trace: null,
          desc: "藥農肩挑兩簍新採下的野山參與當歸，正從三水東街踏上石橋階梯。",
          breath: "挑擔藥農初刻自東街踏上石階，竹簍裡的山參草氣撲面微苦。"
        },
        {
          phase: "center",
          name: "挑擔藥農",
          note: "橋心歇肩",
          x: 48,
          y: 62,
          whisper: "「北街藥鋪今日收山參嗎？」",
          trace: null,
          desc: "藥農在橋心微喘歇肩，低聲向身旁的青衫旅人打聽西街百草藥鋪收貨行情。",
          breath: "藥農在橋心放下扁擔微喘歇肩，低語打聽著北街百草藥鋪的行情。"
        },
        {
          phase: "leave",
          name: "挑擔藥農",
          note: "往西街行去",
          x: 26,
          y: 62,
          whisper: null,
          trace: null,
          desc: "藥農重新挑起竹簍，邁步朝三水西街方向走去。",
          breath: "藥農重新挑起竹簍往三水西街快步走去，扁擔在微風中輕微作響。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 30,
          y: 62,
          whisper: null,
          trace: "餘韻：微苦山參草氣往西散去",
          desc: "挑擔藥農前刻往三水西街走遠，空氣裡還留著一縷微苦的新鮮山參草氣。",
          breath: "石階上藥農行跡漸遠，一縷微苦的山參草氣隨風往西街散去。"
        }
      ]
    },
    west_st: {
      title: "文曜・三水西街",
      placeShort: "三水西街",
      atmo: "青石街面兩側藥鋪與茶屋相連，空氣裡帶著焙茶與草藥氣息。",
      exits: {
        north: { target: "bridge", label: "內環北道" },
        west:  { target: "bridge", label: "西門石道" },
        east:  { target: "bridge", label: "中心石橋" },
        south: { target: "bridge", label: "公議堂前" }
      },
      getResidents: function (tideKey, tick) {
        return [
          {
            id: "herb_shop",
            name: "百草藥鋪",
            kind: "stall",
            isStall: true,
            hasRipple: false,
            x: 34,
            y: 22,
            states: ["切製山參", "竹篩曬藥", "藥香滿堂"],
            actions: ["聞藥香", "與主事搭話", "買賣藥材"],
            desc: "沈主事正在櫃前撥弄算盤，學徒在廊下竹篩翻曬新收的當歸片。"
          },
          {
            id: "herbalist_arrived",
            name: "挑擔藥農",
            kind: "npc",
            x: 64,
            y: 36,
            states: ["卸下竹簍", "過秤交貨", "清點鎂幣"],
            actions: ["端詳", "搭話", "查看竹簍"],
            desc: "正是前刻從中心石橋往西走來的那位藥農！他已在百草藥鋪前卸下竹簍，正喜孜孜地清點結清的鎂幣。"
          },
          {
            id: "tea_guest",
            name: "飲茶書生",
            kind: "npc",
            x: 68,
            y: 80,
            states: ["捧盞吹茶", "翻閱旬報", "凝神思索"],
            actions: ["端詳", "搭話"],
            desc: "書生坐在臨街茶座旁，手裡捧著一盞初沖開的春融毛尖。"
          }
        ];
      },
      passerSequence: [
        {
          phase: "center",
          name: "巡街巡守",
          note: "街心巡視",
          x: 44,
          y: 62,
          whisper: "「今日西街貨擔頗多，留神車馬。」",
          trace: null,
          desc: "巡守按著腰間木牌，在街心停步向藥鋪主事點頭招呼。",
          breath: "巡街巡守手按木牌緩步巡視，提醒街邊攤商留神來往車馬。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 64,
          y: 64,
          whisper: null,
          trace: "餘韻：靴踏青石的輕響往東遠去",
          desc: "巡守朝東側中心石橋方向走去，留下一陣沉穩的皮靴踏石聲。",
          breath: "巡守步伐沉穩地往東行去，靴底踏在青石板上的輕響隨風漸淡。"
        }
      ]
    },
    north_st: {
      title: "文曜・三水北街",
      placeShort: "三水北街",
      atmo: "北街緊鄰渡口與商行，往來挑夫與商旅絡繹不絕，骨湯香氣四溢。",
      exits: {
        north: { target: "bridge", label: "北灣貨船大埠" },
        west:  { target: "bridge", label: "三水商行" },
        east:  { target: "bridge", label: "文曜學院" },
        south: { target: "bridge", label: "中心石橋" }
      },
      getResidents: function (tideKey, tick) {
        return [
          {
            id: "noodle_stall",
            name: "切仔麵攤",
            kind: "stall",
            isStall: true,
            hasRipple: (tideKey === "noon"),
            x: 36,
            y: 34,
            states: ["大骨熬湯", "竹笊甩麵", "白霧蒸騰"],
            actions: ["聞湯香", "點一碗麵", "與罔市姊搭話"],
            desc: "大鍋裡豬骨高湯翻滾，罔市姊熟練地用竹笊將切仔麵甩乾入碗。"
          },
          {
            id: "merchant",
            name: "背簍行商",
            kind: "npc",
            x: 68,
            y: 52,
            states: ["清點貨單", "捆紮布匹", "歇腳飲水"],
            actions: ["端詳", "搭話", "買賣"],
            desc: "行商甫從北岸渡船下來，正對著貨單清點背簍裡的山貨。"
          }
        ];
      },
      passerSequence: [
        {
          phase: "center",
          name: "抱卷學子",
          note: "往學院疾行",
          x: 52,
          y: 66,
          whisper: null,
          trace: null,
          desc: "學子懷抱書卷，匆匆往東側文曜學院趕去。",
          breath: "抱卷學子行色匆匆，衣袂拂過麵攤蒸騰的骨湯白霧。"
        },
        {
          phase: "trace",
          name: null,
          note: null,
          x: 64,
          y: 68,
          whisper: null,
          trace: "餘韻：松煙墨香往學院方向散去",
          desc: "學子正轉入東側街口，留下淡淡松煙墨氣。",
          breath: "學子轉入東街長巷，空氣中留存著一縷清幽的松煙墨香。"
        }
      ]
    },
    east_st: {
      title: "文曜・三水東街",
      placeShort: "三水東街",
      atmo: "東街連接書院與鐵坊，遠處傳來清脆打鐵聲與烤餅焦香。",
      exits: {
        north: { target: "bridge", label: "文曜學院" },
        west:  { target: "bridge", label: "中心石橋" },
        east:  { target: "bridge", label: "東門大道" },
        south: { target: "bridge", label: "第一鐵坊" }
      },
      getResidents: function (tideKey, tick) {
        return [
          {
            id: "pepper_bun",
            name: "胡椒餅攤",
            kind: "stall",
            isStall: true,
            hasRipple: (tideKey === "noon"),
            x: 64,
            y: 36,
            states: ["貼爐烘烤", "芝麻焦香", "敲爐出餅"],
            actions: ["聞餅香", "買胡椒餅", "搭話"],
            desc: "陶缸炭爐壁上貼滿金黃酥脆的蔥香胡椒餅，熱氣混著黑胡椒辛香撲鼻。"
          },
          {
            id: "smith_apprentice",
            name: "鐵坊學徒",
            kind: "npc",
            x: 34,
            y: 56,
            states: ["搬運生鐵", "擦拭汗水", "挑揀木炭"],
            actions: ["端詳", "搭話"],
            desc: "學徒挽著袖子，正把一筐木炭往南側第一鐵坊搬去。"
          }
        ];
      },
      passerSequence: [
        {
          phase: "center",
          name: "挑擔藥農",
          note: "正往石橋去",
          x: 48,
          y: 68,
          whisper: null,
          trace: null,
          desc: "藥農挑著新採的山參，正沿東街往中心石橋方向走。",
          breath: "藥農挑著山參竹簍從東街緩步走來，腳步沉穩有力。"
        }
      ]
    },
    south_st: {
      title: "文曜・三水南街",
      placeShort: "三水南街",
      atmo: "南街通往水埠與公議堂，老槐樹蔭下涼風習習，茶攤銅壺微鳴。",
      exits: {
        north: { target: "bridge", label: "中心石橋" },
        west:  { target: "bridge", label: "公議堂" },
        east:  { target: "bridge", label: "第一鐵坊" },
        south: { target: "bridge", label: "南岸水埠" }
      },
      getResidents: function (tideKey, tick) {
        return [
          {
            id: "herbal_tea",
            name: "青草茶攤",
            kind: "stall",
            isStall: true,
            hasRipple: false,
            x: 42,
            y: 38,
            states: ["冰鎮銅壺", "斟滿涼茶", "薄荷清香"],
            actions: ["飲涼茶", "與攤主搭話", "買賣"],
            desc: "銅壺裡盛著早先熬好放涼的百草薄荷茶，喝一口暑氣全消。"
          },
          {
            id: "clerk",
            name: "議堂文書",
            kind: "npc",
            x: 66,
            y: 58,
            states: ["夾著卷宗", "核對告示", "緩步思索"],
            actions: ["端詳", "搭話"],
            desc: "文書腋下夾著一疊田畝卷宗，正準備回西側公議堂歸檔。"
          }
        ];
      },
      passerSequence: [
        {
          phase: "center",
          name: "挑水腳夫",
          note: "自水埠步入",
          x: 48,
          y: 72,
          whisper: null,
          trace: null,
          desc: "腳夫挑著兩桶清水自南岸水埠走來，扁擔吱呀作響。",
          breath: "挑水腳夫雙桶滿水，扁擔微顫間在青石板上灑落點點晶瑩水花。"
        }
      ]
    }
  };

  var fActionOutcomes = {
    "憑欄遠眺": { breath: "你扶著微涼的青石欄杆遠眺，北岸渡船正徐徐收帆靠岸，江風夾著水草氣息撲面而來。", state: "憑欄望江" },
    "撫摸石刻": { breath: "指尖滑過石欄上的三水匯流紋，石縫間凝著微涼江霧與歲月磨出的溫潤包漿。", state: "指觸石紋" },
    "核對時刻": { breath: "青銅晷針的日影正落在正午 11:15，竹牌隨江風輕晃，指向四方長街。", state: "晷影指午" },
    "借光看路": { breath: "貝母燈罩透出溫潤微黃的光暈，將橋頭濕滑的青石階照得清清楚楚。", state: "燈暈映階" },
    "搭話": { breath: "你上前頷首搭話，對方轉過身來與你寒暄兩句，談起今日三水街面的見聞與行情。", state: "與客交談" },
    "贈物": { breath: "你遞上隨身茶點致意，青衫旅人拱手微笑道謝，向你透露北岸商船帶來的緊俏貨訊。", state: "拱手道謝" },
    "看魚簍": { breath: "你湊近竹編魚簍一看，清水裡已有兩尾銀白江魚正擺尾游動，老叟得意地捻鬚微笑。", state: "魚簍有獲" },
    "嚐肉串": { breath: "你拈出兩枚鎂幣遞給阿火，接過燙手的竹籤炭烤牛肉——外焦裡嫩的肉汁混著辛香濃醬在口中化開。", state: "遞出熱肉串" },
    "買賣": { breath: "阿火俐落地打開竹編食盒，展示現烤原塊牛肉串與紅泥煨番薯，熱氣騰騰。", state: "招呼買賣" },
    "與攤主搭話": { breath: "阿火一邊翻動滋滋作響的肉串一邊笑道：「每日正晝過橋的商旅最多，這炭火得從晨光 07:30 一直旺到暮色 17:30！」", state: "翻動肉串" },
    "撥弄炭灰": { breath: "你蹲下輕撥白灰冷燼，底下猶存一絲微弱餘溫，足見白日炭火何等興旺。", state: "餘溫猶存" },
    "問詢時刻": { breath: "守夜更夫舉起防風油燈：「已是夜深 21:15，橋南攤販都歇了，行客早些投宿吧。」", state: "提燈報更" },
    "攔住搭話": { breath: "挑擔藥農擦了把汗笑道：「初自東山採下這兩簍野山參，聽說西街百草藥鋪今日收價好，正趕過去呢！」", state: "歇肩敘話" },
    "詢問去向": { breath: "藥農往西側一指：「這就去三水西街的百草藥鋪，行客若順路不妨一同走！」", state: "指向西街" },
    "往西追上": { moveWest: true }
  };

  var fState = {
    currentRoom: "bridge",
    prevRoomLabel: null,
    originDir: null,
    tideIdx: 1, // 預設正晝
    tick: 0,
    passerIdx: 0,
    passerPaused: false,
    focusedEntityId: null,
    focusUntil: 0,
    activePopover: null,
    breaths: [
      { text: "正晝江風微潤，炭烤肉攤前炭火正溫，原塊牛油香氣漫過橋欄。", byPlayer: false },
      { text: "挑擔藥農初刻自東街踏上石階，竹簍裡的山參草氣撲面微苦。", byPlayer: false }
    ]
  };

  function pushFBreath(text, byPlayer) {
    if (!text) return;
    var curTop = fState.breaths[0] ? fState.breaths[0].text : "";
    if (text === curTop) return;
    fState.breaths = [
      { text: text, byPlayer: !!byPlayer },
      fState.breaths[0]
    ];
    if (byPlayer) {
      fState.focusUntil = Date.now() + 6500;
    }
    renderFBreaths();
  }

  function renderFBreaths() {
    var box = document.getElementById("f-breaths-list");
    if (!box) return;
    var b0 = fState.breaths[0] || { text: "", byPlayer: false };
    var b1 = fState.breaths[1] || { text: "", byPlayer: false };
    var cls0 = "f-breath-line f-breath-fresh" + (b0.byPlayer ? " is-player-breath" : "");
    box.innerHTML =
      '<p class="' + cls0 + '" id="f-breath-0">' + b0.text + '</p>' +
      '<p class="f-breath-line f-breath-faint" id="f-breath-1">' + b1.text + '</p>';
  }
  function setFTide(idx, manual) {
    fState.tideIdx = (idx + fTides.length) % fTides.length;
    var tide = fTides[fState.tideIdx];
    var screen = document.getElementById("f-screen");
    if (!screen || !tide) return;

    screen.setAttribute("data-tide", tide.key);
    var tideTextEl = document.getElementById("f-tide-text");
    if (tideTextEl) tideTextEl.textContent = tide.clockText;

    var atmoEl = document.getElementById("f-atmo");
    if (atmoEl) {
      atmoEl.style.opacity = "0.2";
      setTimeout(function () {
        atmoEl.textContent = (fState.currentRoom === "bridge") ? tide.atmo : (fRooms[fState.currentRoom].atmo || fRooms[fState.currentRoom].title);
        atmoEl.style.opacity = "1";
      }, 220);
    }

    if (manual) {
      pushFBreath("時光推移至" + tide.label + "初刻，宣紙天光隨之更換，" + tide.trace.replace(/^·\s*|\s*·$/g, "") + "。");
    }

    renderFRoom(null);
  }

  function renderFRoom(shiftClass) {
    var room = fRooms[fState.currentRoom];
    var screen = document.getElementById("f-screen");
    if (!screen || !room) return;

    var tide = fTides[fState.tideIdx];
    var titleEl = document.getElementById("f-title");
    var atmoEl = document.getElementById("f-atmo");
    var placeShortEl = document.getElementById("f-place-short");
    var trailEl = document.getElementById("f-trail-origin");

    if (titleEl) titleEl.textContent = room.title;
    if (placeShortEl) placeShortEl.textContent = room.placeShort;
    if (atmoEl && !shiftClass) {
      atmoEl.textContent = (fState.currentRoom === "bridge") ? tide.atmo : (room.atmo || room.title);
    }

    if (trailEl) {
      trailEl.hidden = true;
    }

    // 更新四向步履列按鈕（由左至右：西｜北｜南｜東，並標示來時餘溫方位）
    ["west", "north", "south", "east"].forEach(function (dir) {
      var btn = document.getElementById("f-exit-" + dir);
      var destEl = document.getElementById("f-dest-" + dir);
      if (btn && room.exits[dir]) {
        if (destEl) destEl.textContent = room.exits[dir].label;
        btn.setAttribute("data-target", room.exits[dir].target);
        if (fState.originDir === dir) {
          btn.classList.add("is-origin-dir");
        } else {
          btn.classList.remove("is-origin-dir");
        }
      }
    });
    // 渲染在場常駐實體與呼吸微態
    var stage = document.getElementById("f-entities-layer");
    if (!stage) return;

    if (shiftClass) {
      stage.classList.remove("shift-from-west", "shift-from-east", "shift-from-north", "shift-from-south");
      void stage.offsetWidth;
      stage.classList.add(shiftClass);
    }

    var renderContextKey = fState.currentRoom + "|" + tide.key;
    var isSameContext = (stage.getAttribute("data-ctx") === renderContextKey) && !shiftClass;
    stage.setAttribute("data-ctx", renderContextKey);

    var residents = room.getResidents(tide.key, fState.tick);
    if (!isSameContext) {
      stage.innerHTML = "";
      residents.forEach(function (ent, idx) {
        var stateText = ent.states[(fState.tick + idx) % ent.states.length];
        var btn = document.createElement("button");
        btn.type = "button";
        btn.setAttribute("data-ent-id", ent.id);
        var classList = "ink-entity is-" + ent.kind;
        if (ent.isStall) classList += " is-stall";
        if (ent.hasRipple) classList += " has-ripple";
        btn.className = classList;
        btn.style.left = ent.x + "%";
        btn.style.top = ent.y + "%";

        var nameSpan = document.createElement("span");
        nameSpan.className = "ent-name";
        nameSpan.textContent = ent.name;

        var stateSpan = document.createElement("span");
        stateSpan.className = "ent-micro-state";
        stateSpan.textContent = stateText;

        btn.appendChild(nameSpan);
        btn.appendChild(stateSpan);
        var actionsWrap = document.createElement("span");
        actionsWrap.className = "ent-inline-actions";
        actionsWrap.hidden = true;

        var followActs = (ent.actions || []).filter(function (a) {
          return a !== "端詳" && a !== "聞焦香";
        }).slice(0, 2);

        followActs.forEach(function (actName) {
          var chip = document.createElement("span");
          chip.className = "ent-act-chip";
          chip.textContent = actName;
          chip.addEventListener("click", function (ev) {
            ev.stopPropagation();
            var out = fActionOutcomes[actName];
            if (out && out.moveWest) {
              moveFRoom("west", "west_st");
              return;
            }
            var breathMsg = (out && out.breath)
              ? out.breath
              : ("你近前與「" + ent.name + "」展開「" + actName + "」，周遭人事隨之回應。");
            pushFBreath(breathMsg, true);
            if (out && out.state) {
              stateSpan.textContent = out.state;
            }
            actionsWrap.hidden = true;
            stateSpan.hidden = false;
            fState.focusedEntityId = null;
          });
          actionsWrap.appendChild(chip);
        });

        btn.appendChild(actionsWrap);

        btn.addEventListener("click", function (ev) {
          ev.stopPropagation();
          /* 一觸即感：單擊直接將第一手鮮活感官推入當下二息，並原位展開 1～2 個後續微動作，零彈窗遮擋 */
          pushFBreath(ent.desc, true);
          Array.from(stage.querySelectorAll(".ink-entity")).forEach(function (other) {
            var os = other.querySelector(".ent-micro-state");
            var oa = other.querySelector(".ent-inline-actions");
            if (os) os.hidden = false;
            if (oa) oa.hidden = true;
          });
          if (followActs.length > 0) {
            stateSpan.hidden = true;
            actionsWrap.hidden = false;
            fState.focusedEntityId = ent.id;
          }
        });

        stage.appendChild(btn);
      });
    } else {
      /* Keyed DOM 原位更新：每次僅輪流推進 1 個未處於聚焦態的常駐微態，避免全場齊步跳字 */
      var activeSlot = fState.tick % residents.length;
      residents.forEach(function (ent, idx) {
        if (idx !== activeSlot || fState.focusedEntityId === ent.id) return;
        var btn = stage.querySelector('[data-ent-id="' + ent.id + '"]');
        if (!btn) return;
        var stateSpan = btn.querySelector(".ent-micro-state");
        if (stateSpan && !stateSpan.hidden) {
          stateSpan.textContent = ent.states[(fState.tick + idx) % ent.states.length];
        }
      });
    }

    renderFPasser(isSameContext);
  }

  function renderFPasser(isSameContext) {
    var room = fRooms[fState.currentRoom];
    var passerLayer = document.getElementById("f-passer-layer");
    if (!passerLayer || !room) return;

    var seq = room.passerSequence;
    if (!seq || seq.length === 0) {
      passerLayer.innerHTML = "";
      return;
    }

    var step = seq[fState.passerIdx % seq.length];

    /* 西向追人提示：當過客在中心石橋往西行去或留下西行餘韻時，點亮「西・三水西街」微光點 */
    var westBtn = document.getElementById("f-exit-west");
    if (westBtn) {
      if (fState.currentRoom === "bridge" && (step.phase === "leave" || step.phase === "trace")) {
        westBtn.classList.add("has-pursuit-hint");
      } else {
        westBtn.classList.remove("has-pursuit-hint");
      }
    }

    var existingPasser = passerLayer.querySelector('[data-role="f-passer"]');
    var oldWhisper = passerLayer.querySelector(".ink-whisper-thread");
    if (oldWhisper) oldWhisper.remove();
    var oldTrace = passerLayer.querySelector(".ink-residual-trace");
    if (oldTrace) oldTrace.remove();

    if (step.name) {
      var pBtn = existingPasser;
      if (!pBtn || !isSameContext || pBtn.getAttribute("data-name") !== step.name) {
        passerLayer.innerHTML = "";
        pBtn = document.createElement("button");
        pBtn.type = "button";
        pBtn.setAttribute("data-role", "f-passer");
        pBtn.setAttribute("data-name", step.name);
        pBtn.className = "ink-entity is-npc is-passer phase-" + step.phase;
        pBtn.style.left = step.x + "%";
        pBtn.style.top = step.y + "%";

        var nSpan = document.createElement("span");
        nSpan.className = "ent-name";
        nSpan.textContent = step.name;

        var dirSpan = document.createElement("span");
        dirSpan.className = "ent-passer-cadence";
        dirSpan.textContent = fState.passerPaused ? "駐足停步" : step.note;

        pBtn.appendChild(nSpan);
        pBtn.appendChild(dirSpan);

        pBtn.addEventListener("click", function (ev) {
          ev.stopPropagation();
          var curStep = room.passerSequence[fState.passerIdx % room.passerSequence.length];
          fState.passerPaused = true;
          var dSpan = pBtn.querySelector(".ent-passer-cadence");
          if (dSpan) dSpan.textContent = "駐足停步 · 再點往西同行";
          pushFBreath((curStep.desc || step.desc) + "（對方因你上前招呼而駐足停步。）", true);
          pBtn.onclick = function (e2) {
            e2.stopPropagation();
            if (fState.currentRoom === "bridge") {
              moveFRoom("west", "west_st");
            }
          };
        });
        passerLayer.appendChild(pBtn);
      } else {
        /* 原位平滑滑行過渡（觸發 CSS transition: left 0.9s cubic-bezier） */
        pBtn.className = "ink-entity is-npc is-passer phase-" + step.phase;
        pBtn.style.left = step.x + "%";
        pBtn.style.top = step.y + "%";
        var cadenceSpan = pBtn.querySelector(".ent-passer-cadence");
        if (cadenceSpan) {
          cadenceSpan.textContent = fState.passerPaused ? "駐足停步" : step.note;
        }
      }
      // 擦肩低語：一觸即聽見完整密語，並原位化為「隨藥農趕往西街」
      if (step.whisper) {
        var wBtn = document.createElement("button");
        wBtn.type = "button";
        wBtn.className = "ink-whisper-thread";
        wBtn.style.left = "48%";
        wBtn.style.top = "49.5%";
        wBtn.textContent = step.whisper;
        wBtn.addEventListener("click", function (ev) {
          ev.stopPropagation();
          pushFBreath("青衫旅人低聲指點：「三水西街百草藥鋪沈主事今日正高價急收三年野山參！」藥農聞言喜道：「那我這便趕往西街去！」", true);
          if (fState.currentRoom === "bridge") {
            wBtn.textContent = "· 隨藥農趕往西街";
            wBtn.onclick = function (e2) {
              e2.stopPropagation();
              moveFRoom("west", "west_st");
            };
          }
        });
        passerLayer.appendChild(wBtn);
      }
    } else {
      passerLayer.innerHTML = "";
    }
  }

  function moveFRoom(dir, targetRoom) {
    closeFPopover();
    var fromRoom = fRooms[fState.currentRoom];
    fState.prevRoomLabel = fromRoom.placeShort;
    var oppositeMap = { west: "east", east: "west", north: "south", south: "north" };
    fState.originDir = oppositeMap[dir] || null;
    fState.currentRoom = targetRoom;
    fState.passerIdx = 0;
    fState.passerPaused = false;
    fState.focusedEntityId = null;

    /* 正向光流同構：往西（左）走，西街景物自左側（shift-from-west）迎面推入 */
    var shiftClass = "shift-from-north";
    if (dir === "west") shiftClass = "shift-from-west";
    else if (dir === "east") shiftClass = "shift-from-east";
    else if (dir === "north") shiftClass = "shift-from-north";
    else if (dir === "south") shiftClass = "shift-from-south";

    var newRoom = fRooms[targetRoom];
    if (targetRoom === "west_st") {
      pushFBreath("你順著微苦的山參草氣踏入三水西街——前刻過橋的挑擔藥農果真正在百草藥鋪廊下卸擔過秤。", true);
    } else {
      pushFBreath("自「" + fromRoom.placeShort + "」移步至「" + newRoom.placeShort + "」，街面風景隨步履迎面展延。", true);
    }

    var atmoEl = document.getElementById("f-atmo");
    if (atmoEl) {
      atmoEl.style.opacity = "0.2";
      setTimeout(function () {
        atmoEl.textContent = (targetRoom === "bridge") ? fTides[fState.tideIdx].atmo : (newRoom.atmo || newRoom.title);
        atmoEl.style.opacity = "1";
      }, 180);
    }

    renderFRoom(shiftClass);
  }
  function openFPopover(title, subtitle, actions, desc) {
    fState.activePopover = title;
    var pop = document.getElementById("f-popover");
    var canvas = document.getElementById("f-canvas");
    if (!pop) return;

    document.getElementById("f-pop-title").textContent = title;
    document.getElementById("f-pop-sub").textContent = subtitle || "";
    document.getElementById("f-pop-log").textContent = desc;

    var actWrap = document.getElementById("f-pop-actions");
    actWrap.innerHTML = "";
    actions.forEach(function (act, idx) {
      var pill = document.createElement("button");
      pill.type = "button";
      pill.className = "pop-pill" + (idx === 0 ? " active" : "");
      pill.textContent = act;
      pill.addEventListener("click", function () {
        Array.from(actWrap.children).forEach(function (c) { c.classList.remove("active"); });
        pill.classList.add("active");
        if (act === "往西追上" || act === "結伴往西街") {
          closeFPopover();
          moveFRoom("west", "west_st");
        } else {
          pushFBreath("你對「" + title + "」施行「" + act + "」，細節已納入當前世界觀照。");
        }
      });
      actWrap.appendChild(pill);
    });

    pop.classList.add("is-open");
    if (canvas) canvas.classList.add("has-popover");
  }

  function closeFPopover() {
    fState.activePopover = null;
    var pop = document.getElementById("f-popover");
    var canvas = document.getElementById("f-canvas");
    if (pop) pop.classList.remove("is-open");
    if (canvas) canvas.classList.remove("has-popover");
  }

  /* ══════════════════════════════════════════════════════════════════
     初始化與全域世界脈動計時器（每 3.8 秒推進一息）
     ══════════════════════════════════════════════════════════════════ */
  function init() {
    /* ── 融匯定案事件綁定 ── */
    var fTideBtn = document.getElementById("f-tide-trigger");
    if (fTideBtn) {
      fTideBtn.addEventListener("click", function () {
        setFTide(fState.tideIdx + 1, true);
      });
    }

    ["north", "west", "east", "south"].forEach(function (dir) {
      var btn = document.getElementById("f-exit-" + dir);
      if (btn) {
        btn.addEventListener("click", function () {
          var target = btn.getAttribute("data-target") || "bridge";
          moveFRoom(dir, target);
        });
      }
    });

    var fCanvas = document.getElementById("f-canvas");
    if (fCanvas) {
      fCanvas.addEventListener("click", function () {
        if (fState.activePopover) closeFPopover();
        fState.focusedEntityId = null;
        var stage = document.getElementById("f-entities-layer");
        if (stage) {
          Array.from(stage.querySelectorAll(".ink-entity")).forEach(function (el) {
            var s = el.querySelector(".ent-micro-state");
            var a = el.querySelector(".ent-inline-actions");
            if (s) s.hidden = false;
            if (a) a.hidden = true;
          });
        }
      });
    }
    var fClose = document.getElementById("f-pop-close");
    if (fClose) fClose.addEventListener("click", closeFPopover);

    /* ── 典範一事件綁定 ── */
    ["north", "west", "east", "south"].forEach(function (dir) {
      var btn = document.getElementById("p1-exit-" + dir);
      if (btn) {
        btn.addEventListener("click", function () {
          var target = btn.getAttribute("data-target") || "bridge";
          moveP1Room(dir, target);
        });
      }
    });

    var p1Close = document.getElementById("p1-pop-close");
    if (p1Close) p1Close.addEventListener("click", closeP1Popover);

    var p2Close = document.getElementById("p2-pop-close");
    if (p2Close) {
      p2Close.addEventListener("click", function () {
        var pop = document.getElementById("p2-popover");
        var page = document.getElementById("p2-page");
        if (pop) pop.classList.remove("is-open");
        if (page) page.classList.remove("has-popover");
      });
    }

    var p3Close = document.getElementById("p3-pop-close");
    if (p3Close) {
      p3Close.addEventListener("click", function () {
        var pop = document.getElementById("p3-popover");
        if (pop) pop.classList.remove("is-open");
      });
    }

    /* 綁定典範二常駐行間字詞 */
    var p2StaticRoot = document.getElementById("p2-static-lines");
    if (p2StaticRoot) bindP2InlineWords(p2StaticRoot);

    /* 綁定典範三時刻撥盤 */
    var tideBtns = document.querySelectorAll(".tide-pill[data-idx]");
    tideBtns.forEach(function (btn) {
      btn.addEventListener("click", function () {
        var idx = parseInt(btn.getAttribute("data-idx"), 10) || 0;
        p3State.autoCycle = false;
        renderP3Phase(idx);
      });
    });

    /* 初次渲染各原型 */
    renderFRoom(null);
    renderFBreaths();
    renderP1Room(null);
    renderP2Breaths();
    renderP3Phase(1);
    /* 世界呼吸脈動 Loop */
    setInterval(function () {
      if (paused) return;

      /* 推進融匯定案：呼吸微態、過客步進、當下二息同步（受 6.5 秒凝視保護窗保護） */
      var inFocusWindow = Date.now() < fState.focusUntil;
      if (!fState.activePopover) {
        fState.tick += 1;
        var nextBreathText = null;
        if (!fState.passerPaused && !inFocusWindow) {
          fState.passerIdx += 1;
          var room = fRooms[fState.currentRoom];
          var step = room.passerSequence[fState.passerIdx % room.passerSequence.length];
          if (step && step.breath) {
            nextBreathText = step.breath;
          }
        }
        renderFRoom(null);
        /* 先見其形、後讀其息：280ms 視覺相位差，避免舞台位移與下方墨跡同時閃動 */
        if (nextBreathText && !inFocusWindow) {
          setTimeout(function () {
            if (Date.now() >= fState.focusUntil) {
              pushFBreath(nextBreathText, false);
            }
          }, 280);
        }
      }
      /* 推進典範一：穿行過客步履與在場微態 */
      if (!p1State.activePopover) {
        p1State.tick += 1;
        p1State.passerIdx += 1;
        renderP1Room(null);
      }

      /* 推進典範二：原位墨變與當下三息 */
      p2State.morphIdx += 1;
      p2State.breathCursor += 1;
      updateP2Morph();
      renderP2Breaths();

      /* 推進典範三：若未手動鎖定時刻，緩慢流轉六時刻潮汐 */
      if (p3State.autoCycle && p1State.tick % 2 === 0) {
        renderP3Phase(p3State.phaseIdx + 1);
      }
    }, 3800);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
