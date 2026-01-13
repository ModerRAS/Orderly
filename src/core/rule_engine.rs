//! 规则引擎模块
//! 
//! 负责规则的存储、加载、匹配和优先级排序。
//! 规则是用户确认后沉淀的分类逻辑，优先于AI判断。

use crate::core::models::{
    FileDescriptor, MoveSuggestion, RuleAction, RuleCondition, RuleDefinition, 
    RuleOrigin, SuggestionSource,
};
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;

/// 规则引擎
pub struct RuleEngine {
    /// 规则列表（按优先级排序）
    rules: Vec<RuleDefinition>,
    /// 输出基础路径
    output_base: PathBuf,
}

impl RuleEngine {
    /// 创建新的规则引擎
    pub fn new(output_base: PathBuf) -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            output_base,
        };
        
        // 加载内置规则
        engine.load_builtin_rules();
        engine
    }

    /// 加载内置规则
    fn load_builtin_rules(&mut self) {
        let builtin_rules = vec![
            // 图片文件规则
            RuleDefinition {
                id: "builtin_images".to_string(),
                name: "图片文件".to_string(),
                priority: 30,
                enabled: true,
                condition: RuleCondition {
                    file_extensions: vec![
                        ".jpg".to_string(),
                        ".jpeg".to_string(),
                        ".png".to_string(),
                        ".gif".to_string(),
                        ".bmp".to_string(),
                        ".webp".to_string(),
                        ".svg".to_string(),
                        ".ico".to_string(),
                        ".heic".to_string(),
                        ".heif".to_string(),
                    ],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Pictures/{year}/{month}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
            // 视频文件规则
            RuleDefinition {
                id: "builtin_videos".to_string(),
                name: "视频文件".to_string(),
                priority: 30,
                enabled: true,
                condition: RuleCondition {
                    file_extensions: vec![
                        ".mp4".to_string(),
                        ".avi".to_string(),
                        ".mkv".to_string(),
                        ".mov".to_string(),
                        ".wmv".to_string(),
                        ".flv".to_string(),
                        ".webm".to_string(),
                        ".m4v".to_string(),
                    ],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Videos/{year}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
            // 音频文件规则
            RuleDefinition {
                id: "builtin_audio".to_string(),
                name: "音频文件".to_string(),
                priority: 30,
                enabled: true,
                condition: RuleCondition {
                    file_extensions: vec![
                        ".mp3".to_string(),
                        ".wav".to_string(),
                        ".flac".to_string(),
                        ".aac".to_string(),
                        ".ogg".to_string(),
                        ".wma".to_string(),
                        ".m4a".to_string(),
                    ],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Music/{year}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
            // 文档文件规则
            RuleDefinition {
                id: "builtin_documents".to_string(),
                name: "文档文件".to_string(),
                priority: 30,
                enabled: true,
                condition: RuleCondition {
                    file_extensions: vec![
                        ".doc".to_string(),
                        ".docx".to_string(),
                        ".xls".to_string(),
                        ".xlsx".to_string(),
                        ".ppt".to_string(),
                        ".pptx".to_string(),
                        ".pdf".to_string(),
                        ".txt".to_string(),
                        ".md".to_string(),
                        ".rtf".to_string(),
                        ".odt".to_string(),
                        ".ods".to_string(),
                        ".odp".to_string(),
                    ],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Documents/{year}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
            // 压缩文件规则
            RuleDefinition {
                id: "builtin_archives".to_string(),
                name: "压缩文件".to_string(),
                priority: 30,
                enabled: true,
                condition: RuleCondition {
                    file_extensions: vec![
                        ".zip".to_string(),
                        ".rar".to_string(),
                        ".7z".to_string(),
                        ".tar".to_string(),
                        ".gz".to_string(),
                        ".bz2".to_string(),
                        ".xz".to_string(),
                    ],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Archives/{year}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
            // 发票/账单规则
            RuleDefinition {
                id: "builtin_invoice".to_string(),
                name: "发票/账单".to_string(),
                priority: 60,
                enabled: true,
                condition: RuleCondition {
                    filename_keywords: vec![
                        "发票".to_string(),
                        "invoice".to_string(),
                        "账单".to_string(),
                        "bill".to_string(),
                        "收据".to_string(),
                        "receipt".to_string(),
                    ],
                    file_extensions: vec![".pdf".to_string(), ".jpg".to_string(), ".png".to_string()],
                    ..Default::default()
                },
                action: RuleAction {
                    move_to: "Finance/Invoice/{year}".to_string(),
                },
                origin: RuleOrigin::BuiltIn,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                hit_count: 0,
            },
        ];

        self.rules.extend(builtin_rules);
        self.sort_rules();
    }

    /// 按优先级排序规则
    fn sort_rules(&mut self) {
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 添加新规则
    pub fn add_rule(&mut self, rule: RuleDefinition) {
        self.rules.push(rule);
        self.sort_rules();
    }

    /// 删除规则
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        if let Some(pos) = self.rules.iter().position(|r| r.id == rule_id) {
            self.rules.remove(pos);
            true
        } else {
            false
        }
    }

    /// 启用/禁用规则
    pub fn set_rule_enabled(&mut self, rule_id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            rule.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// 获取所有规则
    pub fn get_rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    /// 获取可变规则引用
    pub fn get_rules_mut(&mut self) -> &mut Vec<RuleDefinition> {
        &mut self.rules
    }

    /// 为文件匹配规则
    pub fn match_file(&mut self, file: &FileDescriptor) -> Option<MoveSuggestion> {
        // 原子文件不参与规则匹配
        if file.atomic {
            return None;
        }

        // 目录暂不处理
        if file.is_directory {
            return None;
        }

        // 按优先级顺序匹配规则
        for rule in self.rules.iter_mut() {
            if !rule.enabled {
                continue;
            }

            if rule.condition.matches(file) {
                // 更新命中计数
                rule.hit_count += 1;
                rule.updated_at = Utc::now();

                let target_path = rule.action.render_path(file, &self.output_base);
                
                return Some(MoveSuggestion {
                    target_path,
                    reason: format!("匹配规则: {}", rule.name),
                    source: SuggestionSource::Rule,
                    confidence: 0.9, // 规则匹配的置信度固定为0.9
                });
            }
        }

        None
    }

    /// 批量匹配文件
    pub fn match_files(&mut self, files: &mut [FileDescriptor]) {
        for file in files.iter_mut() {
            if let Some(suggestion) = self.match_file(file) {
                file.suggested_action = Some(suggestion);
            }
        }
    }

    /// 从JSON加载规则
    pub fn load_from_json(&mut self, json_str: &str) -> Result<()> {
        let rules: Vec<RuleDefinition> = serde_json::from_str(json_str)?;
        
        // 只加载用户规则，保留内置规则
        for rule in rules {
            if rule.origin == RuleOrigin::UserConfirmed {
                self.add_rule(rule);
            }
        }
        
        Ok(())
    }

    /// 导出用户规则为JSON
    pub fn export_user_rules_to_json(&self) -> Result<String> {
        let user_rules: Vec<_> = self.rules
            .iter()
            .filter(|r| r.origin == RuleOrigin::UserConfirmed)
            .collect();
        
        Ok(serde_json::to_string_pretty(&user_rules)?)
    }

    /// 设置输出基础路径
    pub fn set_output_base(&mut self, path: PathBuf) {
        self.output_base = path;
    }

    /// 获取输出基础路径
    pub fn get_output_base(&self) -> &PathBuf {
        &self.output_base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_rule_matching() {
        let mut engine = RuleEngine::new(PathBuf::from("/output"));
        
        let file = FileDescriptor::new(
            PathBuf::from("/test/photo.jpg"),
            "photo.jpg".to_string(),
            ".jpg".to_string(),
            1024,
            Utc::now(),
            false,
        );

        let suggestion = engine.match_file(&file);
        assert!(suggestion.is_some());
        
        let suggestion = suggestion.unwrap();
        assert!(suggestion.target_path.to_string_lossy().contains("Pictures"));
    }

    #[test]
    fn test_invoice_rule_priority() {
        let mut engine = RuleEngine::new(PathBuf::from("/output"));
        
        let file = FileDescriptor::new(
            PathBuf::from("/test/发票_2023.pdf"),
            "发票_2023.pdf".to_string(),
            ".pdf".to_string(),
            1024,
            Utc::now(),
            false,
        );

        let suggestion = engine.match_file(&file);
        assert!(suggestion.is_some());
        
        let suggestion = suggestion.unwrap();
        // 发票规则优先级更高，应该匹配发票规则
        assert!(suggestion.target_path.to_string_lossy().contains("Finance"));
    }
}
