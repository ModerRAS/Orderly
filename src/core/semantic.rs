//! AI语义分析模块
//! 
//! 负责与AI模型（如qwen3-30b-a3b）交互，进行文件语义分析。
//! 
//! 设计原则：
//! - AI只负责理解和建议，不负责决策
//! - 输入必须"瘦身"，不喂全文
//! - 输出必须是结构化JSON
//! - 禁止AI自由发挥

use crate::core::models::{
    AIConfig, FileDescriptor, MoveSuggestion, RuleAction, RuleCondition, 
    RuleDefinition, SemanticResult, SuggestionSource,
};
use crate::core::scanner::get_content_summary;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiApiKind {
    OllamaGenerate,
    OpenAIChatCompletions,
    OpenAIResponses,
}

/// AI语义分析引擎
pub struct SemanticEngine {
    /// AI配置
    config: AIConfig,
    /// HTTP客户端
    client: reqwest::Client,
    /// 输出基础路径
    output_base: PathBuf,
}

/// 文件档案（发送给AI的结构化输入）
#[derive(Debug, Serialize)]
struct FileProfile {
    name: String,
    extension: String,
    size_kb: f64,
    modified_year: i32,
    modified_month: u32,
    content_summary: Option<String>,
}

/// AI语义分析响应
#[derive(Debug, Deserialize)]
struct SemanticResponse {
    tags: Vec<String>,
    entities: Vec<String>,
    year: Option<i32>,
    confidence: f32,
    explanation: String,
}

/// AI路径建议响应
#[derive(Debug, Deserialize)]
struct PathSuggestionResponse {
    suggested_path: String,
    reason: String,
    confidence: f32,
}

/// AI规则抽取响应
#[derive(Debug, Deserialize)]
struct RuleExtractionResponse {
    rule_name: String,
    condition: ExtractedCondition,
    action: ExtractedAction,
    priority: u8,
}

#[derive(Debug, Deserialize)]
struct ExtractedCondition {
    semantic_tags: Option<Vec<String>>,
    file_extensions: Option<Vec<String>>,
    filename_keywords: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ExtractedAction {
    move_to: String,
}

impl SemanticEngine {
    /// 创建新的语义引擎
    pub fn new(config: AIConfig, output_base: PathBuf) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            output_base,
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: AIConfig) {
        self.config = config;
    }

    /// 分析单个文件的语义
    pub async fn analyze_file(&self, file: &FileDescriptor) -> Result<SemanticResult> {
        // 原子文件不分析
        if file.atomic {
            return Ok(SemanticResult::default());
        }

        // 目录不分析
        if file.is_directory {
            return Ok(SemanticResult::default());
        }

        // 构建文件档案
        let profile = self.build_file_profile(file);

        // 构建提示词
        let prompt = self.build_semantic_prompt(&profile);

        // 调用AI
        let response = self.call_ai(&prompt).await?;

        // 解析响应
        self.parse_semantic_response(&response)
    }

    /// 为文件生成路径建议
    pub async fn suggest_path(
        &self,
        file: &FileDescriptor,
        candidate_paths: &[String],
    ) -> Result<MoveSuggestion> {
        let profile = self.build_file_profile(file);
        let prompt = self.build_path_suggestion_prompt(&profile, candidate_paths);

        let response = self.call_ai(&prompt).await?;
        let suggestion = self.parse_path_suggestion(&response)?;

        Ok(MoveSuggestion {
            target_path: self.output_base.join(&suggestion.suggested_path),
            reason: suggestion.reason,
            source: SuggestionSource::AI,
            confidence: suggestion.confidence,
        })
    }

    /// 从用户反馈中抽取规则
    pub async fn extract_rule(&self, user_feedback: &str, context: &str) -> Result<RuleDefinition> {
        let prompt = self.build_rule_extraction_prompt(user_feedback, context);
        let response = self.call_ai(&prompt).await?;
        let extracted = self.parse_rule_extraction(&response)?;

        let condition = RuleCondition {
            semantic_tags: extracted.condition.semantic_tags.unwrap_or_default(),
            file_extensions: extracted.condition.file_extensions.unwrap_or_default(),
            filename_keywords: extracted.condition.filename_keywords.unwrap_or_default(),
            ..Default::default()
        };

        let action = RuleAction {
            move_to: extracted.action.move_to,
        };

        let mut rule = RuleDefinition::new(extracted.rule_name, condition, action);
        rule.priority = extracted.priority;

        Ok(rule)
    }

    /// 构建文件档案
    fn build_file_profile(&self, file: &FileDescriptor) -> FileProfile {
        // 尝试获取内容摘要（仅文本文件）
        let content_summary = if self.is_text_file(&file.extension) {
            get_content_summary(&file.full_path, 500).ok()
        } else {
            None
        };

        FileProfile {
            name: file.name.clone(),
            extension: file.extension.clone(),
            size_kb: file.size as f64 / 1024.0,
            modified_year: file.modified_at.format("%Y").to_string().parse().unwrap_or(2024),
            modified_month: file.modified_at.format("%m").to_string().parse().unwrap_or(1),
            content_summary,
        }
    }

    /// 判断是否为文本文件
    fn is_text_file(&self, extension: &str) -> bool {
        let text_extensions = [
            ".txt", ".md", ".json", ".xml", ".html", ".css", ".js", ".ts",
            ".py", ".rs", ".go", ".java", ".c", ".cpp", ".h", ".hpp",
            ".yaml", ".yml", ".toml", ".ini", ".cfg", ".log", ".csv",
        ];
        text_extensions.contains(&extension.to_lowercase().as_str())
    }

    /// 构建语义分析提示词
    fn build_semantic_prompt(&self, profile: &FileProfile) -> String {
        format!(
            r#"你是一个文件整理助手，请分析以下文件的语义信息。

文件信息：
- 文件名: {}
- 扩展名: {}
- 大小: {:.2} KB
- 修改年份: {}
- 修改月份: {}
{}

请根据以上信息，输出以下JSON格式（不要输出其他内容）：
{{
  "tags": ["标签1", "标签2"],
  "entities": ["实体1", "实体2"],
  "year": 2023,
  "confidence": 0.85,
  "explanation": "判断理由"
}}

要求：
1. tags: 描述文件类型、用途、主题的标签（如 invoice, photo, work, personal）
2. entities: 识别出的实体（如公司名、人名、项目名）
3. year: 从文件名或内容推断的年份，如果无法确定则为null
4. confidence: 分析置信度 (0-1)
5. explanation: 简短的判断理由

只输出JSON，不要输出其他任何内容。"#,
            profile.name,
            profile.extension,
            profile.size_kb,
            profile.modified_year,
            profile.modified_month,
            profile
                .content_summary
                .as_ref()
                .map(|s| format!("- 内容摘要: {}", s))
                .unwrap_or_default()
        )
    }

    /// 构建路径建议提示词
    fn build_path_suggestion_prompt(&self, profile: &FileProfile, candidates: &[String]) -> String {
        format!(
            r#"你是一个文件整理助手，请为以下文件推荐最合适的存放路径。

文件信息：
- 文件名: {}
- 扩展名: {}
- 大小: {:.2} KB
- 修改年份: {}
{}

候选路径：
{}

请输出以下JSON格式（不要输出其他内容）：
{{
  "suggested_path": "建议的路径",
  "reason": "选择理由",
  "confidence": 0.85
}}

要求：
1. 如果候选路径中有合适的，从中选择
2. 如果候选路径都不合适，可以建议新路径
3. 路径支持变量：{{year}}, {{month}}, {{extension}}
4. confidence: 推荐置信度 (0-1)

只输出JSON，不要输出其他任何内容。"#,
            profile.name,
            profile.extension,
            profile.size_kb,
            profile.modified_year,
            profile
                .content_summary
                .as_ref()
                .map(|s| format!("- 内容摘要: {}", s))
                .unwrap_or_default(),
            candidates
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{}. {}", i + 1, p))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// 构建规则抽取提示词
    fn build_rule_extraction_prompt(&self, user_feedback: &str, context: &str) -> String {
        format!(
            r#"你是规则工程师，请将用户的自然语言反馈抽象为可复用的分类规则。

用户反馈：
{}

上下文（用户修改了哪些文件的分类）：
{}

请输出以下JSON格式（不要输出其他内容）：
{{
  "rule_name": "规则名称",
  "condition": {{
    "semantic_tags": ["标签1", "标签2"],
    "file_extensions": [".pdf", ".jpg"],
    "filename_keywords": ["关键词1", "关键词2"]
  }},
  "action": {{
    "move_to": "目标路径模板"
  }},
  "priority": 70
}}

要求：
1. rule_name: 简洁描述规则用途
2. condition: 至少填写一个匹配条件
3. move_to: 支持变量 {{year}}, {{month}}, {{extension}}
4. priority: 0-100，数字越大优先级越高，一般用户规则建议60-80

只输出JSON，不要输出其他任何内容。"#,
            user_feedback,
            context
        )
    }

    /// 调用AI API
    async fn call_ai(&self, prompt: &str) -> Result<String> {
        let (kind, endpoint) = self.normalize_ai_endpoint()?;
        match kind {
            AiApiKind::OllamaGenerate => self.call_ollama(prompt, &endpoint).await,
            AiApiKind::OpenAIChatCompletions => self.call_openai_chat_completions(prompt, &endpoint).await,
            AiApiKind::OpenAIResponses => self.call_openai_responses(prompt, &endpoint).await,
        }
    }

    fn normalize_ai_endpoint(&self) -> Result<(AiApiKind, String)> {
        let raw = self.config.api_endpoint.trim();
        if raw.is_empty() {
            return Err(anyhow::anyhow!("AI API端点为空"));
        }

        // 统一去掉尾部斜杠，避免后续拼接出现双斜杠
        let endpoint = raw.trim_end_matches('/').to_string();

        // 1) Ollama: 允许用户只填 host（如 http://localhost:11434），自动补齐到 /api/generate
        let looks_like_ollama = endpoint.contains("11434") || endpoint.contains("ollama");
        if looks_like_ollama {
            if endpoint.contains("/api/generate") {
                return Ok((AiApiKind::OllamaGenerate, endpoint));
            }
            return Ok((
                AiApiKind::OllamaGenerate,
                format!("{}/api/generate", endpoint),
            ));
        }

        // 2) OpenAI: 允许用户填 base（如 https://api.openai.com/v1），自动补齐到 /chat/completions
        if endpoint.contains("/v1/responses") {
            return Ok((AiApiKind::OpenAIResponses, endpoint));
        }
        if endpoint.contains("/v1/chat/completions") || endpoint.contains("/chat/completions") {
            return Ok((AiApiKind::OpenAIChatCompletions, endpoint));
        }

        // 常见的 OpenAI 兼容基地址（例如 .../v1 或 .../compatible-mode/v1）
        let is_v1_like_base = endpoint.ends_with("/v1") || endpoint.ends_with("compatible-mode/v1");
        if is_v1_like_base {
            return Ok((
                AiApiKind::OpenAIChatCompletions,
                format!("{}/chat/completions", endpoint),
            ));
        }

        // OpenAI 官方域名但没写 /v1 时，补齐到 /v1/chat/completions
        if endpoint.contains("api.openai.com") && !endpoint.contains("/v1") {
            return Ok((
                AiApiKind::OpenAIChatCompletions,
                format!("{}/v1/chat/completions", endpoint),
            ));
        }

        // 兜底：认为用户填写的是完整 OpenAI 兼容接口路径
        Ok((AiApiKind::OpenAIChatCompletions, endpoint))
    }

    /// 调用Ollama API
    async fn call_ollama(&self, prompt: &str, endpoint: &str) -> Result<String> {
        #[derive(Serialize)]
        struct OllamaRequest {
            model: String,
            prompt: String,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
        }

        let request = OllamaRequest {
            model: self.config.model_name.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(endpoint)
            .json(&request)
            .send()
            .await?
            .json::<OllamaResponse>()
            .await?;

        Ok(response.response)
    }

    /// 调用OpenAI兼容API（Chat Completions）
    async fn call_openai_chat_completions(&self, prompt: &str, endpoint: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: u32,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: MessageContent,
        }

        #[derive(Deserialize)]
        struct MessageContent {
            content: String,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
        }

        let request = OpenAIRequest {
            model: self.config.model_name.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        let mut req = self.client.post(endpoint).json(&request);

        if !self.config.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.config.api_key));
        }

        let response = req.send().await?.json::<OpenAIResponse>().await?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("AI返回空响应"))
    }

    /// 调用 OpenAI Responses API（如果用户配置了 /v1/responses）
    async fn call_openai_responses(&self, prompt: &str, endpoint: &str) -> Result<String> {
        #[derive(Serialize)]
        struct ResponsesRequest {
            model: String,
            input: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_output_tokens: Option<u32>,
        }

        let request = ResponsesRequest {
            model: self.config.model_name.clone(),
            input: prompt.to_string(),
            temperature: Some(self.config.temperature),
            max_output_tokens: Some(self.config.max_tokens),
        };

        let mut req = self.client.post(endpoint).json(&request);
        if !self.config.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.config.api_key));
        }

        let value: serde_json::Value = req.send().await?.json().await?;

        // 尽量兼容不同实现：优先找 output_text，其次尝试 output->content->text
        if let Some(s) = value.get("output_text").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }

        let text = value
            .get("output")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("content"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.iter().find_map(|c| c.get("text").and_then(|t| t.as_str())))
            .map(|s| s.to_string());

        text.ok_or_else(|| anyhow::anyhow!("AI返回空响应"))
    }

    /// 解析语义分析响应
    fn parse_semantic_response(&self, response: &str) -> Result<SemanticResult> {
        // 尝试从响应中提取JSON
        let json_str = self.extract_json(response);
        
        let parsed: SemanticResponse = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("解析AI响应失败: {}, 响应内容: {}", e, response))?;

        Ok(SemanticResult {
            tags: parsed.tags,
            entities: parsed.entities,
            year: parsed.year,
            confidence: parsed.confidence,
            explanation: parsed.explanation,
        })
    }

    /// 解析路径建议响应
    fn parse_path_suggestion(&self, response: &str) -> Result<PathSuggestionResponse> {
        let json_str = self.extract_json(response);
        serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("解析路径建议响应失败: {}", e))
    }

    /// 解析规则抽取响应
    fn parse_rule_extraction(&self, response: &str) -> Result<RuleExtractionResponse> {
        let json_str = self.extract_json(response);
        serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("解析规则抽取响应失败: {}", e))
    }

    /// 从响应中提取JSON
    fn extract_json(&self, response: &str) -> String {
        // 查找JSON开始和结束位置
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                return response[start..=end].to_string();
            }
        }
        response.to_string()
    }
}

/// 模拟AI响应（用于测试或离线模式）
pub fn mock_semantic_analysis(file: &FileDescriptor) -> SemanticResult {
    let mut tags = Vec::new();
    
    // 根据扩展名推断基础标签
    match file.extension.to_lowercase().as_str() {
        ".jpg" | ".jpeg" | ".png" | ".gif" => tags.push("image".to_string()),
        ".mp4" | ".avi" | ".mkv" => tags.push("video".to_string()),
        ".mp3" | ".wav" | ".flac" => tags.push("audio".to_string()),
        ".pdf" => tags.push("document".to_string()),
        ".doc" | ".docx" => tags.push("word".to_string()),
        ".xls" | ".xlsx" => tags.push("excel".to_string()),
        _ => {}
    }

    // 根据文件名关键词添加标签
    let name_lower = file.name.to_lowercase();
    if name_lower.contains("发票") || name_lower.contains("invoice") {
        tags.push("invoice".to_string());
    }
    if name_lower.contains("合同") || name_lower.contains("contract") {
        tags.push("contract".to_string());
    }
    if name_lower.contains("报告") || name_lower.contains("report") {
        tags.push("report".to_string());
    }

    // 尝试从文件名提取年份
    let year = extract_year_from_filename(&file.name);

    SemanticResult {
        tags,
        entities: Vec::new(),
        year,
        confidence: 0.6,
        explanation: "基于文件名和扩展名的本地分析".to_string(),
    }
}

/// 从文件名中提取年份
fn extract_year_from_filename(filename: &str) -> Option<i32> {
    use std::str::FromStr;
    
    // 匹配4位数字年份（2000-2099）
    for word in filename.split(|c: char| !c.is_ascii_digit()) {
        if word.len() == 4 {
            if let Ok(year) = i32::from_str(word) {
                if (2000..=2099).contains(&year) {
                    return Some(year);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year_from_filename("report_2023.pdf"), Some(2023));
        assert_eq!(extract_year_from_filename("2024_invoice.pdf"), Some(2024));
        assert_eq!(extract_year_from_filename("no_year.pdf"), None);
    }
}
