set working-directory := '..'

# 初始化开发环境及检查，刚 clone 后必须执行通过此指令
[default]
@all: hooks
    typos --version || echo "请使用 brew install typos-cli 安装 typos 用于拼写检查"
    tombi --version || echo "请使用 brew install tombi 安装 tombi 用于 toml 文件格式化"
    sqlx --version && "{{ just_executable() }}" --justfile "{{ justfile() }}" db setup || echo "请使用 brew install sqlx-cli 安装 sqlx 用于数据库相关操作"

# 更新 Git 钩子
hooks:
    git config --local core.hooksPath .git-hooks
