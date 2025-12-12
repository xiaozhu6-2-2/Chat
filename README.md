# Rust Chat - 即时通讯系统后端

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个基于 Rust 和 Axum 框架构建的高性能即时通讯系统后端，支持私聊、群聊、好友管理等核心功能。

## ✨ 特性

- 🚀 **高性能** - 基于 Tokio 异步运行时和 Axum Web 框架
- 🔐 **安全可靠** - JWT 认证、Argon2 密码加密、RSA 端到端加密
- 💬 **实时通讯** - WebSocket 支持实时消息推送
- 👥 **群组功能** - 创建群聊、管理成员、权限控制
- 🤝 **好友系统** - 添加好友、分组管理、黑名单
- 📱 **在线状态** - 实时在线用户状态展示
- 📝 **消息历史** - 完整的消息记录和已读状态
- 🗃️ **数据持久化** - MySQL 数据库 + Redis 缓存

## 🏗️ 技术栈

### 后端
- **Web框架**: [Axum](https://github.com/tokio-rs/axum) 0.8.4
- **异步运行时**: [Tokio](https://tokio.rs/) 1.46
- **数据库**: [MySQL](https://www.mysql.com/) (通过 [SQLx](https://github.com/launchbadge/sqlx))
- **缓存**: [Redis](https://redis.io/)
- **认证**: JWT + Argon2 + RSA
- **序列化**: Serde + Serde JSON
- **WebSocket**: tokio-tungstenite

### 前端（构建工具）
- **TypeScript** 5.8
- **Vue.js** 3.5
- **加密**: crypto-js, jsencrypt

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- MySQL 8.0+
- Redis 6.0+
- Node.js 16+ (用于前端构建)

### 安装步骤

1. **克隆仓库**
```bash
git clone https://github.com/xiaozhu6-2-2/Chat.git
cd Chat
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，配置数据库连接等信息
```

3. **配置数据库**
```bash
# 创建数据库
mysql -u root -p
CREATE DATABASE echat;

# 运行数据库迁移（TODO: 添加迁移脚本）
```

4. **启动 Redis**
```bash
redis-server
```

5. **运行项目**
```bash
cargo run
```

服务将在 `http://localhost:3000` 启动

### 环境变量配置

创建 `.env` 文件并配置以下变量：

```env
# 数据库连接
DATABASE_URL=mysql://username:password@localhost/echat

# JWT 密钥（请使用强随机字符串）
JWT_SECRET=your_super_secret_jwt_key_here

# 服务绑定地址
BIND_ADDRESS=0.0.0.0:3000

# Redis 连接（TODO: 添加到环境变量）
REDIS_URL=redis://localhost:6379
```

## 📚 API 文档

### 认证相关

- `POST /auth/register` - 用户注册
- `POST /auth/login` - 用户登录
- `POST /auth/session-key` - 获取会话密钥

### 用户管理

- `GET /auth/user/info` - 获取用户信息
- `POST /auth/user/update-user-info` - 更新用户信息
- `POST /auth/user/search` - 搜索用户

### 好友系统

- `POST /auth/friends/add` - 添加好友
- `POST /auth/friends/remove` - 删除好友
- `POST /auth/friends/list` - 获取好友列表
- `POST /auth/friends/request` - 发送好友请求

### 群聊管理

- `POST /auth/groups/create` - 创建群聊
- `POST /auth/groups/list` - 获取群聊列表
- `POST /auth/groups/join` - 申请加入群聊
- `POST /auth/groups/members` - 获取群成员列表

### 消息系统

- `POST /auth/messages/private/history` - 获取私聊历史
- `POST /auth/messages/group/history` - 获取群聊历史
- `WebSocket /ws` - 实时消息推送

详细的 API 文档请参考 [API.md](docs/API.md) (TODO: 创建)

## 🏛️ 项目架构

```
src/
├── handlers/        # HTTP 请求处理器
│   ├── auth.rs     # 认证相关
│   ├── user.rs     # 用户管理
│   ├── friends.rs  # 好友系统
│   ├── groups.rs   # 群聊管理
│   ├── message.rs  # 消息处理
│   └── ...
├── models/         # 数据模型
│   ├── entities.rs # 数据库实体
│   ├── requests.rs # 请求模型
│   ├── responses.rs# 响应模型
│   └── errors.rs   # 错误定义
├── repository/     # 数据访问层
│   ├── UserRepository.rs
│   ├── GroupMessageRepository.rs
│   └── ...
├── utils/          # 工具函数
│   ├── snowflake.rs # 雪花算法ID生成
│   └── ...
├── routes.rs       # 路由配置
├── middleware.rs   # 中间件
├── state.rs        # 应用状态
└── main.rs         # 程序入口
```

## 🔧 开发指南

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --test websocket_test
```

注意：当前测试代码被注释，需要恢复后才能运行。

### 代码格式化

```bash
cargo fmt
```

### 代码检查

```bash
cargo clippy
```

## 🚧 部署

### Docker 部署（TODO）

```bash
# 构建镜像
docker build -t rust-chat .

# 运行容器
docker run -d -p 3000:3000 --env-file .env rust-chat
```

### 生产环境配置

- 使用 HTTPS
- 配置反向代理（Nginx）
- 设置环境变量
- 配置日志级别
- 设置监控告警

## 🤝 贡献指南

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交你的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开一个 Pull Request

## 📝 更新日志

### v0.1.0 (当前版本)
- ✅ 用户认证系统
- ✅ 好友管理功能
- ✅ 群聊功能
- ✅ 实时消息推送
- ✅ 消息历史记录

### 计划功能
- [ ] 文件传输
- [ ] 语音消息
- [ ] 视频通话
- [ ] 消息撤回
- [ ] 群公告
- [ ] 表情包

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## ⚠️ 安全注意事项

1. **生产环境前请务必修改**：
   - JWT_SECRET 使用强随机字符串
   - 数据库密码使用强密码
   - 配置 CORS 允许的域名

2. **建议的安全措施**：
   - 启用 HTTPS
   - 添加速率限制
   - 定期更新依赖
   - 配置防火墙

## 📞 联系方式

如有问题或建议，请通过以下方式联系：

- 提交 [Issue](https://github.com/xiaozhu6-2-2/Chat/issues)
- 发送邮件至：[your-email@example.com]

## 🙏 致谢

感谢所有贡献者和以下开源项目：

- [Axum](https://github.com/tokio-rs/axum)
- [SQLx](https://github.com/launchbadge/sqlx)
- [Tokio](https://tokio.rs/)
- [Serde](https://serde.rs/)
- [Redis](https://redis.io/)

---

⭐ 如果这个项目对你有帮助，请给个 Star！