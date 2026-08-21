set working-directory := '..'
set default-list

# 构建前端
frontend:
    pnpm -F @ani-dock/frontend build
