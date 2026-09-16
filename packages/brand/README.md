# Codex Switch icon

白色圆角底板，蓝紫色拨动开关，白色滑钮上带有终端提示符 `>_`，表达编程工具和模型路由的开关。采用用户选定的图片原稿，不使用 Codex 的官方花形轮廓、Logo 或角标。产品与 Codex 的关联通过页面文字说明。

- `icon.png`：生成的高分辨率图标母版。
- `mark.png`：网页与应用页眉使用的 256 px 导出。
- `pnpm icons`：从母版重新生成 macOS ICNS、Windows ICO、多尺寸 PNG，并同步官网资源。

原生导出位于 `apps/desktop/src-tauri/icons/`。官网资源位于 `apps/website/public/mark.png`。不再使用旧的绿色双箭头 SVG。

设计日期：2026-09-15。使用 imagegen 生成，导出过程只转换格式与尺寸。没有读取或修改 Codex 的用户配置。

来源：用户于 2026-09-15 上传并选定的开关图标，最初使用内置 imagegen 生成；原始提示词见 `prompt.txt`。用户上传版本为导出母版。
