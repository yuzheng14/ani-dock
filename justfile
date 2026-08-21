# 未指定命令时列出顶层子命令
set default-list

# 初始化开发环境
mod setup '.just/setup.just'

# 数据库相关命令
mod db '.just/db.just'

# 启动开发服务
mod dev '.just/dev.just'

# 构建项目
mod build '.just/build.just'

# 检查代码质量
mod check '.just/check.just'

# 运行测试
mod test '.just/test.just'

# 生成代码
mod generate '.just/generate.just'

# 管理前端 UI 组件
mod ui '.just/ui.just'
