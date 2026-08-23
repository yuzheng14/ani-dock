<p align="center">
  <img src="./assets/logo.png" alt="AniDock 标志" width="240">
</p>

<h1 align="center">AniDock</h1>

<p align="center">你的动画，停泊在家中。</p>

<p align="center">
  <a href="https://github.com/yuzheng14/ani-dock/actions/workflows/ci.yml"><img src="https://github.com/yuzheng14/ani-dock/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/yuzheng14/ani-dock/releases/latest"><img src="https://img.shields.io/github/v/release/yuzheng14/ani-dock" alt="最新版本"></a>
  <a href="https://github.com/yuzheng14/ani-dock/pkgs/container/ani-dock"><img src="https://img.shields.io/badge/container-ghcr.io-blue" alt="容器镜像"></a>
</p>

<p align="center">
  <a href="./README.md">English</a> · 简体中文 · <a href="./README.zh-TW.md">繁體中文</a>
</p>

粘贴 AniGamer 任意一集的 SN，选择想下载的剧集，其余工作交给 AniDock。

AniDock 会导入整部动画、管理下载队列，并通过自托管 Web 界面将所有数据
保留在你自己的设备上。

<p align="center">
  <img src="./assets/demo.webp" alt="AniDock 实际使用演示：导入动画、选择剧集并查看下载进度" width="960">
</p>

## 工作方式

1. 🔎 **粘贴剧集 SN。** AniDock 会导入整部动画。
2. ✅ **选择剧集。** 只勾选你想下载的内容。
3. 📥 **查看进度。** 实时查看每一集的准备、下载与合并过程。

## 功能

- 🖥️ **在浏览器中管理一切。** 通过个人电脑、NAS 或 VPS 配置 AniDock、
  整理动画库并监控下载。
- 💾 **将数据留在本地。** 在本地保存动画库、未完成队列、Cookie、配置
  和下载的文件。
- 🔄 **从中断处继续。** 重启后自动恢复未完成的下载。
- ⚙️ **按你的方式下载。** 调整清晰度、清晰度锁定、仅限 VIP 下载、
  分段下载并发数、广告等待时间、User-Agent 和代理设置。
- ⏱️ **让队列自行运行。** 按顺序调度下载，并在任务之间加入类似真人操作
  的冷却时间。
- 🐳 **几乎可以部署在任何地方。** 支持 `linux/amd64` 和
  `linux/arm64`，容器镜像中已包含 FFmpeg。

## 开始之前

AniDock 目前仍是早期的 `0.x` 项目，配置和行为可能会在不同版本之间变化。

AniDock 仅供个人使用。你有责任遵守 AniGamer 服务条款和适用法律。
AniDock 与 AniGamer 不存在关联，也未获得其认可。

> [!WARNING]
> AniDock 尚未内置身份验证。请保留默认的本地回环地址绑定，不要将其直接
> 暴露到公网。AniDock 保存的 Cookie 属于账号凭据，请勿分享，也不要将其
> 包含在日志或 Issue 中。需要从 NAS 或 VPS 远程访问时，请使用可信的
> 私有网络、VPN 或带身份验证的反向代理。

## 快速开始

### 要求

- Docker Engine 或 Docker Desktop。
- 使用 Docker Compose 部署方式时，需要 Docker Compose v2 和 curl。
- 运行 AniDock 的设备能够访问 AniGamer。

### 使用 Docker 启动

无需 Docker Compose，也可以直接运行已发布的镜像：

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

### 使用 Docker Compose 启动

只需要 Compose 文件，无需克隆整个仓库：

```bash
mkdir -p ani-dock
cd ani-dock

curl -fsSL \
  https://raw.githubusercontent.com/yuzheng14/ani-dock/main/docker-compose.yaml \
  > docker-compose.yaml

docker compose pull ani-dock
docker compose up -d --no-build
```

容器进入健康状态后，打开 <http://127.0.0.1:6789>。

Compose 文件默认使用 `ghcr.io/yuzheng14/ani-dock:latest`。如果需要
可复现的部署，请将 `ANI_DOCK_IMAGE` 设为带版本标签的完整镜像名，例如
`ghcr.io/yuzheng14/ani-dock:0.1.0`。可用的镜像标签可以在
[容器软件包](https://github.com/yuzheng14/ani-dock/pkgs/container/ani-dock)
页面查看。

### 将数据存放到宿主机目录

上面的两种部署方式默认都会将所有应用数据保存在 `ani-dock-data` 命名卷中。
如果希望在宿主机上直接访问下载的文件，请在首次启动前设置卷来源：

```bash
mkdir -p ./data
```

容器使用 UID 和 GID `10001` 运行。在 Linux 上，必要时请将目录设置为
该用户可写：

```bash
sudo chown -R 10001:10001 ./data
```

如果 NAS 无法使用 Shell 或 `chown`，请在 NAS 管理界面中为 UID `10001`
授予所选目录的写入权限。如果 NAS 不提供 Unix 所有权控制，也可以继续使用
命名卷。

使用 Docker Compose 时，设置宿主机目录并启动服务：

```bash
export ANI_DOCK_VOLUMES=./data
docker compose up -d --no-build
```

也可以将 `ANI_DOCK_IMAGE`、`ANI_DOCK_VOLUMES` 等 Compose 覆盖值写入
部署目录中的 `.env` 文件，无需在每次打开 Shell 时重新导出。

直接使用 Docker 部署时，将 `docker run` 命令中的
`-v ani-dock-data:/home/anidock/.ani-dock` 替换为
`-v "$(pwd)/data:/home/anidock/.ani-dock"`。

## 首次设置

1. 在 AniDock 中打开 **设置**。
2. 如需使用已登录的账号，请从已登录 AniGamer 的浏览器请求中复制
   `Cookie` 请求头，并填写同一浏览器的 User-Agent。请妥善保管这两个值。
   未配置已登录账号的 Cookie 时，AniDock 将以访客身份访问，并且只能下载
   360p。
3. 配置清晰度、代理和下载行为，然后点击 **提交**。如果 AniDock 提示需要
   重启，请按照提示操作。
4. 打开 **所有动画**，点击 **添加动画**，填写 AniGamer 剧集 URL 中的数字
   `sn`，例如 `animeVideo.php?sn=3499`。
5. 点击 **下载**，选择剧集后点击 **确认**，然后在 **下载列表** 页面查看
   进度。

## 数据与下载文件

容器中的持久化卷挂载在 `/home/anidock/.ani-dock`，其中包含：

| 路径 | 内容 |
| --- | --- |
| `config.toml` | 应用配置。 |
| `cookie.txt` | AniGamer Cookie。请将此文件视为敏感凭据。 |
| `data.db` | 动画库和下载队列数据库。 |
| `bangumi/` | 已完成的视频文件，按动画系列和季度分类。 |
| `tmp/` | 临时下载文件。 |

替换或删除持久化卷或宿主机目录前，请先进行备份。

## 容器操作

### 控制日志级别

AniDock 默认使用 `info` 日志级别。可以通过 `RUST_LOG` 环境变量将级别设为
`error`、`warn`、`info`、`debug` 或 `trace`，日志详细程度依次增加。也可以
只为特定组件开启详细日志；例如，`info,ani_dock_core=debug` 会保持默认级别
为 `info`，同时为核心下载器和 AniGamer 请求开启 `debug` 日志。

使用 Docker Compose 时，在部署目录的 `.env` 文件中添加所需级别：

```dotenv
RUST_LOG=debug
```

然后重新创建服务并持续查看日志：

```bash
docker compose up -d --no-build
docker compose logs -f ani-dock
```

`docker compose restart` 不会重新读取环境变量。直接使用 Docker 时，请在
`docker run` 命令中添加 `-e RUST_LOG=debug`，并重新创建容器。从源码运行时，
可以直接为启动命令设置该变量：

```bash
RUST_LOG=debug cargo run -p ani-dock-server
```

`debug` 和 `trace` 日志可能包含请求 URL 等诊断信息。请仅在排查问题时启用，
排查完成后恢复为 `info`，并在分享日志前检查和移除敏感信息。

### Docker Compose

```bash
# 查看状态与健康状况
docker compose ps

# 持续查看日志
docker compose logs -f ani-dock

# 重启服务
docker compose restart ani-dock

# 停止应用，但保留持久化数据
docker compose down
```

### Docker

```bash
# 查看状态与健康状况
docker ps -f name=ani-dock

# 持续查看日志
docker logs -f ani-dock

# 重启容器
docker restart ani-dock

# 停止容器，但保留持久化数据
docker stop ani-dock
```

更新使用已发布镜像的部署：

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yuzheng14/ani-dock/main/docker-compose.yaml \
  > docker-compose.yaml
docker compose pull ani-dock
docker compose up -d --no-build
```

如果自定义了 `ANI_DOCK_IMAGE` 或 `ANI_DOCK_VOLUMES`，运行这些命令时
请确保覆盖值仍已导出，或已经保存在 `.env` 中。

直接使用 Docker 部署时，请拉取新镜像、删除旧容器，然后重新执行上面的
`docker run` 命令。删除容器不会删除命名卷：

```bash
docker pull ghcr.io/yuzheng14/ani-dock:latest
docker stop ani-dock
docker rm ani-dock
```

## 从源码构建

Docker 构建包含前端、Rust 服务端、FFmpeg 和所有运行时依赖：

```bash
git clone https://github.com/yuzheng14/ani-dock.git
cd ani-dock

ANI_DOCK_IMAGE=ani-dock:local docker compose up -d --build
```

## 报告问题

请在 [Issue 页面](https://github.com/yuzheng14/ani-dock/issues/new) 提供
AniDock 版本或镜像标签、部署方式、复现步骤和相关日志。公开日志前，请移除
Cookie、令牌、账号标识符和其他敏感信息。

## 致谢

- [aniGamerPlus](https://github.com/miyouzi/aniGamerPlus)
