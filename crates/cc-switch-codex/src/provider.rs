use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CodexChatReasoningConfig {
    #[serde(rename = "supportsThinking", skip_serializing_if = "Option::is_none")]
    pub supports_thinking: Option<bool>,
    #[serde(rename = "supportsEffort", skip_serializing_if = "Option::is_none")]
    pub supports_effort: Option<bool>,
    #[serde(rename = "thinkingParam", skip_serializing_if = "Option::is_none")]
    pub thinking_param: Option<String>,
    #[serde(rename = "effortParam", skip_serializing_if = "Option::is_none")]
    pub effort_param: Option<String>,
    #[serde(rename = "effortValueMode", skip_serializing_if = "Option::is_none")]
    pub effort_value_mode: Option<String>,
    /// 声明性字段：标注上游 reasoning 的回传位置（reasoning_content / reasoning /
    /// reasoning_details / think_tags）。当前响应侧 `extract_reasoning_field_text`
    /// 靠穷举字段提取、并不读取本字段；保留作文档说明与未来按格式分发（如 think_tags）的预留。
    #[serde(rename = "outputFormat", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// 运行时字段（不持久化、不进 meta）：当前请求模型在平台侧声明的合法 effort
    /// 档位，由 resolve 按请求模型从供应商 `settings_config.modelCatalog` 的
    /// `reasoningLevels`（逐模型声明，见 #6228）查表填充。仅 "zen" 值映射消费：
    /// Some → 钳到合法档；None → 不发 effort 字段（模型未收录或为 toggle 型）。
    #[serde(skip)]
    pub effort_levels: Option<Vec<String>>,
}
