use serde::Serialize;
use std::process::Command;

#[cfg(target_os = "windows")]
fn background_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    // Redirecting stdout alone does not stop Windows from creating a console.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub installed: bool,
    pub running: bool,
}
pub fn app_info() -> AppInfo {
    #[cfg(target_os = "macos")]
    {
        let installed = objc2_app_kit::NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&objc2_foundation::NSString::from_str(
                "com.openai.codex",
            ))
            .is_some();
        let running =
            !objc2_app_kit::NSRunningApplication::runningApplicationsWithBundleIdentifier(
                &objc2_foundation::NSString::from_str("com.openai.codex"),
            )
            .is_empty();
        AppInfo { installed, running }
    }
    #[cfg(target_os = "windows")]
    {
        let installed = windows_path().is_some();
        let running = background_command("tasklist.exe")
            .args(["/FI", "IMAGENAME eq Codex.exe", "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("Codex.exe"))
            .unwrap_or(false);
        AppInfo { installed, running }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        AppInfo {
            installed: false,
            running: false,
        }
    }
}
#[cfg(target_os = "windows")]
fn windows_path() -> Option<std::path::PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    [
        "Programs/Codex/Codex.exe",
        "Codex/Codex.exe",
        "Microsoft/WindowsApps/Codex.exe",
    ]
    .iter()
    .map(|p| std::path::PathBuf::from(&local).join(p))
    .find(|p| p.exists())
}
pub fn open_codex() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let r = Command::new("/usr/bin/open")
        .args(["-b", "com.openai.codex"])
        .status();
    #[cfg(target_os = "windows")]
    let r = Command::new(windows_path().ok_or("未找到 Codex，请安装官方应用。")?)
        .spawn()
        .map(|_| ())
        .map_err(|_| "无法打开 Codex。".to_string());
    #[cfg(target_os = "windows")]
    return r;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Err("此平台暂未支持。".into());
    #[cfg(target_os = "macos")]
    if r.map(|s| s.success()).unwrap_or(false) {
        Ok(())
    } else {
        Err("未能打开 Codex，请检查应用是否已安装。".into())
    }
}
pub fn open_url(url: &str) -> Result<(), String> {
    let u = url::Url::parse(url).map_err(|_| "链接无效")?;
    if u.scheme() != "https" || !u.username().is_empty() {
        return Err("只允许打开 HTTPS 链接。".into());
    }
    #[cfg(target_os = "macos")]
    let result = Command::new("/usr/bin/open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = background_command("rundll32.exe")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Err("此平台暂未支持。".into());
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    result.map(|_| ()).map_err(|_| "无法打开链接。".into())
}
pub struct Vault;
impl Vault {
    fn entry(id: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new("com.codexdocs.switch", id).map_err(|_| "无法访问系统凭据库。".into())
    }
    pub fn read(id: &str) -> Result<String, String> {
        Self::entry(id)?
            .get_password()
            .map_err(|_| "无法读取此连接的密钥，请重新填写。".into())
    }
    pub fn write(id: &str, key: &str) -> Result<(), String> {
        Self::entry(id)?
            .set_password(key)
            .map_err(|_| "密钥保存失败，未写入明文。".into())
    }
    pub fn delete(id: &str) -> Result<(), String> {
        match Self::entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("无法删除此连接的凭据。".into()),
        }
    }
}

/// Ask the application to quit normally. Never force-kill a process.
pub async fn close_codex() -> Result<(), String> {
    if !app_info().running {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let sent = {
            let apps = objc2_app_kit::NSRunningApplication::runningApplicationsWithBundleIdentifier(
                &objc2_foundation::NSString::from_str("com.openai.codex"),
            );
            apps.iter().all(|app| app.terminate())
        };
        if !sent {
            return Err("Codex 未接受退出请求，请手动正常退出。".into());
        }
        for _ in 0..30 {
            if !app_info().running {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err("Codex 仍在运行，请先完成任务并正常退出。".into())
    }
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = tokio::process::Command::from(background_command("powershell.exe"));
        c.args(["-NoProfile","-NonInteractive","-Command","Get-Process -Name Codex -ErrorAction SilentlyContinue | ForEach-Object { [void]$_.CloseMainWindow() }"]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Err("此平台暂未支持。".into());
    #[cfg(target_os = "windows")]
    {
        command
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(20), command.status()).await;
        if !matches!(result,Ok(Ok(status)) if status.success()) {
            return Err("Codex 未能正常退出，请手动退出后再打开。".into());
        }
        for _ in 0..30 {
            if !app_info().running {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err("Codex 仍在运行，可能有未完成任务；请手动正常退出后重试。".into())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    fn console_probe() -> Command {
        let mut command = background_command("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"Add-Type -Namespace ConsoleProbe -Name Native -MemberDefinition '[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern System.IntPtr GetConsoleWindow();'; [ConsoleProbe.Native]::GetConsoleWindow().ToInt64()"#,
        ]);
        command
    }

    #[tokio::test]
    async fn background_commands_have_no_console_and_still_capture_output() {
        // Check the child's actual console handle, including the Tokio conversion
        // used by close_codex, instead of only inspecting command configuration.
        let sync_output = console_probe().output().unwrap();
        let async_output = tokio::process::Command::from(console_probe())
            .output()
            .await
            .unwrap();
        for output in [sync_output, async_output] {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
        }
    }
}

impl codex_switch_core::connections::Credentials for Vault {
    fn read(&self, id: &str) -> codex_switch_core::Result<String> {
        Vault::read(id).map_err(codex_switch_core::message)
    }
    fn write(&self, id: &str, key: &str) -> codex_switch_core::Result<()> {
        Vault::write(id, key).map_err(codex_switch_core::message)
    }
    fn delete(&self, id: &str) -> codex_switch_core::Result<()> {
        Vault::delete(id).map_err(codex_switch_core::message)
    }
}
