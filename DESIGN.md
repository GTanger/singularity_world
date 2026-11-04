---
version: "1.0"
name: "Singularity World Living MUD Surface"
description: "奇點世界（白塔半島）雙態視覺設計系統：宣紙水墨遊戲主介面（game.css）與暗色終端外殼（style.css）。"
colors:
  paper: "#f0e6d3"
  paperLight: "#f7f0e3"
  paperDark: "#e0d4c0"
  ink: "#2c1e0f"
  inkLight: "#6b5a47"
  inkFaint: "#a89880"
  redInk: "#8b2020"
  blueInk: "#2a4a6b"
  purpleInk: "#4a2a5a"
  greenInk: "#2a5a3a"
  gold: "#8b6914"
  shellBg: "#1e2328"
  shellPanel: "#252b33"
  shellText: "#e6edf3"
  shellTextMuted: "#8b949e"
  shellRoomName: "#e6a23c"
  shellBorder: "#3d444d"
  barHp: "#f56c6c"
  barSpirit: "#b37feb"
  barInner: "#79bbff"
typography:
  body-md:
    fontFamily: "'LXGW WenKai Mono TC', 'Chiron GoRound TC', 'Noto Serif TC', 'Songti SC', serif"
    fontSize: "15px"
    fontWeight: "400"
    lineHeight: "1.6"
  body-sm:
    fontFamily: "'LXGW WenKai Mono TC', 'Chiron GoRound TC', 'Noto Serif TC', serif"
    fontSize: "13px"
    fontWeight: "400"
    lineHeight: "1.4"
  room-title:
    fontFamily: "'LXGW WenKai Mono TC', 'Noto Serif TC', serif"
    fontSize: "18px"
    fontWeight: "700"
    lineHeight: "1.3"
  shell-mono:
    fontFamily: "'Noto Sans TC', 'Microsoft JhengHei', sans-serif"
    fontSize: "14px"
    fontWeight: "400"
    lineHeight: "1.5"
rounded:
  sm: "4px"
  md: "8px"
  lg: "10px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
components:
  paper-canvas:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "16px"
  paper-panel:
    backgroundColor: "{colors.paperDark}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "12px"
  auth-card:
    backgroundColor: "{colors.paperLight}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "24px"
  combat-alert:
    backgroundColor: "{colors.paperLight}"
    textColor: "{colors.redInk}"
    rounded: "{rounded.sm}"
    padding: "8px"
  shell-canvas:
    backgroundColor: "{colors.shellBg}"
    textColor: "{colors.shellText}"
    rounded: "{rounded.sm}"
    padding: "16px"
  shell-room-header:
    backgroundColor: "{colors.shellPanel}"
    textColor: "{colors.shellRoomName}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
---

## Overview

奇點世界（白塔半島）採用雙態共構的文字 MUD 互動介面：
1. **宣紙水墨態 (`web/game.css`)**：遊戲沉浸主介面，以溫潤宣紙為畫布，墨色深淺構建文字骨肉，揉合東方人文質地與傳統武俠氣韻。
2. **終端外殼態 (`web/style.css`)**：系統管理、登入認證與開發者除錯外殼，以深灰玄鐵色系突顯資訊架構與各項數值狀態條。

## Colors

色彩嚴格對應 `web/game.css` 與 `web/style.css` 原始定義，消除雙向漂移：
- **宣紙水墨色盤**：基底宣紙 `#f0e6d3`、明紙 `#f7f0e3`、暮紙 `#e0d4c0`；濃墨 `#2c1e0f`（在宣紙達 12.1:1 對比）、淡墨 `#6b5a47`、枯墨 `#a89880`。
- **水墨硃砂與五行靈韻**：硃砂血氣 `#8b2020`（明紙對比 6.5:1）、青墨 `#2a4a6b`、紫黛 `#4a2a5a`、竹青 `#2a5a3a`、秋金 `#8b6914`。
- **終端外殼色盤**：玄黑基底 `#1e2328`、面板深灰 `#252b33`、文字白銀 `#e6edf3`（對比 11.4:1）、次要灰 `#8b949e`、地名金橙 `#e6a23c`（對比 5.8:1）、邊框暗線 `#3d444d`。
- **狀態計量色**：氣血紅 `#f56c6c`、神念紫 `#b37feb`、真氣藍 `#79bbff`。

## Typography

依循世界觀配置專屬字型體驗：
- **水墨江湖正文**：首選 `LXGW WenKai Mono TC`（霞鶩文楷）與 `Noto Serif TC`，呈現楷體與宋體之書法筆意。
- **外殼資訊與數值**：採用 `Noto Sans TC` 與黑體無襯線字型，確保血條、即時座標與系統日誌一目了然。
- **排版間距**：正文行高維持 1.6 倍，保留閱讀留白，避免武俠敘事密集擁擠。

## Layout

佈局以垂直沉浸流為核心：
- **水墨介面**：頂欄固定 36px 顯示氣血、真氣、精神之墨色進度；中央為無邊界卷軸式動態敘事，底部為情境感知指令與羅盤導航。
- **外殼介面**：經典現代終端佈局，頂欄標明區域，中央雙欄呈現動態場景與角色裝備狀態，下方收納歷史行。

## Elevation & Depth

放棄現代厚重投影，以水墨暈染與紙張色階表現層次：
- **水墨層次**：透過宣紙層次（`#f0e6d3` $\rightarrow$ `#e0d4c0`）搭配 1px 枯墨細線 (`#a89880`) 區隔面板；陰影僅採用 `rgba(44, 30, 15, 0.08)` 極淡水暈。
- **終端外殼**：以深色色階（`#1e2328` 至 `#252b33`）與 `#3d444d` 邊框勾勒硬朗層次。

## Shapes

幾何輪廓採內斂微圓角：
- `sm (4px)`：水墨狀態標籤、計量條內膽、即時路徑指示按鈕。
- `md (8px)`：行囊格子、武功秘笈卡片、系統操作確認按鈕。
- `lg (10px)`：登入宣紙卡片、主動對話彈窗。
- `pill (999px)`：精力球指示標記與呼吸狀態燈。

## Components

所有元件組合均通過 WCAG 2.1 AA（$\ge 4.5:1$）色彩對比驗證：
- `paper-canvas`：宣紙遊戲主畫布，背景為 `{colors.paper}`，內文為濃墨 `{colors.ink}`。
- `paper-panel`：背包與狀態暮紙容器，背景 `{colors.paperDark}`，文字濃墨 `{colors.ink}`。
- `auth-card`：登入創角淺色宣紙卡，背景 `{colors.paperLight}`，文字濃墨 `{colors.ink}`。
- `combat-alert`：戰鬥警告與傷害提示，背景 `{colors.paperLight}`，文字硃砂 `{colors.redInk}`。
- `shell-canvas`：暗色終端畫布，背景 `{colors.shellBg}`，文字白銀 `{colors.shellText}`。
- `shell-room-header`：終端房間標題列，背景 `{colors.shellPanel}`，文字金橙 `{colors.shellRoomName}`。

## Do's and Don'ts

### Do's
- **保持水墨留白美感**：敘事卡片與文字留白不可過於緊繃，體現白塔半島特有氛圍。
- **恪守色彩真實來源**：嚴格使用 `game.css` 與 `style.css` 已定義的色碼。
- **確保全天候可讀性**：所有場景與戰鬥提示文字必須符合 WCAG AA 對比要求。

### Don'ts
- **切勿混用科技霓虹色彩**：水墨態嚴禁使用未定義的高飽和發光色或刺眼漸層。
- **切勿破壞跨端雙態語意**：遊戲主流程遵循水墨態，管理與除錯遵循外殼態，兩者風格切換邊界需清晰。
- **切勿使用未驗證的隨意邊框**：避免破壞宣紙天然古樸的邊界感。
