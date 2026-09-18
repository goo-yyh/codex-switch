import type { Locale } from './index';
import { en } from './en';

// Translate application-owned diagnostics at the presentation boundary. Keep
// unknown upstream/OS messages intact so their diagnostic details are not lost.
const diagnostics: Record<string, string> = {
  '测试标识无效。': 'Invalid test identifier.',
  '验证标识无效。': 'Invalid validation identifier.',
  '验证请求过多。': 'Too many validation requests.',
  '配置测试已取消。': 'Configuration test cancelled.',
  配置测试通过: 'Configuration test passed',
  '存储不可用。': 'Storage is unavailable.',
  '连接存储不可用。': 'Connection storage is unavailable.',
  '还有请求正在进行，请等待当前任务结束后再重启。':
    'Requests are still running. Wait for the current task to finish before restarting.',
  '请先在 Codex 中完成任务并正常退出，再点击打开。原有任务不会被强制结束。':
    'Finish your task and quit Codex normally before opening it again. Existing tasks will not be forcibly stopped.',
  '无法设置登录时启动。': 'Unable to change the start-at-login preference.',
  '请先退出 Codex；关闭本窗口会继续在后台提供连接。':
    'Quit Codex first. Closing this window keeps the connection running in the background.',
  '未找到 Codex，请安装官方应用。': 'Codex was not found. Install the official app.',
  '无法打开 Codex。': 'Unable to open Codex.',
  '此平台暂未支持。': 'This platform is not supported yet.',
  '未能打开 Codex，请检查应用是否已安装。':
    'Unable to open Codex. Check that the app is installed.',
  链接无效: 'Invalid link',
  '只允许打开 HTTPS 链接。': 'Only HTTPS links are allowed.',
  '无法打开链接。': 'Unable to open the link.',
  '无法访问系统凭据库。': 'Unable to access the system credential store.',
  '无法读取此连接的密钥，请重新填写。': 'Unable to read this connection’s key. Enter it again.',
  '密钥保存失败，未写入明文。': 'Failed to save the key. No plaintext key was written.',
  '无法删除此连接的凭据。': 'Unable to delete this connection’s credentials.',
  'Codex 未接受退出请求，请手动正常退出。':
    'Codex did not accept the quit request. Quit it normally yourself.',
  'Codex 仍在运行，请先完成任务并正常退出。':
    'Codex is still running. Finish your task and quit it normally.',
  'Codex 未能正常退出，请手动退出后再打开。':
    'Codex did not quit normally. Quit it yourself before reopening.',
  'Codex 仍在运行，可能有未完成任务；请手动正常退出后重试。':
    'Codex is still running and may have unfinished tasks. Quit it normally and retry.',
  '文件路径无效。': 'Invalid file path.',
  '拒绝写入符号链接配置文件。': 'Refusing to write to a symlinked configuration file.',
  '配置文件是符号链接，请使用独立的普通配置文件。':
    'The configuration file is a symlink. Use a regular configuration file.',
  '另一个配置操作正在进行，请稍后重试。':
    'Another configuration operation is running. Try again shortly.',
  '配置不是有效的 UTF-8 文本。': 'The configuration is not valid UTF-8 text.',
  '配置位置与备份不一致。': 'The configuration location does not match the backup.',
  '配置无法解析，保留所有模型目录。':
    'Unable to parse the configuration. All model catalogs were preserved.',
  '配置位置与备份不一致，未执行恢复。':
    'The configuration location does not match the backup. Nothing was restored.',
  '现有 config.toml 无法解析，原文件未修改。':
    'Unable to parse the existing config.toml. The original file was not changed.',
  '检测到用户自定义模型目录，请先在 Codex 中处理该目录配置；原配置未修改。':
    'A custom model catalog was found. Resolve it in Codex first. The original configuration was not changed.',
  'model_providers 必须是 TOML 表，原配置未修改。':
    'model_providers must be a TOML table. The original configuration was not changed.',
  '原配置不存在，请重新添加。': 'The original configuration no longer exists. Add it again.',
  '请选择 1 到 20 个模型。': 'Select between 1 and 20 models.',
  '模型不能重复。': 'Duplicate models are not allowed.',
  '请填写有效的模型名称。': 'Enter a valid model name.',
  '请填写 API Key。': 'Enter an API key.',
  '厂商或地址已改变，请重新填写 API Key。': 'The provider or URL changed. Enter the API key again.',
  '密钥为空，请重新填写。': 'The key is empty. Enter it again.',
  '请填写连接名称和模型。': 'Enter a connection name and model.',
  '连接标识无效。': 'Invalid connection identifier.',
  '候选地址或模型能力配置过多。': 'Too many candidate URLs or model capability overrides.',
  '模型说明过长。': 'The model description is too long.',
  '模型上下文长度无效。': 'Invalid model context window.',
  '模型思考档位无效。': 'Invalid model reasoning level.',
  '默认思考档位必须属于支持的档位。':
    'The default reasoning level must be one of the supported levels.',
  '请先设置支持的思考档位。': 'Set the supported reasoning levels first.',
  '输入模态必须包含 text，可选 image。': 'Input modalities must include text; image is optional.',
  '上下文长度应在 4096 到 2000000 之间。': 'Context window must be between 4096 and 2000000.',
  'API 地址无效。': 'Invalid API URL.',
  '完整 URL 无法推导压缩端点，请使用以 /responses 结尾的地址或基础地址模式。':
    'Cannot infer the compaction endpoint from this full URL. Use a URL ending in /responses or base URL mode.',
  'API 地址无效，请填写完整的 https 地址。': 'Invalid API URL. Enter a complete HTTPS URL.',
  '服务地址必须使用 HTTPS，且不能包含账号、查询参数或片段。':
    'The service URL must use HTTPS and must not contain credentials, unsupported query parameters or fragments.',
  '不能将本地路由设置为上游服务。': 'The local router cannot be used as the upstream service.',
  '密钥无效，或当前套餐没有此权限。': 'Invalid key or insufficient permissions for this plan.',
  '服务地址或模型不存在。': 'The service URL or model does not exist.',
  '服务限流或可用额度不足。': 'The service is rate limited or has insufficient quota.',
  '模型或请求参数不兼容，请检查套餐与接口格式。':
    'Incompatible model or parameters. Check the plan and API format.',
  '供应商暂时无法处理请求。': 'The provider cannot handle this request right now.',
  '连接失败，请检查网络或 API 地址。': 'Connection failed. Check your network or API URL.',
  连接验证通过: 'Connection test passed',
  '接口已响应，但没有完整文本结果': 'The API responded without a complete text result',
  连接测试超时: 'Connection test timed out',
  '配置选择无效。': 'Invalid configuration selection.',
  '配置不存在。': 'Configuration not found.',
  '正在处理其他操作，请稍后再试。': 'Another operation is running. Try again shortly.',
  '配置被其他程序修改，请重试。': 'Another program changed the configuration. Please retry.',
};

export function localizeMessage(locale: Locale, value: string): string {
  if (locale !== 'en' || !value) return value;
  const known = Object.hasOwn(en, value)
    ? en[value as keyof typeof en]
    : Object.hasOwn(diagnostics, value)
      ? diagnostics[value]
      : undefined;
  if (known) return known;
  if (value.startsWith('Error: ')) return `Error: ${localizeMessage(locale, value.slice(7))}`;
  let match = value.match(/^配置测试通过，(\d+) 个模型请求成功。$/);
  if (match) return `Configuration test passed: ${match[1]} model requests succeeded.`;
  match = value.match(/^(.+) 的密钥不可用，请编辑配置重新填写。$/);
  if (match)
    return `The key for ${match[1]} is unavailable. Edit the configuration and enter it again.`;
  match = value.match(/^(.+) 的密钥为空。$/);
  if (match) return `The key for ${match[1]} is empty.`;
  match = value.match(/^(.+) · (\d+) ms$/);
  if (match) return `${localizeMessage(locale, match[1])} · ${match[2]} ms`;
  match = value.match(/^([^：]+)：(.+)$/s);
  if (match) {
    const detail = localizeMessage(locale, match[2]);
    if (detail !== match[2]) return `${match[1]}: ${detail}`;
  }
  return value;
}
