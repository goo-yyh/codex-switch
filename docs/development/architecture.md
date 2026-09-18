# 仓库结构与维护边界

## 目录职责

```text
apps/
  desktop/
    src/
      App.tsx                  导航、全局锁定蒙层与确认弹窗
      api/                     IPC 类型、Tauri 通信、隔离的浏览器模拟
      hooks/                   状态刷新、操作互斥和错误反馈
      i18n/                    中英文界面文案、插值与应用诊断翻译
      components/              无业务状态的基础控件与厂商标识
      features/
        profiles/              配置列表、编辑草稿、模型能力与凭据复用提示
        settings/              通用设置与远程压缩偏好
        updates/               后台检查触发与新版本下载提示
    src-tauri/src/
      main.rs                  应用装配与事件注册
      state.rs                 共享状态与命令错误类型
      commands.rs              IPC 输入校验、操作锁与结果序列化
      service.rs               托盘和 IPC 共用的启停、选择配置逻辑
      tray/                    托盘菜单与 macOS 连续多选
      platform.rs              系统凭据库和 Codex 应用操作
      app_menu.rs              macOS 应用菜单
      locale.rs                语言偏好的读取、持久化值与原生菜单文案选择
      updates.rs               GitHub 正式版本检查、版本比较与请求缓存
      model_registry.rs        公共模型目录读取、增量合并与 SQLite 缓存
    src-tauri/examples/        不访问真实配置的原生托盘测试程序
  website/
    src/pages/                 robots.txt 等静态端点
    src/components/            共享网站、主题切换与 SEO 组件
    src/content/docs/          中英文文档内容
crates/
  core/src/
    profiles.rs                配置校验、共享密钥与逐模型路由展开
    connections.rs             凭据作用域与保存失败补偿
    store.rs                   SQLite 事务、旧格式迁移与历史别名
    activation.rs              网关、配置文件、数据库的启用事务
    config.rs                  文件锁、原子写入、恢复日志和配置投影
    gateway/
      mod.rs                   认证、HTTP 路由、转发与运行时
      history.rs               按路由隔离的会话及工具调用续接
      http.rs                  禁止重定向、超时和响应体大小限制
      probe.rs                 用户显式触发的连通测试
      tests.rs                 网关行为测试
      review_regressions.rs    历史问题的回归测试
    protocol.rs                Chat / Responses 和 SSE 适配
    providers.rs               预设读取、地址规范化与模型目录
    bin/                       显式真实供应商测试工具
  cc-switch-codex/              固定版本的上游代码，保持来源和校验值
packages/                      品牌、设计变量、产品信息、厂商预设
scripts/                       构建、部署、SEO、来源和凭据扫描工具
docs/                          开发说明、设计依据、研究与历史验证记录
```

测试与对应功能就近存放。`App.test.tsx` 验证跨页面操作；`crates/core/tests/` 验证跨模块协议行为。历史报告、品牌设计稿和固定的上游来源属于维护依据，不以“没有运行时引用”为理由删除。

## 关键调用链

1. **保存配置**：`ProfileEditor` → `save_profile` → `prepare_profile` → `persist_connection` → `Store::save_profile`。一个配置共享一个凭据，模型路由和配置元数据在同一数据库事务中写入；失败只清理本次新建的凭据。保存不发网络请求。
2. **测试配置**：`probe_endpoint` 校验并解析当前草稿，逐模型调用 `gateway::probe`。支持取消，不保存草稿；真实调用只由用户明确点击测试或显式运行测试工具触发。
3. **选择配置**：页面或托盘 → `select_profiles_inner` → `Store::select_profiles`。只保存选择，不启动服务、不写 Codex 配置。
4. **开启服务**：页面或托盘 → `set_enabled_inner` → `enable_inner` → `activate_with_options`。先准备路由与凭据，再按恢复事务提交配置和路由状态。
5. **关闭服务**：先恢复原配置，保留外部修改的副本。正在运行的 Codex 可能仍使用旧的本地地址，因此网关按现有生命周期保留到客户端退出。
6. **切换语言**：顶部语言按钮 → `set_locale` → SQLite `locale` 设置 → 刷新面板和原生菜单。默认中文，重开沿用上次选择；浏览器预览使用独立的 localStorage。只翻译展示文案，不改配置名称、模型 ID 或接口值，文档入口随语言选择对应路径。

## 不应破坏的约束

- 启用后，页面蒙层、控件 disabled 和原生命令的 `ensure_editable` 一起限制修改；前端限制不能代替原生校验。
- `operation` 锁串行化托盘与 IPC 的状态修改。不要持有同步数据库锁跨越 `await`。
- 托盘复选框必须同步保存，并复用现有原生菜单对象。macOS 正在跟踪菜单时，重建菜单或重复更新状态栏尺寸会让菜单关闭。
- 凭据复用要同时核对厂商、地址、完整 URL 模式和逐模型地址。前端只提供提示，以 Rust 校验为准。
- `models[0]` 是默认模型；修改模型顺序、预设上下文或思考档位时，不重写已有用户配置。
- 会话缓存只接收完整响应，并按客户端模型别名隔离；工具调用身份冲突必须报错。拆分模块不改变历史续接语义。
- 保留 SQLite 旧格式迁移、历史路由和旧会话别名。已移除无调用的旧批量创建、选择即启用路径及其专属测试，不移除旧数据的读取兼容。
- 上游 `cc-switch-codex` 的固定源文件不能随本地整理一起格式化或搬迁；使用 `pnpm check:upstream` 验证。

## 验证入口

`pnpm check` 检查桌面 TypeScript 和 Astro；`pnpm test` 运行桌面 UI、核心、上游适配及原生协调测试。Rust 修改还需 `cargo fmt -p codex-switch-core -p codex-switch-desktop --check` 和 `cargo clippy --locked -p codex-switch-core -p codex-switch-desktop --all-targets --no-deps -- -D warnings`。

`pnpm build` 后执行链接、SEO、部署路由和凭据扫描检查。浏览器模拟、离线测试、原生托盘测试和真实供应商测试是不同验证层次，不能互相替代。离线验证不启动真实 Codex，也不读取模型密钥发请求。

## 更新提示与发布约定

应用启动、回到前台和定时触发 `check_update`，原生端成功结果缓存 6 小时，失败后至少间隔 15 分钟重试。检查独立于服务操作锁，只请求公开 GitHub Release 元数据，不读取 API Key，不影响启停和配置编辑。

发布者需在 `goo-yyh/codex-switch` 发布语义化版本标签（例如 `v0.2.0`）的正式 Release，并上传 macOS `.dmg` 或 Windows `.exe` 安装包。仅显示比当前原生版本更新、且当前系统与架构已有非空安装包的版本；草稿、预发布、源码归档和未完成的附件不触发提示。请同步更新 Cargo 工作区、Tauri 配置和产品元数据版本。现有 CI 仍只上传构建产物，不会自动发布 Release。

安装包名称需保留 Tauri 的架构标识，例如 `_aarch64.dmg`、`_x64.dmg`、`_x64-setup.exe` 或 `_arm64-setup.exe`；macOS 也支持 `_universal.dmg`，优先选择原生架构包。架构不明或不匹配时不提示更新。

按钮直接打开本仓库 `releases/download/<tag>/<asset>` 的安装包链接，由系统默认浏览器下载，不进入 Release 页面，不执行安装或重启。浏览器预览默认无更新；使用 `?previewUpdate=available` 可展示 0.2.0 的模拟提示，不请求 GitHub，也不表示该版本已发布。


## 远程模型目录

网站从公共预设生成 `/registry/models-v1.json`。`schemaVersion` 是格式版本（目前为 1），`version` 是数据版本，桌面和网站共同读取 `packages/provider-registry/version.json`，初始为 `1`；`revision` 仅用于内容哈希追踪，不参与更新判断。发布模型数据时手动递增 `version`，不要用格式版本判断数据是否更新。

原生启动先加载内置预设及 SQLite `model_registry_v1` 缓存，再后台请求固定 HTTPS 地址 `https://www.codex-switch.com/registry/models-v1.json`。每次进程启动检查一次，不占用服务操作锁；请求超时 10 秒、响应上限 1 MiB，禁止重定向。远程版本相同或更低则跳过合并和写入，首次两端均为 `1` 时无需建立缓存。只有远程版本更高才合并；缺少 `version` 的旧接口和请求失败均保留本地数据，不弹出错误。

版本变化后，按厂商与已有地址/协议识别服务范围，只追加不存在的模型能力，已有默认值保持不变。普通 API 与套餐的推荐候选分别最多 5 个；移出推荐列表的模型元数据仍保留在缓存中。不会导入新厂商、接口地址、协议或指令覆盖；这些改动仍需应用更新。排除名单继续生效。

后台更新只写模型缓存，不写用户配置、默认模型、勾选状态、凭据或 Codex 配置。保存缓存成功后发出 `model-registry-updated`，界面通过 snapshot 刷新候选，保留未保存的编辑草稿。用户主动勾选一个新模型且本地没有其设置时，才复制对应服务范围的完整默认能力；已有自定义值（包括显式空配置）不覆盖。上下文已是预留 20% 后的值，不再重复乘 80%。

离线单元测试覆盖版本跳过、合并优先级、套餐隔离、缓存重开、失败回退和编辑草稿；`cargo test --locked -p codex-switch-desktop live_public_registry_can_be_downloaded_and_merged -- --ignored` 仅显式读取公开 CDN，不调用模型供应商，也不启动 Codex。
