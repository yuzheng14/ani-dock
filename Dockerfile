# syntax=docker/dockerfile:1.20

# ============================================================
# Frontend builder
# ============================================================

FROM node:24-bookworm-slim AS frontend

RUN npm install -g pnpm@11.18.0

WORKDIR /src

# only copy metadata for better docker build cache
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY --parents packages/*/package.json ./

RUN --mount=type=cache,id=pnpm,target=/pnpm/store \
  pnpm config set store-dir /pnpm/store && \
  pnpm i --frozen-lockfile

COPY packages packages

COPY assets/logo.png packages/frontend/public/logo.png

RUN pnpm --filter @ani-dock/frontend build

# ============================================================
# Backend builder
# ============================================================

FROM rust:1.97-trixie AS backend

RUN apt update && apt install -y --no-install-recommends \
  cmake \
  libclang-dev

WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY .cargo/ .cargo/

COPY crates/ crates/
COPY tests/ tests/

ENV DATABASE_URL=sqlite://crates/ani-dock-db/schema.sqlite

RUN --mount=type=cache,id=ani-dock-cargo-registry,target=/usr/local/cargo/registry \
  --mount=type=cache,id=ani-dock-cargo-git,target=/usr/local/cargo/git \
  cargo build \
    --locked \
    --release \
    --package ani-dock-server

# ============================================================
# Runtime
# ============================================================
FROM debian:trixie-slim AS runtime

LABEL org.opencontainers.image.licenses="Apache-2.0"

RUN apt update && \
  apt install -y --no-install-recommends \
  ca-certificates \
  curl \
  ffmpeg \
  && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 10001 anidock && \
  useradd \
    --uid 10001 \
    --gid anidock \
    --create-home \
    --home-dir /home/anidock \
    --shell /usr/sbin/nologin \
    anidock && \
  install -d \
    -o anidock \
    -g anidock \
    /app \
    /home/anidock/.ani-dock

WORKDIR /app

COPY LICENSE /licenses/ani-dock/LICENSE

COPY --from=backend \
  --chown=10001:10001 \
  /src/target/release/ani-dock-server \
  /app/ani-dock-server

COPY --from=frontend \
  --chown=10001:10001 \
  /src/packages/frontend/dist \
  /app/dist

ENV ANI_DOCK_HOST=0.0.0.0 \
  ANI_DOCK_PORT=6789 \
  RUST_LOG=info

USER anidock

VOLUME ["/home/anidock/.ani-dock"]

EXPOSE 6789

HEALTHCHECK \
  --interval=30s \
  --timeout=3s \
  --retries=3 \
  CMD ["curl", "-fsS", "http://127.0.0.1:6789/api/health"]

CMD ["/app/ani-dock-server"]
