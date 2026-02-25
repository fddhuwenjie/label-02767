# 多阶段构建 - 支持 ARM64 和 AMD64
FROM rust:latest as builder

WORKDIR /app

# 安装编译依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源码
COPY Cargo.toml ./
COPY src ./src

# 构建
RUN cargo build --release

# 运行时镜像 - 使用支持多架构的基础镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制编译产物
COPY --from=builder /app/target/release/rocket_project /app/
COPY templates /app/templates
COPY static /app/static
COPY Rocket.toml /app/

# 环境变量
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000
ENV RUST_LOG=info

EXPOSE 8000

CMD ["./rocket_project"]
