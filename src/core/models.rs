//! 核心数据模型定义
//! 
//! 所有数据结构必须严格遵守设计文档定义，不允许自行添加未定义的字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 目录类型枚举
/// 用于标识目录的性质，决定是否可以拆分处理
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DirectoryType {
    /// 普通目录，可以自由操作其中的文件
    #[default]
    Normal,
    /// 程序根目录（包含.exe, .dll等）
    ProgramRoot,
    /// 虚拟环境目录（Python venv, node_modules等）
    VirtualEnv,
    /// 包仓库目录（npm cache, pip cache等）
    PackageRepo,
    /// 系统目录（Windows, Program Files等）
    System,
}

impl DirectoryType {
    /// 判断此类型目录是否为原子目录（不可拆分）
    pub fn is_atomic(&self) -> bool {
        matches!(
            self,
            DirectoryType::ProgramRoot
                | DirectoryType::VirtualEnv
                | DirectoryType::PackageRepo
                | DirectoryType::System
        )
    }
}

/// 文件描述符 - 核心数据结构
/// 描述一个文件或目录的完整信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDescriptor {
    /// 稳定唯一标识符（基于路径的hash）
    pub id: String,
    /// 文件/目录名称
    pub name: String,
    /// 文件扩展名（目录为空）
    pub extension: String,
    /// 完整路径
    pub full_path: PathBuf,
    /// 父目录路径
    pub parent_dir: PathBuf,
    /// 文件大小（字节），目录为0或子文件总大小
    pub size: u64,
    /// 最后修改时间
    pub modified_at: DateTime<Utc>,
    /// 是否为目录
    pub is_directory: bool,
    /// 目录类型
    pub directory_type: DirectoryType,
    /// 是否为原子项（不可拆分）
    pub atomic: bool,
    /// AI语义分析结果（可选）
    pub semantic: Option<SemanticResult>,
    /// 建议的移动操作（可选）
    pub suggested_action: Option<MoveSuggestion>,
    /// 用户是否选中此项进行操作
    pub selected: bool,
}

impl FileDescriptor {
    /// 创建新的文件描述符
    pub fn new(
        full_path: PathBuf,
        name: String,
        extension: String,
        size: u64,
        modified_at: DateTime<Utc>,
        is_directory: bool,
    ) -> Self {
        use sha2::{Digest, Sha256};
        
        let parent_dir = full_path.parent().unwrap_or(&full_path).to_path_buf();
        
        // 生成稳定ID
        let mut hasher = Sha256::new();
        hasher.update(full_path.to_string_lossy().as_bytes());
        let id = hex::encode(&hasher.finalize()[..16]);

        Self {
            id,
            name,
            extension,
            full_path,
            parent_dir,
            size,
            modified_at,
            is_directory,
            directory_type: DirectoryType::Normal,
            atomic: false,
            semantic: None,
            suggested_action: None,
            selected: true, // 默认选中
        }
    }
}

/// AI语义分析结果
/// AI输出必须严格遵循此结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticResult {
    /// 语义标签列表（如 ["invoice", "telecom", "2023"]）
    pub tags: Vec<String>,
    /// 识别出的实体（如公司名、人名等）
    pub entities: Vec<String>,
    /// 识别出的年份
    pub year: Option<i32>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// AI给出的解释
    pub explanation: String,
}

impl Default for SemanticResult {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            entities: Vec::new(),
            year: None,
            confidence: 0.0,
            explanation: String::new(),
        }
    }
}

/// 移动建议
/// 描述AI或规则引擎给出的文件移动建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveSuggestion {
    /// 建议的目标路径
    pub target_path: PathBuf,
    /// 建议理由
    pub reason: String,
    /// 建议来源
    pub source: SuggestionSource,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
}

/// 建议来源枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionSource {
    /// AI语义分析
    AI,
    /// 规则引擎
    Rule,
    /// 历史记忆
    Memory,
}

impl std::fmt::Display for SuggestionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuggestionSource::AI => write!(f, "AI"),
            SuggestionSource::Rule => write!(f, "规则"),
            SuggestionSource::Memory => write!(f, "记忆"),
        }
    }
}

/// 规则定义
/// 用户确认后沉淀的分类规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    /// 规则唯一ID
    pub id: String,
    /// 规则名称（用于显示）
    pub name: String,
    /// 优先级（数字越大优先级越高，0-100）
    pub priority: u8,
    /// 是否启用
    pub enabled: bool,
    /// 匹配条件
    pub condition: RuleCondition,
    /// 执行动作
    pub action: RuleAction,
    /// 规则来源
    pub origin: RuleOrigin,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后修改时间
    pub updated_at: DateTime<Utc>,
    /// 命中次数（统计用）
    pub hit_count: u64,
}

impl RuleDefinition {
    /// 创建新规则
    pub fn new(name: String, condition: RuleCondition, action: RuleAction) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            id,
            name,
            priority: 50,
            enabled: true,
            condition,
            action,
            origin: RuleOrigin::UserConfirmed,
            created_at: now,
            updated_at: now,
            hit_count: 0,
        }
    }
}

/// 规则匹配条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleCondition {
    /// 需要匹配的语义标签（任一匹配即可）
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    /// 需要匹配的文件扩展名（任一匹配即可）
    #[serde(default)]
    pub file_extensions: Vec<String>,
    /// 需要匹配的文件名关键词（任一包含即可）
    #[serde(default)]
    pub filename_keywords: Vec<String>,
    /// 排除的目录路径模式
    #[serde(default)]
    pub directory_excludes: Vec<String>,
    /// 最小文件大小（字节）
    pub min_size: Option<u64>,
    /// 最大文件大小（字节）
    pub max_size: Option<u64>,
}

impl RuleCondition {
    /// 检查文件是否匹配此条件
    pub fn matches(&self, file: &FileDescriptor) -> bool {
        // 检查扩展名
        if !self.file_extensions.is_empty() {
            let ext_lower = file.extension.to_lowercase();
            if !self.file_extensions.iter().any(|e| e.to_lowercase() == ext_lower) {
                return false;
            }
        }

        // 检查文件名关键词
        if !self.filename_keywords.is_empty() {
            let name_lower = file.name.to_lowercase();
            if !self.filename_keywords.iter().any(|k| name_lower.contains(&k.to_lowercase())) {
                return false;
            }
        }

        // 检查语义标签
        if !self.semantic_tags.is_empty() {
            if let Some(ref semantic) = file.semantic {
                let has_match = self.semantic_tags.iter().any(|t| {
                    semantic.tags.iter().any(|st| st.to_lowercase() == t.to_lowercase())
                });
                if !has_match {
                    return false;
                }
            } else {
                return false;
            }
        }

        // 检查排除目录
        let path_str = file.full_path.to_string_lossy().to_lowercase();
        if self.directory_excludes.iter().any(|d| path_str.contains(&d.to_lowercase())) {
            return false;
        }

        // 检查文件大小
        if let Some(min) = self.min_size {
            if file.size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if file.size > max {
                return false;
            }
        }

        true
    }
}

/// 规则动作
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleAction {
    /// 目标路径模板，支持变量如 {year}, {extension}
    #[serde(default)]
    pub move_to: String,
}

impl RuleAction {
    /// 根据文件信息渲染实际目标路径
    pub fn render_path(&self, file: &FileDescriptor, base_path: &PathBuf) -> PathBuf {
        let mut path = self.move_to.clone();
        
        // 替换年份变量
        if let Some(ref semantic) = file.semantic {
            if let Some(year) = semantic.year {
                path = path.replace("{year}", &year.to_string());
            }
        }
        // 如果没有语义年份，尝试从修改时间获取
        if path.contains("{year}") {
            let year = file.modified_at.format("%Y").to_string();
            path = path.replace("{year}", &year);
        }
        
        // 替换扩展名变量
        let ext = file.extension.trim_start_matches('.');
        path = path.replace("{extension}", ext);
        
        // 替换月份变量
        let month = file.modified_at.format("%m").to_string();
        path = path.replace("{month}", &month);
        
        base_path.join(path)
    }
}

/// 规则来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleOrigin {
    /// 用户确认的规则
    UserConfirmed,
    /// 系统内置规则
    BuiltIn,
}

/// 移动计划 - 描述一批文件的移动操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePlan {
    /// 批次ID
    pub batch_id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 操作列表
    pub operations: Vec<MoveOperation>,
}

impl MovePlan {
    /// 创建新的移动计划
    pub fn new() -> Self {
        Self {
            batch_id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            operations: Vec::new(),
        }
    }
    
    /// 添加操作
    pub fn add_operation(&mut self, from: PathBuf, to: PathBuf, file_id: String) {
        self.operations.push(MoveOperation {
            from,
            to,
            file_id,
            status: OperationStatus::Pending,
            error: None,
        });
    }
}

impl Default for MovePlan {
    fn default() -> Self {
        Self::new()
    }
}

/// 单个移动操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOperation {
    /// 源路径
    pub from: PathBuf,
    /// 目标路径
    pub to: PathBuf,
    /// 文件ID
    pub file_id: String,
    /// 操作状态
    pub status: OperationStatus,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 操作状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// 待执行
    Pending,
    /// 执行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已跳过
    Skipped,
    /// 已回滚
    RolledBack,
}

/// 历史记录项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// 批次ID
    pub batch_id: String,
    /// 执行时间
    pub executed_at: DateTime<Utc>,
    /// 操作列表
    pub operations: Vec<MoveOperation>,
    /// 是否已回滚
    pub rolled_back: bool,
}

/// AI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// API端点URL
    pub api_endpoint: String,
    /// API密钥
    pub api_key: String,
    /// 模型名称
    pub model_name: String,
    /// 最大token数
    pub max_tokens: u32,
    /// 温度参数
    pub temperature: f32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            api_key: String::new(),
            model_name: "qwen3:30b-a3b".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
        }
    }
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 默认扫描路径
    pub default_scan_path: Option<PathBuf>,
    /// 默认输出基础路径
    pub default_output_base: Option<PathBuf>,
    /// AI配置
    pub ai_config: AIConfig,
    /// 是否启用AI分类
    pub ai_enabled: bool,
    /// 置信度阈值（低于此值需要人工确认）
    pub confidence_threshold: f32,
    /// 是否默认Dry Run模式
    pub dry_run_default: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_scan_path: None,
            default_output_base: None,
            ai_config: AIConfig::default(),
            ai_enabled: true,
            confidence_threshold: 0.7,
            dry_run_default: true,
        }
    }
}

/// 错误聚类信息
/// 用于检测用户频繁修改的分类模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCluster {
    /// 相关的语义标签
    pub semantic_tags: Vec<String>,
    /// 原始建议路径
    pub original_path: String,
    /// 用户修改后的路径
    pub corrected_path: String,
    /// 发生次数
    pub occurrence_count: u32,
    /// 最后发生时间
    pub last_occurrence: DateTime<Utc>,
}
