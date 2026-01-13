//! 对话框组件

use eframe::egui::{self, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiInterfaceKind {
    Ollama,
    OpenAIChatCompletions,
    OpenAIResponses,
    Custom,
}

impl ApiInterfaceKind {
    fn label(&self) -> &'static str {
        match self {
            ApiInterfaceKind::Ollama => "Ollama（/api/generate）",
            ApiInterfaceKind::OpenAIChatCompletions => "OpenAI Chat Completions（/v1/chat/completions）",
            ApiInterfaceKind::OpenAIResponses => "OpenAI Responses（/v1/responses）",
            ApiInterfaceKind::Custom => "自定义（完整URL）",
        }
    }

    fn standard_suffix(&self) -> Option<&'static str> {
        match self {
            ApiInterfaceKind::Ollama => Some("/api/generate"),
            ApiInterfaceKind::OpenAIChatCompletions => Some("/v1/chat/completions"),
            ApiInterfaceKind::OpenAIResponses => Some("/v1/responses"),
            ApiInterfaceKind::Custom => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlSuffixMode {
    Standard,
    Custom,
}

/// 提示词输入对话框
pub struct PromptDialog {
    /// 是否显示
    pub visible: bool,
    /// 标题
    pub title: String,
    /// 提示文本
    pub prompt: String,
    /// 用户输入
    pub input: String,
    /// 上下文信息（显示给用户）
    pub context: String,
}

impl Default for PromptDialog {
    fn default() -> Self {
        Self {
            visible: false,
            title: "输入提示词".to_string(),
            prompt: "请输入您的修正建议...".to_string(),
            input: String::new(),
            context: String::new(),
        }
    }
}

impl PromptDialog {
    /// 显示对话框
    pub fn show(&mut self, title: &str, prompt: &str, context: &str) {
        self.visible = true;
        self.title = title.to_string();
        self.prompt = prompt.to_string();
        self.context = context.to_string();
        self.input.clear();
    }

    /// 渲染对话框
    pub fn render(&mut self, ctx: &egui::Context) -> PromptDialogResult {
        let mut result = PromptDialogResult::None;

        if !self.visible {
            return result;
        }

        egui::Window::new(&self.title)
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.label(&self.prompt);
                
                if !self.context.is_empty() {
                    ui.separator();
                    ui.group(|ui| {
                        ui.label(RichText::new("上下文信息:").small());
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.label(&self.context);
                            });
                    });
                }

                ui.separator();

                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .hint_text("在此输入...")
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ 确认").clicked() {
                        result = PromptDialogResult::Confirm(self.input.clone());
                        self.visible = false;
                    }
                    if ui.button("✗ 取消").clicked() {
                        result = PromptDialogResult::Cancel;
                        self.visible = false;
                    }
                });
            });

        result
    }
}

/// 提示词对话框结果
#[derive(Debug)]
pub enum PromptDialogResult {
    None,
    Confirm(String),
    Cancel,
}

/// 规则确认对话框
pub struct RuleConfirmDialog {
    /// 是否显示
    pub visible: bool,
    /// 规则名称
    pub rule_name: String,
    /// 条件描述
    pub condition_desc: String,
    /// 目标路径
    pub target_path: String,
    /// 预估影响文件数
    pub affected_count: usize,
}

impl Default for RuleConfirmDialog {
    fn default() -> Self {
        Self {
            visible: false,
            rule_name: String::new(),
            condition_desc: String::new(),
            target_path: String::new(),
            affected_count: 0,
        }
    }
}

impl RuleConfirmDialog {
    /// 显示对话框
    pub fn show(&mut self, name: &str, condition: &str, target: &str, count: usize) {
        self.visible = true;
        self.rule_name = name.to_string();
        self.condition_desc = condition.to_string();
        self.target_path = target.to_string();
        self.affected_count = count;
    }

    /// 渲染对话框
    pub fn render(&mut self, ctx: &egui::Context) -> RuleConfirmResult {
        let mut result = RuleConfirmResult::None;

        if !self.visible {
            return result;
        }

        egui::Window::new("确认新规则")
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading(&self.rule_name);
                
                ui.separator();

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("匹配条件:");
                        ui.label(&self.condition_desc);
                    });

                    ui.horizontal(|ui| {
                        ui.label("目标路径:");
                        ui.label(&self.target_path);
                    });

                    ui.horizontal(|ui| {
                        ui.label("预估影响:");
                        ui.label(
                            RichText::new(format!("{} 个文件", self.affected_count))
                                .color(egui::Color32::YELLOW)
                        );
                    });
                });

                ui.separator();

                ui.label(
                    RichText::new("⚠️ 该规则将在未来自动生效")
                        .color(egui::Color32::YELLOW)
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ 接受并保存").clicked() {
                        result = RuleConfirmResult::Accept;
                        self.visible = false;
                    }
                    if ui.button("⏱️ 仅本次应用").clicked() {
                        result = RuleConfirmResult::ApplyOnce;
                        self.visible = false;
                    }
                    if ui.button("✗ 取消").clicked() {
                        result = RuleConfirmResult::Cancel;
                        self.visible = false;
                    }
                });
            });

        result
    }
}

/// 规则确认结果
#[derive(Debug)]
pub enum RuleConfirmResult {
    None,
    Accept,
    ApplyOnce,
    Cancel,
}

/// 执行确认对话框
pub struct ExecuteConfirmDialog {
    /// 是否显示
    pub visible: bool,
    /// 操作数量
    pub operation_count: usize,
    /// 总文件大小
    pub total_size: String,
    /// 目标目录数
    pub target_dirs: usize,
    /// 潜在问题
    pub warnings: Vec<String>,
}

impl Default for ExecuteConfirmDialog {
    fn default() -> Self {
        Self {
            visible: false,
            operation_count: 0,
            total_size: String::new(),
            target_dirs: 0,
            warnings: Vec::new(),
        }
    }
}

impl ExecuteConfirmDialog {
    /// 显示对话框
    pub fn show(&mut self, ops: usize, size: String, dirs: usize, warnings: Vec<String>) {
        self.visible = true;
        self.operation_count = ops;
        self.total_size = size;
        self.target_dirs = dirs;
        self.warnings = warnings;
    }

    /// 渲染对话框
    pub fn render(&mut self, ctx: &egui::Context) -> ExecuteConfirmResult {
        let mut result = ExecuteConfirmResult::None;

        if !self.visible {
            return result;
        }

        egui::Window::new("确认执行")
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("即将执行以下操作");
                
                ui.separator();

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("移动文件数:");
                        ui.label(
                            RichText::new(format!("{}", self.operation_count))
                                .strong()
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("总大小:");
                        ui.label(&self.total_size);
                    });

                    ui.horizontal(|ui| {
                        ui.label("目标目录:");
                        ui.label(format!("{} 个", self.target_dirs));
                    });
                });

                if !self.warnings.is_empty() {
                    ui.separator();
                    ui.label(
                        RichText::new("⚠️ 警告")
                            .color(egui::Color32::YELLOW)
                    );
                    for warning in &self.warnings {
                        ui.label(format!("• {}", warning));
                    }
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ 执行").clicked() {
                        result = ExecuteConfirmResult::Execute;
                        self.visible = false;
                    }
                    if ui.button("✗ 取消").clicked() {
                        result = ExecuteConfirmResult::Cancel;
                        self.visible = false;
                    }
                });
            });

        result
    }
}

/// 执行确认结果
#[derive(Debug)]
pub enum ExecuteConfirmResult {
    None,
    Execute,
    Cancel,
}

/// 错误聚类提示对话框
pub struct ErrorClusterDialog {
    /// 是否显示
    pub visible: bool,
    /// 检测到的问题描述
    pub description: String,
    /// 相关文件
    pub related_files: Vec<String>,
}

impl Default for ErrorClusterDialog {
    fn default() -> Self {
        Self {
            visible: false,
            description: String::new(),
            related_files: Vec::new(),
        }
    }
}

impl ErrorClusterDialog {
    /// 显示对话框
    pub fn show(&mut self, desc: &str, files: Vec<String>) {
        self.visible = true;
        self.description = desc.to_string();
        self.related_files = files;
    }

    /// 渲染对话框
    pub fn render(&mut self, ctx: &egui::Context) -> ErrorClusterResult {
        let mut result = ErrorClusterResult::None;

        if !self.visible {
            return result;
        }

        egui::Window::new("检测到分类问题")
            .collapsible(false)
            .resizable(true)
            .default_width(450.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("🔍 检测到分类逻辑可能不符合您的习惯")
                        .color(egui::Color32::YELLOW)
                );
                
                ui.separator();

                ui.label(&self.description);

                if !self.related_files.is_empty() {
                    ui.separator();
                    ui.label("相关文件:");
                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            for file in &self.related_files {
                                ui.label(format!("• {}", file));
                            }
                        });
                }

                ui.separator();

                ui.label("您可以通过自然语言修正规则，例如：");
                ui.label(
                    RichText::new("\"以后运营商账单放到 Bills 目录下\"")
                        .italics()
                        .color(egui::Color32::LIGHT_BLUE)
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✏️ 写提示词修正").clicked() {
                        result = ErrorClusterResult::WritePrompt;
                        self.visible = false;
                    }
                    if ui.button("🚫 忽略").clicked() {
                        result = ErrorClusterResult::Ignore;
                        self.visible = false;
                    }
                });
            });

        result
    }
}

/// 错误聚类对话框结果
#[derive(Debug)]
pub enum ErrorClusterResult {
    None,
    WritePrompt,
    Ignore,
}

/// 设置对话框
pub struct SettingsDialog {
    /// 是否显示
    pub visible: bool,
    /// API 接口类型
    pub api_kind: ApiInterfaceKind,
    /// URL 后缀模式
    pub suffix_mode: UrlSuffixMode,
    /// API 基地址（Custom 模式下为完整 URL）
    pub api_base_url: String,
    /// 自定义后缀（仅 suffix_mode=Custom 且 api_kind!=Custom 时生效）
    pub custom_suffix: String,
    /// AI密钥
    pub ai_key: String,
    /// 模型名称
    pub model_name: String,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 是否启用AI
    pub ai_enabled: bool,
    /// 默认扫描路径
    pub default_scan_path: String,
    /// 默认输出路径
    pub default_output_path: String,
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self {
            visible: false,
            api_kind: ApiInterfaceKind::Ollama,
            suffix_mode: UrlSuffixMode::Standard,
            api_base_url: "http://localhost:11434".to_string(),
            custom_suffix: "/api/generate".to_string(),
            ai_key: String::new(),
            model_name: "qwen3:30b-a3b".to_string(),
            confidence_threshold: 0.7,
            ai_enabled: true,
            default_scan_path: String::new(),
            default_output_path: String::new(),
        }
    }
}

impl SettingsDialog {
    pub fn load_from_config(&mut self, config: &crate::core::models::AppConfig) {
        self.ai_enabled = config.ai_enabled;
        self.ai_key = config.ai_config.api_key.clone();
        self.model_name = config.ai_config.model_name.clone();
        self.confidence_threshold = config.confidence_threshold;

        if let Some(ref p) = config.default_scan_path {
            self.default_scan_path = p.to_string_lossy().to_string();
        }
        if let Some(ref p) = config.default_output_base {
            self.default_output_path = p.to_string_lossy().to_string();
        }

        let (kind, base, suffix_mode, custom_suffix) = Self::split_endpoint(&config.ai_config.api_endpoint);
        self.api_kind = kind;
        self.api_base_url = base;
        self.suffix_mode = suffix_mode;
        self.custom_suffix = custom_suffix;
    }

    pub fn effective_endpoint(&self) -> String {
        let base = self.api_base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return String::new();
        }

        if self.api_kind == ApiInterfaceKind::Custom {
            return base.to_string();
        }

        let suffix = match self.suffix_mode {
            UrlSuffixMode::Standard => self.api_kind.standard_suffix().unwrap_or(""),
            UrlSuffixMode::Custom => self.custom_suffix.trim(),
        };

        if suffix.is_empty() {
            return base.to_string();
        }

        if suffix.starts_with('/') {
            format!("{}{}", base, suffix)
        } else {
            format!("{}/{}", base, suffix)
        }
    }

    fn split_endpoint(endpoint: &str) -> (ApiInterfaceKind, String, UrlSuffixMode, String) {
        let e = endpoint.trim().trim_end_matches('/');
        if e.is_empty() {
            return (
                ApiInterfaceKind::Ollama,
                "http://localhost:11434".to_string(),
                UrlSuffixMode::Standard,
                "/api/generate".to_string(),
            );
        }

        // Ollama: 以 /api/ 作为分割点
        if e.contains("11434") || e.contains("ollama") {
            if let Some(idx) = e.find("/api/") {
                let base = e[..idx].to_string();
                let suffix = e[idx..].to_string();
                if suffix == "/api/generate" {
                    return (ApiInterfaceKind::Ollama, base, UrlSuffixMode::Standard, suffix);
                }
                return (ApiInterfaceKind::Ollama, base, UrlSuffixMode::Custom, suffix);
            }
            return (
                ApiInterfaceKind::Ollama,
                e.to_string(),
                UrlSuffixMode::Standard,
                "/api/generate".to_string(),
            );
        }

        // OpenAI / OpenAI-compatible: 以 /v1/ 分割
        if let Some(idx) = e.find("/v1/") {
            let base = e[..idx].to_string();
            let suffix = e[idx..].to_string();
            if suffix.starts_with("/v1/chat/completions") {
                let mode = if suffix == "/v1/chat/completions" {
                    UrlSuffixMode::Standard
                } else {
                    UrlSuffixMode::Custom
                };
                return (ApiInterfaceKind::OpenAIChatCompletions, base, mode, suffix);
            }
            if suffix.starts_with("/v1/responses") {
                let mode = if suffix == "/v1/responses" {
                    UrlSuffixMode::Standard
                } else {
                    UrlSuffixMode::Custom
                };
                return (ApiInterfaceKind::OpenAIResponses, base, mode, suffix);
            }
            return (ApiInterfaceKind::OpenAIChatCompletions, base, UrlSuffixMode::Custom, suffix);
        }

        // 兼容只填到 /chat/completions 的情况
        if let Some(idx) = e.find("/chat/completions") {
            let base = e[..idx].to_string();
            let suffix = e[idx..].to_string();
            return (ApiInterfaceKind::OpenAIChatCompletions, base, UrlSuffixMode::Custom, suffix);
        }

        (ApiInterfaceKind::Custom, e.to_string(), UrlSuffixMode::Custom, String::new())
    }

    /// 渲染对话框
    pub fn render(&mut self, ctx: &egui::Context) -> SettingsResult {
        let mut result = SettingsResult::None;

        if !self.visible {
            return result;
        }

        egui::Window::new("⚙️ 设置")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("AI 配置");
                
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ai_enabled, "启用 AI 分类");
                });

                ui.horizontal(|ui| {
                    ui.label("API 接口:");
                    egui::ComboBox::from_id_salt("api_kind")
                        .selected_text(self.api_kind.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.api_kind, ApiInterfaceKind::Ollama, ApiInterfaceKind::Ollama.label());
                            ui.selectable_value(&mut self.api_kind, ApiInterfaceKind::OpenAIChatCompletions, ApiInterfaceKind::OpenAIChatCompletions.label());
                            ui.selectable_value(&mut self.api_kind, ApiInterfaceKind::OpenAIResponses, ApiInterfaceKind::OpenAIResponses.label());
                            ui.selectable_value(&mut self.api_kind, ApiInterfaceKind::Custom, ApiInterfaceKind::Custom.label());
                        });
                });

                if self.api_kind == ApiInterfaceKind::Custom {
                    ui.horizontal(|ui| {
                        ui.label("API 端点(完整URL):");
                        ui.text_edit_singleline(&mut self.api_base_url);
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("API 基地址:");
                        ui.text_edit_singleline(&mut self.api_base_url);
                    });

                    ui.horizontal(|ui| {
                        ui.label("URL 后缀:");
                        ui.radio_value(&mut self.suffix_mode, UrlSuffixMode::Standard, "标准后缀");
                        ui.radio_value(&mut self.suffix_mode, UrlSuffixMode::Custom, "自定义后缀");
                    });

                    if self.suffix_mode == UrlSuffixMode::Custom {
                        ui.horizontal(|ui| {
                            ui.label("自定义后缀:");
                            ui.text_edit_singleline(&mut self.custom_suffix);
                        });
                    } else if let Some(s) = self.api_kind.standard_suffix() {
                        ui.horizontal(|ui| {
                            ui.label("标准后缀:");
                            ui.label(s);
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("最终请求URL:");
                        ui.label(self.effective_endpoint());
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("API 密钥:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ai_key)
                            .password(true)
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("模型名称:");
                    ui.text_edit_singleline(&mut self.model_name);
                });

                ui.horizontal(|ui| {
                    ui.label("置信度阈值:");
                    ui.add(egui::Slider::new(&mut self.confidence_threshold, 0.0..=1.0));
                });

                ui.separator();
                ui.heading("默认路径");

                ui.horizontal(|ui| {
                    ui.label("扫描路径:");
                    ui.text_edit_singleline(&mut self.default_scan_path);
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.default_scan_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("输出路径:");
                    ui.text_edit_singleline(&mut self.default_output_path);
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.default_output_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("💾 保存").clicked() {
                        result = SettingsResult::Save;
                        self.visible = false;
                    }
                    if ui.button("✗ 取消").clicked() {
                        result = SettingsResult::Cancel;
                        self.visible = false;
                    }
                });
            });

        result
    }
}

/// 设置对话框结果
#[derive(Debug)]
pub enum SettingsResult {
    None,
    Save,
    Cancel,
}
