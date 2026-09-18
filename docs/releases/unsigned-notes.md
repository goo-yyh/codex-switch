本版本安装包未使用发布者证书签名；macOS 未经 Apple 公证。
These installers are not publisher-signed. The macOS apps are not notarized by Apple.

| 下载文件 / Asset  | 系统 / Platform                             |
| ----------------- | ------------------------------------------- |
| `*_aarch64.dmg`   | macOS Apple Silicon（M1 及更新芯片）        |
| `*_x64.dmg`       | macOS Intel                                 |
| `*_x64-setup.exe` | Windows x64                                 |
| `SHA256SUMS.txt`  | 安装包 SHA-256 校验值 / Installer checksums |

安装前请关闭 Codex Switch 的服务并退出应用。macOS 将应用拖入 Applications；Windows 运行安装程序。
由于没有发布者签名，macOS Gatekeeper 或 Windows SmartScreen 可能提示或阻止运行。请确认文件来自本仓库并核对校验值；组织管理的设备可能不允许安装。

Before installing, disable the Codex Switch service and quit the app. Drag the macOS app into Applications or run the Windows installer. Gatekeeper / SmartScreen may warn or block unsigned software. Verify the download source and checksum; managed devices may prohibit installation.

macOS 校验 / Verify on macOS: `shasum -a 256 <installer.dmg>`
Windows 校验 / Verify on Windows: `Get-FileHash <installer.exe> -Algorithm SHA256`

应用内更新提示只提供下载，不会自动安装。预发布版本不会触发稳定版更新提示。
In-app updates offer a download only; they do not install automatically. Prereleases are excluded from stable update notifications.
