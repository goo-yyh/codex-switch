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

### macOS 首次打开 / First launch

**当前为 Beta 测试版，尚未使用正式的发布者证书签名，macOS 版本也未经 Apple 公证。**确认安装包来自本项目 GitHub Release 且 SHA-256 校验一致后，先将 Codex Switch 拖入「应用程序」，再打开「终端」，粘贴以下命令并按回车：

```bash
xattr -dr com.apple.quarantine "/Applications/Codex Switch.app"
```

执行后，从「应用程序」重新打开 Codex Switch。若出现“已损坏，无法打开”提示，可按此步骤处理。该命令仅移除 Codex Switch 的下载隔离标记，不会关闭系统全局安全保护，也不能修复真正损坏的文件。若仍无法打开，请保留报错信息并反馈。

**This is a Beta release. The installers are not yet publisher-signed, and the macOS app is not notarized by Apple.** After confirming the installer comes from this project’s GitHub Release and its SHA-256 checksum matches, drag Codex Switch into **Applications**, open **Terminal**, paste the following command, and press Return:

```bash
xattr -dr com.apple.quarantine "/Applications/Codex Switch.app"
```

Then reopen Codex Switch from **Applications**. Use these steps if macOS reports that the app “is damaged and can’t be opened.” This command only removes the download quarantine attribute from Codex Switch; it does not disable system-wide security or repair damaged files. If the app still cannot open, report the error message.

macOS 校验 / Verify on macOS: `shasum -a 256 <installer.dmg>`
Windows 校验 / Verify on Windows: `Get-FileHash <installer.exe> -Algorithm SHA256`

应用内更新提示只提供下载，不会自动安装。预发布版本不会触发稳定版更新提示。
In-app updates offer a download only; they do not install automatically. Prereleases are excluded from stable update notifications.
