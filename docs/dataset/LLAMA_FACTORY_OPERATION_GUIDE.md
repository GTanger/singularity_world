# LLaMA-Factory 操作手冊 — 奇點地球人訓練流程

> **日期**：2026-04-20
> **目的**：從零到跑出 Ollama 可對話的奇點地球人 persona 的完整操作步驟
> **設計者動手範圍**：打開終端機照指令貼、Web UI 點一點、`ollama run` 對話
> **Claude 動手範圍**：dataset 格式轉換、驗收測試腳本、遇到問題判讀
> **關鍵校正**：模型正確名是 **Gemma 3n E2B**（`google/gemma-3n-E2B-it`），不是「Gemma 4」——之前記憶 `a36fdfe4` 講「Gemma 4 E2B」是名字錯，實際是 Google 的 Gemma 3n 系列（n = nano，為 edge/mobile 設計）
> **LLaMA-Factory template**：`gemma3n`

---

## 你電腦已有的（已盤點確認）

- ROCm 7.2.0 ✓
- AMD RX 9060 XT 16GB VRAM ✓
- Python 3.12 ✓
- Ollama 0.21.0 ✓
- Anthropic API key（存在 `~/.memory-mcp/.env`）✓
- OpenAI API key ✓

## 你電腦還要裝的

- **Docker**（如果沒裝）：`sudo apt install docker.io docker-compose-v2`
- **LLaMA-Factory**（透過 git clone 一次）

沒了。就這兩樣。

---

## 階段 0 · 起動 LLaMA-Factory（一次性，~30 分鐘）

```bash
cd ~/Projects
git clone --depth 1 https://github.com/hiyouga/LLaMA-Factory.git
cd LLaMA-Factory/docker/docker-rocm
docker compose up -d
```

第一次跑會下載 + build 映像檔（10-30 分鐘、5-10 GB 空間）。完成後：

```bash
docker compose exec llamafactory bash
```

進到容器裡面。然後：

```bash
llamafactory-cli webui
```

瀏覽器打開 **http://localhost:7860** → **這是你的操作介面**。

往後只要你重開機，只要：
```bash
cd ~/Projects/LLaMA-Factory/docker/docker-rocm
docker compose up -d
docker compose exec llamafactory bash
llamafactory-cli webui
```
就能再用。

---

## 階段 1 · 資料準備（Claude 動手）

LLaMA-Factory 看得懂兩種資料格式：

**CPT 格式**（Continuous Pre-Training，灌世界觀用）：純文字 JSON
```json
[
  {"text": "整段原文或 synthetic 改寫的變體..."},
  {"text": "另一段..."}
]
```

**SFT 格式**（Supervised Fine-Tuning，教口吻用）：問答對 JSON
```json
[
  {"instruction": "你是 AI 嗎？", "output": "你問我是不是 ai？我若是 ai 的話..."}
]
```

### Claude 會產出的檔案

1. **`singularity_cpt.json`**：五話+續篇+wiki+briefings 切 chunk + synthetic 改寫 × 3 角度 → 總量約 1.5-2.5M tokens
2. **`singularity_sft.json`**：v4 的 20 筆 gold samples 轉 alpaca 格式
3. **`dataset_info.json` 片段**：讓 LLaMA-Factory 認得這兩個資料集

放進容器的路徑：`LLaMA-Factory/data/`

### dataset_info.json 要補的內容

編輯 `LLaMA-Factory/data/dataset_info.json`，在結尾加：

```json
"singularity_cpt": {
  "file_name": "singularity_cpt.json",
  "columns": {
    "prompt": "text"
  }
},
"singularity_sft": {
  "file_name": "singularity_sft.json",
  "columns": {
    "prompt": "instruction",
    "response": "output"
  }
}
```

---

## 階段 2 · 灌世界觀（CPT，~3-6 小時）

這階段的目的：**讓模型把奇點世界吃進骨子裡**（對應前面討論的「讓它永遠記住」五個動作中的第 1-4 步）。

Web UI 操作：

| 分頁/欄位 | 填什麼 |
|---|---|
| 分頁 | **Train** |
| Language | `zh` |
| Model name | `Gemma3n-4B-Chat`（下拉選）或手動填 `google/gemma-3n-E2B-it` |
| Finetuning method | `lora` |
| **Stage** | `Pre-Training` ← 這是 CPT |
| Dataset dir | `data` |
| Dataset | 勾選 `singularity_cpt` |
| Cutoff length | `2048` |
| Max samples | （留空，跑全部） |
| **Learning rate** | `1e-4` |
| **Epochs** | `3` ← 讀 3 遍反覆強化 |
| Max grad norm | `1.0` |
| Batch size | `1` |
| Gradient accumulation | `8` |
| LR scheduler | `cosine` |
| Warmup steps | `100` |
| Compute type | `bf16` |
| **LoRA rank** | `64` ← 比 SFT 大，吃深知識 |
| **LoRA alpha** | `128` |
| **LoRA target** | `all` |
| **Modules to save** | `lm_head,embed_tokens` ← 關鍵！訓到「字意義」底層 |
| **RSLoRA** | ✓ 勾 |
| Output dir | `saves/gemma-singularity-cpt` |

按 **Preview command** 看一下 CLI 指令、按 **Start**。下面會畫 loss 曲線。

**預期**：loss 從 2.5-3.5 開始，3 個 epoch 後降到 1.5-2.0 附近。
**VRAM**：預估 12-14 GB（16GB 顯卡剛好吃得下）。
**失敗訊號**：OOM → 把 cutoff_length 從 2048 降到 1024；loss 不降 → learning_rate 從 1e-4 升到 2e-4。

完成後 `saves/gemma-singularity-cpt/` 會有 LoRA adapter 檔案。

---

## 階段 3 · 教老人口吻（SFT，~20-40 分鐘）

載入剛才 CPT 訓出來的 adapter 再訓 SFT。這對應「永遠記住」第 5 步（驗收前的 final 口吻校正）。

Web UI 操作（**換欄位**，跟階段 2 不一樣）：

| 欄位 | 填什麼 |
|---|---|
| Model name | `google/gemma-3n-E2B-it` |
| **Adapter path** | `saves/gemma-singularity-cpt` ← 接續階段 2 |
| Finetuning method | `lora` |
| **Stage** | `Supervised Fine-Tuning` ← SFT |
| Dataset | 勾 `singularity_sft` |
| **Template** | `gemma3n` |
| Cutoff length | `1024` |
| Learning rate | `5e-5` ← 比 CPT 低 |
| Epochs | `5` |
| Batch size | `1` |
| Gradient accumulation | `4` |
| LoRA rank | `16` ← 比 CPT 小，只校表面行為 |
| LoRA alpha | `32` |
| LoRA target | `all` |
| Output dir | `saves/gemma-singularity-sft` |

按 **Start**。20 筆資料很快，20-40 分鐘。loss 降到 0.5-1.0 附近算過關。

---

## 階段 4 · 匯出 + 放 Ollama（~20 分鐘）

Web UI 切到 **Export** 分頁：

| 欄位 | 填什麼 |
|---|---|
| Model name | `google/gemma-3n-E2B-it` |
| Adapter path | `saves/gemma-singularity-sft` |
| Finetuning method | `lora` |
| Export dir | `export/gemma-singularity` |
| Export size | `1 GB`（每個 shard 多大，`1 GB` 常用） |
| Export device | `auto` |
| Export quantization bit | `4`（匯出時順便壓縮成 4-bit，模型變 2-3 GB） |
| **Save Ollama modelfile** | ✓ 勾 |

按 **Export**。完成後 `export/gemma-singularity/` 有：
- 模型權重檔
- `Modelfile`（Ollama 格式定義）

### 裝進 Ollama

**離開 Docker 容器**（輸入 `exit`），回到你自己電腦：

```bash
cd ~/Projects/LLaMA-Factory/export/gemma-singularity
ollama create gemma-singularity -f Modelfile
```

完成後：

```bash
ollama run gemma-singularity
```

開始對話——這就是訓好的奇點地球人。

---

## 階段 5 · 變題驗收（Claude 會寫腳本）

對應「永遠記住」第 5 步。

Claude 會寫一個 Python 腳本：
- 50 題測試問題（原題 + 變體 + 陷阱 + 禁忌碰撞）
- 自動丟給 Ollama 跑
- 答案存成 `verification_results.txt`

你手審那份結果檔。

### 失敗訊號對應處理

| 訊號 | 回哪階段 | 怎麼改 |
|---|---|---|
| 世界觀記不住（答錯事實） | 階段 2（CPT） | 加 synthetic 改寫量（從 3 角度增到 5 角度）或 epoch 從 3 升到 5 |
| 口吻歪（文學腔/書面語） | 階段 3（SFT） | 補 gold samples 到 50 筆 |
| 碰禁忌詞（講「訊號場/班底」等） | 階段 3 | SFT 加幾筆禁忌碰撞範例 |
| 忘基本中文 | 階段 2 | 加入 20% 中文通用文本進 CPT corpus（防遺忘） |

---

## 常見問題

**Q: `docker compose up -d` 失敗**
A: 檢查 Docker 有沒有裝：`docker --version`。確認 /dev/kfd 有權限：`ls -la /dev/kfd`，你的使用者需在 `render` 群組裡（`sudo usermod -a -G render $USER`、重登入）。

**Q: Web UI 連不上 localhost:7860**
A: port 可能被佔：`sudo lsof -i :7860`。或容器沒跑：`docker compose ps`。

**Q: 下載模型慢**
A: 在容器裡 `export USE_MODELSCOPE_HUB=1`，改從 ModelScope（阿里雲）下載。

**Q: 訓練 OOM（記憶體爆）**
A: cutoff_length 2048 → 1024；gradient_accumulation 從 8 → 4；或打開 quantization 4-bit（QLoRA）。

**Q: 不確定 LoRA 參數**
A: **先用預設跑一次看結果**，不對再調。LLaMA-Factory 預設就是業界標準。

---

## 整體工時估計

| 階段 | 工時 |
|---|---|
| 階段 0 Docker 起 | 30 分鐘（只做一次） |
| 階段 1 資料準備（Claude） | 1-2 天（含 synthetic 擴寫跑 API 過夜） |
| 階段 2 CPT 訓練 | 3-6 小時 |
| 階段 3 SFT 訓練 | 20-40 分鐘 |
| 階段 4 匯出 Ollama | 20 分鐘 |
| 階段 5 驗收 | 1-2 小時（含人工審） |
| **第一次跑到出 MVP** | **約 2-3 天** |

失敗重跑：階段 2 最貴（3-6 小時 + 可能要補擴寫 $25）、階段 3-5 便宜。

---

## 下一步

Claude 要做：
1. 寫 **synthetic 擴寫腳本**（Python 呼叫 Anthropic API，從 `~/.memory-mcp/.env` 讀 key）
2. 寫 **dataset 轉換腳本**（把現有 .md 資產切 chunk、組 CPT JSON + SFT JSON）
3. 寫 **驗收測試腳本**（50 題問答自動跑 Ollama）

設計者要做：
1. `apt install docker.io docker-compose-v2`（如果沒有）
2. 跑階段 0 一次起 Docker
3. 跑 Claude 寫的 synthetic 擴寫腳本（一晚上）
4. 用 Web UI 操作階段 2、3、4
5. 手審階段 5 結果

這份手冊之後遇到問題、調參、或 debug 都是在對著這檔對照哪一步出事。
