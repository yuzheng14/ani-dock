<p align="center">
  <img src="./assets/logo.png" alt="AniDock 標誌" width="240">
</p>

<h1 align="center">AniDock</h1>

<p align="center">你的動畫，停泊在家中。</p>

<p align="center">
  <a href="https://github.com/yuzheng14/ani-dock/actions/workflows/ci.yml"><img src="https://github.com/yuzheng14/ani-dock/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/yuzheng14/ani-dock/releases/latest"><img src="https://img.shields.io/github/v/release/yuzheng14/ani-dock" alt="最新版本"></a>
  <a href="https://github.com/yuzheng14/ani-dock/pkgs/container/ani-dock"><img src="https://img.shields.io/badge/container-ghcr.io-blue" alt="容器映像檔"></a>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh-CN.md">简体中文</a> · 繁體中文
</p>

貼上 AniGamer 任一集的 SN，選擇想下載的集數，其餘工作交給 AniDock。

AniDock 會匯入整部動畫、管理下載佇列，並透過自行託管的 Web 介面將所有資料
保留在你自己的裝置上。

<p align="center">
  <img src="./assets/demo.webp" alt="AniDock 實際使用示範：匯入動畫、選擇集數並查看下載進度" width="960">
</p>

## 運作方式

1. 🔎 **貼上集數 SN。** AniDock 會匯入整部動畫。
2. ✅ **選擇集數。** 只勾選你想下載的內容。
3. 📥 **查看進度。** 即時查看每一集的準備、下載與合併過程。

## 功能

- 🖥️ **在瀏覽器中管理一切。** 透過個人電腦、NAS 或 VPS 設定 AniDock、
  整理動畫庫並監控下載。
- 💾 **將資料留在本機。** 在本機儲存動畫庫、未完成佇列、Cookie、設定
  和下載的檔案。
- 🔄 **從中斷處繼續。** 重新啟動後自動恢復未完成的下載。
- ⚙️ **依你的方式下載。** 調整畫質、畫質鎖定、僅限 VIP 下載、
  分段下載並行數、廣告等待時間、User-Agent 和代理伺服器設定。
- ⏱️ **讓佇列自行運作。** 依序排程下載，並在工作之間加入類似真人操作
  的冷卻時間。
- 🐳 **幾乎可以部署在任何地方。** 支援 `linux/amd64` 和
  `linux/arm64`，容器映像檔中已包含 FFmpeg。

## 開始之前

AniDock 目前仍是早期的 `0.x` 專案，設定和行為可能會在不同版本之間變更。

AniDock 僅供個人使用。你有責任遵守 AniGamer 服務條款和適用法律。
AniDock 與 AniGamer 沒有關聯，也未獲得其認可。

> [!WARNING]
> AniDock 尚未內建身分驗證。請保留預設的本機迴環位址繫結，不要將其直接
> 暴露到公開網路。AniDock 儲存的 Cookie 屬於帳號憑證，請勿分享，也不要
> 將其包含在日誌或 Issue 中。需要從 NAS 或 VPS 遠端存取時，請使用可信的
> 私有網路、VPN 或具備身分驗證的反向代理伺服器。

## 快速開始

### 需求

- Docker Engine 或 Docker Desktop。
- 使用 Docker Compose 部署方式時，需要 Docker Compose v2 和 curl。
- 執行 AniDock 的裝置能夠存取 AniGamer。

### 使用 Docker 啟動

不需要 Docker Compose，也可以直接執行已發布的映像檔：

```bash
docker volume create ani-dock-data

docker run -d \
  --name ani-dock \
  --restart unless-stopped \
  --init \
  --read-only \
  --tmpfs /tmp \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --stop-timeout 30 \
  -p 127.0.0.1:6789:6789 \
  -v ani-dock-data:/home/anidock/.ani-dock \
  ghcr.io/yuzheng14/ani-dock:latest
```

### 使用 Docker Compose 啟動

只需要 Compose 檔案，不需要複製整個儲存庫：

```bash
mkdir -p ani-dock
cd ani-dock

curl -fsSL \
  https://raw.githubusercontent.com/yuzheng14/ani-dock/main/docker-compose.yaml \
  > docker-compose.yaml

docker compose pull ani-dock
docker compose up -d --no-build
```

容器進入健康狀態後，開啟 <http://127.0.0.1:6789>。

Compose 檔案預設使用 `ghcr.io/yuzheng14/ani-dock:latest`。如果需要
可重現的部署，請將 `ANI_DOCK_IMAGE` 設為帶版本標籤的完整映像檔名稱，例如
`ghcr.io/yuzheng14/ani-dock:0.1.0`。可用的映像檔標籤可以在
[容器套件](https://github.com/yuzheng14/ani-dock/pkgs/container/ani-dock)
頁面查看。

### 將資料儲存到主機目錄

上面的兩種部署方式預設都會將所有應用程式資料儲存在 `ani-dock-data`
具名磁碟區中。如果希望在主機上直接存取下載的檔案，請在第一次啟動前設定
磁碟區來源：

```bash
mkdir -p ./data
```

容器使用 UID 和 GID `10001` 執行。在 Linux 上，必要時請將目錄設定為
該使用者可寫入：

```bash
sudo chown -R 10001:10001 ./data
```

如果 NAS 無法使用 Shell 或 `chown`，請在 NAS 管理介面中為 UID `10001`
授予所選目錄的寫入權限。如果 NAS 不提供 Unix 所有權控制，也可以繼續使用
具名磁碟區。

使用 Docker Compose 時，設定主機目錄並啟動服務：

```bash
export ANI_DOCK_VOLUMES=./data
docker compose up -d --no-build
```

也可以將 `ANI_DOCK_IMAGE`、`ANI_DOCK_VOLUMES` 等 Compose 覆寫值寫入
部署目錄中的 `.env` 檔案，不需要在每次開啟 Shell 時重新匯出。

直接使用 Docker 部署時，將 `docker run` 指令中的
`-v ani-dock-data:/home/anidock/.ani-dock` 替換為
`-v "$(pwd)/data:/home/anidock/.ani-dock"`。

## 第一次設定

介面目前使用簡體中文，所以下列按鈕名稱保留介面中的原文。

1. 在 AniDock 中開啟 **设置**。
2. 如需使用已登入的帳號，請從已登入 AniGamer 的瀏覽器請求中複製
   `Cookie` 請求標頭，並填入同一瀏覽器的 User-Agent。請妥善保管這兩個值。
   未設定已登入帳號的 Cookie 時，AniDock 將以訪客身分存取，而且只能下載
   360p。
3. 設定畫質、代理伺服器和下載行為，然後點選 **提交**。如果 AniDock 提示
   需要重新啟動，請依照提示操作。
4. 開啟 **所有动画**，點選 **添加动画**，填入 AniGamer 集數 URL 中的數字
   `sn`，例如 `animeVideo.php?sn=3499`。
5. 點選 **下载**，選擇集數後點選 **确认**，然後在 **下载列表** 頁面查看
   進度。

## 資料與下載檔案

容器中的持久化磁碟區掛載於 `/home/anidock/.ani-dock`，其中包含：

| 路徑 | 內容 |
| --- | --- |
| `config.toml` | 應用程式設定。 |
| `cookie.txt` | AniGamer Cookie。請將此檔案視為敏感憑證。 |
| `data.db` | 動畫庫和下載佇列資料庫。 |
| `bangumi/` | 已完成的影片檔案，依動畫系列和季度分類。 |
| `tmp/` | 暫存下載檔案。 |

替換或刪除持久化磁碟區或主機目錄前，請先備份。

## 容器操作

### 控制日誌層級

AniDock 預設使用 `info` 日誌層級。可以透過 `RUST_LOG` 環境變數將層級設為
`error`、`warn`、`info`、`debug` 或 `trace`，日誌詳細程度依序增加。也可以
只為特定元件開啟詳細日誌；例如，`info,ani_dock_core=debug` 會維持預設層級
為 `info`，同時為核心下載器和 AniGamer 請求開啟 `debug` 日誌。

使用 Docker Compose 時，在部署目錄的 `.env` 檔案中加入所需層級：

```dotenv
RUST_LOG=debug
```

然後重新建立服務並持續查看日誌：

```bash
docker compose up -d --no-build
docker compose logs -f ani-dock
```

`docker compose restart` 不會重新讀取環境變數。直接使用 Docker 時，請在
`docker run` 指令中加入 `-e RUST_LOG=debug`，並重新建立容器。從原始碼執行
時，可以直接為啟動指令設定該變數：

```bash
RUST_LOG=debug cargo run -p ani-dock-server
```

`debug` 和 `trace` 日誌可能包含請求 URL 等診斷資訊。請只在疑難排解時啟用，
排解完成後恢復為 `info`，並在分享日誌前檢查及移除敏感資訊。

### Docker Compose

```bash
# 查看狀態與健康狀況
docker compose ps

# 持續查看日誌
docker compose logs -f ani-dock

# 重新啟動服務
docker compose restart ani-dock

# 停止應用程式，但保留持久化資料
docker compose down
```

### Docker

```bash
# 查看狀態與健康狀況
docker ps -f name=ani-dock

# 持續查看日誌
docker logs -f ani-dock

# 重新啟動容器
docker restart ani-dock

# 停止容器，但保留持久化資料
docker stop ani-dock
```

更新使用已發布映像檔的部署：

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yuzheng14/ani-dock/main/docker-compose.yaml \
  > docker-compose.yaml
docker compose pull ani-dock
docker compose up -d --no-build
```

如果自訂了 `ANI_DOCK_IMAGE` 或 `ANI_DOCK_VOLUMES`，執行這些指令時
請確認覆寫值仍已匯出，或已經儲存在 `.env` 中。

直接使用 Docker 部署時，請拉取新映像檔、刪除舊容器，然後重新執行上面的
`docker run` 指令。刪除容器不會刪除具名磁碟區：

```bash
docker pull ghcr.io/yuzheng14/ani-dock:latest
docker stop ani-dock
docker rm ani-dock
```

## 從原始碼建置

Docker 建置包含前端、Rust 伺服器、FFmpeg 和所有執行階段相依套件：

```bash
git clone https://github.com/yuzheng14/ani-dock.git
cd ani-dock

ANI_DOCK_IMAGE=ani-dock:local docker compose up -d --build
```

## 回報問題

請在 [Issue 頁面](https://github.com/yuzheng14/ani-dock/issues/new) 提供
AniDock 版本或映像檔標籤、部署方式、重現步驟和相關日誌。公開日誌前，請移除
Cookie、權杖、帳號識別碼和其他敏感資訊。

## 致謝

- [aniGamerPlus](https://github.com/miyouzi/aniGamerPlus)
