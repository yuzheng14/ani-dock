set working-directory := '..'

# 检查所有前端代码
[default]
all: format types lint

# 运行 Prettier 检查前端代码格式
format:
    pnpm -F @ani-dock/frontend exec prettier --check .

# 检查前端代码类型
types:
    pnpm -F @ani-dock/frontend exec tsc -b --noEmit

# 使用 oxlint 检查前端代码
lint:
    pnpm -F @ani-dock/frontend exec oxlint --deny-warnings
