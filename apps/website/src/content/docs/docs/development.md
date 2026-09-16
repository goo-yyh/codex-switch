---
title: 开发与测试
description: Codex Switch 开发与测试与使用说明。
---

## 仓库结构

```text
apps/desktop/             React + Vite 桌面界面
apps/desktop/src-tauri/   Tauri 原生壳与平台能力
apps/website/             Astro + Starlight 官网和文档
crates/core/              配置事务、路由、协议转换、SQLite
packages/                共享设计变量与服务商预设
```

## 安装与检查

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm build
cargo check -p codex-switch-desktop
pnpm check:secrets
```

## 安全的界面预览

```sh
pnpm dev
pnpm website
```

`pnpm dev` 仅启动网页界面，使用内存模拟数据，不调用模型、不读写 Codex 配置或系统凭据库。官网是独立静态构建。

`pnpm desktop` 会启动真实桌面应用；它会访问本机配置与凭据库，开启后会修改配置。不能把这个命令当作无副作用的单元测试。

## 供应商 HTTP 测试

在仓库根目录复制 `.env.example` 为 `.env` 并填入密钥，然后显式运行：

```sh
pnpm test:providers --all
# 或只测一个
pnpm test:providers deepseek
```

测试从指定文件读取密钥，不通过命令行传递密钥；每个服务最多发送少量受限测试请求。不读取真实 Codex 配置，不启动 Codex App，不执行模型返回的工具。结果为脱敏回执。

## 离线测试边界

单元测试注入临时目录、内存数据库和合成 SSE，不创建默认 Codex 路径、不读系统 Keychain。CI 不运行供应商测试，不需要模型密钥。

实现依据及完整设计见仓库 `docs/`。参考 CC Switch 的架构和路由行为，本项目界面与实现独立编写。

## 发布信息与独立文档入口

`packages/product-info/product.json` 集中保存官网和应用显示的版本、渠道、仓库、下载地址及 SHA-256 校验值。安装包地址和校验值同时填写后，下载页才显示下载按钮；正式发布前还须同步 Cargo / package / Tauri 的构建版本。当前没有公开安装包地址。

应用构建时可设置 `VITE_PUBLIC_DOCS_URL` 为独立产品文档的 HTTPS 地址；未设置时使用源码文档入口。官网构建使用 `PUBLIC_SITE_URL` 配置站点地址。两者均不依赖原有 Codex 中文站。不要在这两个公开构建变量里放凭据。

修复回归在核心层注入模拟网关、模拟凭据库、临时配置目录和内存 SQLite；前端模拟全部原生命令，不运行真实 Codex App。
