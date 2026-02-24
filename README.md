# 虚拟资源售卖平台

基于 Rust Rocket 框架构建的虚拟资源售卖平台，包含管理后台和前台展示系统。

## 快速开始

```bash
# 启动服务
docker-compose up --build -d

# 查看日志
docker-compose logs -f backend

# 停止服务
docker-compose down

# 完全重置（清除数据库）
docker-compose down -v
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

### 测试用户账号
| 用户名 | 密码 | 会员类型 |
|--------|------|----------|
| zhangsan | 123456 | 月度会员 |
| lisi | 123456 | 季度会员 |
| wangwu | 123456 | 年度会员 |
| testuser | 123456 | 普通用户 |

---

## 支付系统说明

### ⚠️ 当前状态：模拟支付

当前系统的支付功能为**模拟演示模式**，用于开发测试和功能展示：

- 点击"支付宝支付"或"微信支付"后，系统会模拟1.5秒的支付处理时间
- 支付完成后直接更新订单状态和用户会员信息
- **不会产生真实的资金交易**

### 会员升级/续费逻辑

1. **新用户购买**：直接开通对应会员，设置到期时间
2. **续费同类型会员**：在当前到期时间基础上累加时长
3. **升级会员**：直接切换到新会员类型，时长从当前时间开始计算
4. **永久会员**：一次购买，永久有效，无需续费

### 接入真实支付的步骤

要将模拟支付替换为真实支付网关，需要完成以下步骤：

#### 1. 选择支付服务商

| 支付方式 | 服务商 | 官方文档 |
|----------|--------|----------|
| 支付宝 | 蚂蚁金服开放平台 | https://open.alipay.com |
| 微信支付 | 微信支付商户平台 | https://pay.weixin.qq.com |
| 聚合支付 | Ping++、PayJS等 | 各平台官网 |

#### 2. 申请商户资质

- 企业营业执照
- 对公银行账户
- ICP备案域名
- 完成平台审核

#### 3. 修改后端代码

在 `backend/src/routes/api.rs` 中修改 `pay_order` 函数：

```rust
#[post("/orders/pay", data = "<form>")]
pub async fn pay_order(
    pool: &State<MySqlPool>,
    user: CurrentUser,
    form: Json<PayOrderRequest>,
) -> Json<ApiResponse<PaymentResponse>> {
    // 1. 验证订单
    let order = get_pending_order(&pool, &form.order_id, &user.0.id).await?;
    
    // 2. 调用支付网关创建支付订单
    let payment = match form.payment_method.as_str() {
        "alipay" => {
            // 调用支付宝SDK
            alipay::create_payment(&order).await?
        }
        "wechat" => {
            // 调用微信支付SDK
            wechat_pay::create_payment(&order).await?
        }
        _ => return Json(ApiResponse::error("不支持的支付方式")),
    };
    
    // 3. 返回支付链接/二维码给前端
    Json(ApiResponse::success(PaymentResponse {
        payment_url: payment.url,
        qr_code: payment.qr_code,
    }, "请完成支付"))
}

// 4. 添加支付回调接口
#[post("/orders/callback/<provider>", data = "<payload>")]
pub async fn payment_callback(
    pool: &State<MySqlPool>,
    provider: &str,
    payload: String,
) -> &'static str {
    // 验证签名
    // 更新订单状态
    // 更新用户会员
    "success"
}
```

#### 4. 添加支付SDK依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
# 支付宝SDK（示例）
alipay-sdk = "0.1"
# 或使用HTTP客户端自行对接
reqwest = { version = "0.11", features = ["json"] }
```

#### 5. 配置支付参数

创建 `backend/.env` 或环境变量：

```env
# 支付宝配置
ALIPAY_APP_ID=your_app_id
ALIPAY_PRIVATE_KEY=your_private_key
ALIPAY_PUBLIC_KEY=alipay_public_key
ALIPAY_NOTIFY_URL=https://yourdomain.com/api/orders/callback/alipay

# 微信支付配置
WECHAT_APP_ID=your_app_id
WECHAT_MCH_ID=your_mch_id
WECHAT_API_KEY=your_api_key
WECHAT_NOTIFY_URL=https://yourdomain.com/api/orders/callback/wechat
```

#### 6. 前端对接

修改 `backend/templates/frontend/pricing.html.tera` 中的 `payOrder` 函数：

```javascript
async function payOrder(method) {
    const res = await fetch('/api/orders/pay', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ order_id: orderId, payment_method: method })
    });
    const result = await res.json();
    
    if (result.success) {
        if (method === 'alipay') {
            // 跳转到支付宝支付页面
            window.location.href = result.data.payment_url;
        } else if (method === 'wechat') {
            // 显示微信支付二维码
            showQRCode(result.data.qr_code);
            // 轮询查询支付状态
            pollPaymentStatus(orderId);
        }
    }
}
```

#### 7. 安全注意事项

- ✅ 支付回调必须验证签名
- ✅ 使用HTTPS保护通信
- ✅ 订单金额在服务端计算，不信任前端传值
- ✅ 防止重复支付和重复回调
- ✅ 记录完整的支付日志

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
- 网盘平台管理

### 前台功能
- 资源浏览和搜索（带封面图片展示）
- 用户注册（邮箱MX验证、IP限制5个账号）
- 用户登录
- 会员购买/升级/续费
- 个人中心（订单记录）
- 会员可查看网盘链接，非会员不可见

### 技术栈
- 后端：Rust + Rocket 0.5.1
- 模板：Tera
- 数据库：MariaDB
- 编辑器：EasyMDE (Markdown)
- 样式：TailwindCSS
- 容器：Docker + Docker Compose

### 数据表设计
- `users` - 用户表（UUID主键）
- `articles` - 文章表（含封面图片）
- `categories` - 分类表
- `tags` - 标签表
- `article_tags` - 文章标签关联表
- `disk_platforms` - 网盘平台表
- `orders` - 订单表
- `pricing` - 定价表
- `sessions` - 会话表
- `ip_register_limits` - IP注册限制表

### 安全特性
- 管理后台路径 `/xuadmin` 隐蔽设计
- 管理员登录页面无注册入口
- 密码使用 bcrypt 加密
- 会话 token 安全存储
- IP 注册限制防止批量注册
- 邮箱 MX 记录验证
- 登录防暴破保护：
  - IP维度：15分钟内失败5次锁定
  - 账户维度：15分钟内失败3次锁定
  - 登录尝试日志记录
- Cookie安全设置（HttpOnly、SameSite）

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
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── Rocket.toml
├── docker-compose.yml
└── README.md
```

## License

MIT
