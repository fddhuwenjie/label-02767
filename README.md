# 虚拟资源售卖平台

基于 Rust Rocket 框架构建的虚拟资源售卖平台，包含管理后台和前台展示系统。

## 快速开始

### 开发环境

```bash
# 复制环境变量配置
cp .env.example .env

# 编辑 .env 文件，设置安全的密码和密钥
vim .env

# 启动服务
docker compose up --build -d

# 查看日志
docker compose logs -f backend

# 停止服务
docker compose down

# 完全重置（清除数据库）
docker compose down -v
```

### 生产环境部署

```bash
# 1. 复制并配置环境变量
cp .env.example .env

# 2. 生成安全密钥
openssl rand -base64 32  # 用于 SECRET_KEY 和 ROCKET_SECRET_KEY

# 3. 编辑 .env 设置生产配置
# - 设置强密码
# - 设置 ROCKET_ENV=production
# - 配置真实的数据库密码

# 4. 启动服务
docker compose up -d
```

## 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| Backend | 8090 | Rocket Web 应用 |
| MariaDB | 3310 | 数据库服务 |

- 前台首页: http://localhost:8090/
- 管理后台: http://localhost:8090/xuadmin/login

## 测试账号

### 管理员账号
- 用户名: `admin`
- 密码: `admin123`

> ⚠️ **生产环境请务必修改默认密码！**

### 测试用户账号
| 用户名 | 密码 | 会员类型 |
|--------|------|----------|
| zhangsan | 123456 | 月度会员 |
| lisi | 123456 | 季度会员 |
| wangwu | 123456 | 年度会员 |
| testuser | 123456 | 普通用户 |

---

## 安全配置

### 环境变量配置

所有敏感配置通过环境变量管理，参考 `.env.example`：

```env
# 数据库配置
DB_ROOT_PASSWORD=your_secure_root_password
DB_PASSWORD=your_secure_db_password

# 应用密钥 - 生产环境必须更改
SECRET_KEY=your_secure_secret_key
ROCKET_SECRET_KEY=your_secure_rocket_secret_key

# 运行环境
ROCKET_ENV=production  # 生产环境启用 Secure Cookie
```

### 安全特性

1. **后台隐蔽设计**
   - 管理后台路径 `/xuadmin` 不在前台暴露
   - 未授权访问统一返回 404，不暴露后台存在
   - `robots.txt` 禁止搜索引擎抓取后台路径

2. **登录防护**
   - IP维度：15分钟内失败5次锁定
   - 账户维度：15分钟内失败3次锁定
   - 登录尝试日志记录

3. **密码安全**
   - bcrypt 加密存储
   - 密码强度校验：至少8位，需包含大小写字母/数字/特殊字符中的两种
   - 弱密码检测（常见密码黑名单）

4. **Cookie 安全**
   - HttpOnly：防止 XSS 窃取
   - SameSite=Strict：防止 CSRF
   - Secure：生产环境启用（需 HTTPS）

5. **注册限制**
   - 单 IP 最多注册 5 个账号
   - 邮箱 MX 记录验证

### 生产环境检查清单

- [ ] 修改默认管理员密码
- [ ] 设置强数据库密码
- [ ] 生成新的 SECRET_KEY 和 ROCKET_SECRET_KEY
- [ ] 设置 ROCKET_ENV=production
- [ ] 配置 HTTPS（Nginx/Caddy 反向代理）
- [ ] 配置防火墙，仅开放必要端口
- [ ] 定期备份数据库

---

## 支付系统

### 支付流程

系统集成了支付宝（PC网站支付）和微信支付（Native扫码支付）两种支付方式，完整支付流程如下：

1. 用户选择会员套餐，前端调用 `POST /api/orders` 创建订单
2. 用户选择支付方式，前端调用 `POST /api/orders/pay` 发起支付
3. 后端根据支付方式生成支付宝跳转URL或微信支付二维码
4. 用户在支付宝/微信完成支付
5. 支付平台通过异步回调通知后端 (`POST /api/payment/callback/alipay` 或 `/wechat`)
6. 后端验证回调签名，确认支付成功后更新订单状态和用户会员
7. 前端通过轮询 `GET /api/orders/<id>/status` 检测支付完成，展示成功页面

### 支付安全

- 回调接口验证支付平台签名，防止伪造通知
- 订单金额在服务端从定价表计算，不信任前端传值
- 使用 `status = 'pending'` 条件更新防止重复支付
- 支付日志完整记录（tracing）
- 生产环境通过 HTTPS 保护通信

### 会员升级/续费逻辑

1. **新用户购买**：直接开通对应会员，设置到期时间
2. **续费同类型会员**：在当前到期时间基础上累加时长
3. **升级会员**：直接切换到新会员类型，时长从当前时间开始计算
4. **永久会员**：一次购买，永久有效，无需续费

### 配置支付参数

在 `.env` 文件中配置支付网关凭据：

```env
# 支付宝配置
ALIPAY_APP_ID=your_app_id
ALIPAY_PRIVATE_KEY=your_rsa2_private_key
ALIPAY_PUBLIC_KEY=alipay_rsa2_public_key
ALIPAY_NOTIFY_URL=https://yourdomain.com/api/payment/callback/alipay
ALIPAY_RETURN_URL=https://yourdomain.com/api/payment/return

# 微信支付配置
WECHAT_APP_ID=your_app_id
WECHAT_MCH_ID=your_mch_id
WECHAT_API_KEY=your_api_key
WECHAT_NOTIFY_URL=https://yourdomain.com/api/payment/callback/wechat

# 站点地址（用于自动生成回调URL）
PAYMENT_BASE_URL=https://yourdomain.com
```

> 当支付参数未配置时，系统会使用直接支付模式（支付后直接完成订单），适用于开发测试环境。配置支付参数后自动切换为对接真实支付网关

---

## 功能特性

### 后台管理 (`/xuadmin`)
- 管理员登录（无注册入口，安全防护）
- 仪表盘数据统计
- 文章管理（CRUD、Markdown编辑器、封面图片）
- 用户管理（添加、编辑、删除、会员设置）
- 订单管理
- 定价管理（月度/季度/年度/永久会员）
- 分类管理
- 标签管理（查看、重命名、删除）
- 网盘平台管理

### 前台功能
- 资源浏览和搜索（全文搜索、带封面图片展示）
- 用户注册（邮箱MX验证、IP限制5个账号、密码强度校验）
- 用户登录
- 会员购买/升级/续费
- 个人中心（订单记录）
- 会员可查看网盘链接，非会员不可见

### 技术栈
- 后端：Rust + Rocket 0.5.1
- 模板：Tera
- 数据库：MariaDB（FULLTEXT 全文索引）
- 编辑器：EasyMDE (Markdown)
- 样式：TailwindCSS
- 容器：Docker + Docker Compose

### 数据表设计
- `users` - 用户表（UUID主键）
- `articles` - 文章表（含封面图片、FULLTEXT索引）
- `categories` - 分类表
- `tags` - 标签表
- `article_tags` - 文章标签关联表
- `disk_platforms` - 网盘平台表
- `orders` - 订单表
- `pricing` - 定价表
- `sessions` - 会话表
- `ip_register_limits` - IP注册限制表
- `login_attempts` - 登录尝试记录表

---

## 项目结构

```
.
├── backend/
│   ├── src/
│   │   ├── main.rs          # 应用入口
│   │   ├── config.rs        # 配置管理
│   │   ├── db.rs            # 数据库初始化和迁移
│   │   ├── models.rs        # 数据模型
│   │   ├── middleware.rs    # 认证中间件
│   │   ├── utils.rs         # 工具函数
│   │   └── routes/
│   │       ├── mod.rs
│   │       ├── admin.rs     # 后台路由
│   │       ├── frontend.rs  # 前台路由
│   │       └── api.rs       # API路由（支付等）
│   ├── templates/
│   │   ├── admin/           # 后台模板
│   │   └── frontend/        # 前台模板
│   ├── static/
│   │   └── robots.txt       # 搜索引擎爬虫规则
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── Rocket.toml
├── docker-compose.yml
├── .env.example             # 环境变量示例
└── README.md
```

## License

MIT
