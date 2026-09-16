# 服务商标识

资源来源：[Lobe Icons](https://github.com/lobehub/lobe-icons) 的 `@lobehub/icons-static-svg@1.95.0`，于 2026-09-16 获取。SVG 原样保留，许可说明见 `LICENSE.lobe-icons`。这些标识只用于识别对应服务商。

| 本项目 ID | 服务商      | 上游文件           |
| --------- | ----------- | ------------------ |
| qianwen   | 千问 / Qwen | qwen-color.svg     |
| minimax   | MiniMax     | minimax-color.svg  |
| zhipu     | 智谱 GLM    | zhipu-color.svg    |
| kimi      | Kimi        | kimi-color.svg     |
| deepseek  | DeepSeek    | deepseek-color.svg |

固定版本源文件链接格式：`https://unpkg.com/@lobehub/icons-static-svg@1.95.0/icons/<上游文件>`。

- 通过 `packages/provider-registry/logos.ts` 按不可编辑的服务商 ID 匹配，不根据用户填写的连接名猜测。
- Kimi 原图是白色 K 和蓝点，使用深色底板保证浅色界面也可识别，不反色或改写路径。
- App、服务选择和官网使用同一组本地 SVG，运行时无需访问 CDN。
- 自定义中转站使用通用终端标记，不冒用某个服务商的身份。
- `sources.json` 记录 npm 包完整性和每个 SVG 的 SHA-256。
