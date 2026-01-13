//! 移动计划生成器模块
//! 
//! 负责整合规则引擎和AI分析结果，生成最终的移动计划。

use crate::core::models::{FileDescriptor, MovePlan, MoveSuggestion, SuggestionSource};
use std::path::PathBuf;

/// 移动计划生成器
pub struct Planner {
    /// 输出基础路径
    output_base: PathBuf,
    /// 置信度阈值
    confidence_threshold: f32,
}

impl Planner {
    /// 创建新的计划生成器
    pub fn new(output_base: PathBuf, confidence_threshold: f32) -> Self {
        Self {
            output_base,
            confidence_threshold,
        }
    }

    /// 设置输出基础路径
    pub fn set_output_base(&mut self, path: PathBuf) {
        self.output_base = path;
    }

    /// 生成移动计划
    pub fn generate_plan(&self, files: &[FileDescriptor]) -> MovePlan {
        let mut plan = MovePlan::new();

        for file in files {
            // 跳过未选中的文件
            if !file.selected {
                continue;
            }

            // 跳过没有建议的文件
            let suggestion = match &file.suggested_action {
                Some(s) => s,
                None => continue,
            };

            // 跳过原子文件（除非是原子目录整体移动）
            if file.atomic && !file.is_directory {
                continue;
            }

            // 跳过低置信度的建议
            if suggestion.confidence < self.confidence_threshold {
                continue;
            }

            // 规则/AI 的 target_path 通常是“目录”，这里必须拼上文件名。
            // 否则会把最后一级目录名当作目标文件名，导致扩展名丢失、文件被“改名”。
            let mut target = suggestion.target_path.clone();
            let target_leaf = target
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let looks_like_full_file_path = target_leaf == file.name || target_leaf.contains('.');
            if !looks_like_full_file_path {
                target = target.join(&file.name);
            }

            plan.add_operation(
                file.full_path.clone(),
                target,
                file.id.clone(),
            );
        }

        plan
    }

    /// 融合规则和AI建议
    /// 
    /// 置信度融合公式：
    /// - rule_score × 0.6 + ai_score × 0.4
    /// - uncertainty 作为降权因子
    pub fn fuse_suggestions(
        &self,
        rule_suggestion: Option<&MoveSuggestion>,
        ai_suggestion: Option<&MoveSuggestion>,
    ) -> Option<MoveSuggestion> {
        match (rule_suggestion, ai_suggestion) {
            (Some(rule), Some(ai)) => {
                // 两者都有，进行融合
                let rule_score = rule.confidence * 0.6;
                let ai_score = ai.confidence * 0.4;
                let fused_confidence = rule_score + ai_score;

                // 如果路径相同，提高置信度
                if rule.target_path == ai.target_path {
                    Some(MoveSuggestion {
                        target_path: rule.target_path.clone(),
                        reason: format!("规则+AI一致: {} | {}", rule.reason, ai.reason),
                        source: SuggestionSource::Rule,
                        confidence: (fused_confidence * 1.1).min(1.0),
                    })
                } else {
                    // 路径不同，选择置信度更高的
                    if rule_score >= ai_score {
                        Some(MoveSuggestion {
                            target_path: rule.target_path.clone(),
                            reason: format!("规则优先: {}", rule.reason),
                            source: SuggestionSource::Rule,
                            confidence: fused_confidence,
                        })
                    } else {
                        Some(MoveSuggestion {
                            target_path: ai.target_path.clone(),
                            reason: format!("AI建议: {}", ai.reason),
                            source: SuggestionSource::AI,
                            confidence: fused_confidence,
                        })
                    }
                }
            }
            (Some(rule), None) => Some(rule.clone()),
            (None, Some(ai)) => Some(ai.clone()),
            (None, None) => None,
        }
    }

    /// 验证移动计划
    pub fn validate_plan(&self, plan: &MovePlan) -> Vec<PlanValidationError> {
        let mut errors = Vec::new();

        for (i, op) in plan.operations.iter().enumerate() {
            // 检查源文件是否存在
            if !op.from.exists() {
                errors.push(PlanValidationError {
                    operation_index: i,
                    error_type: ValidationErrorType::SourceNotFound,
                    message: format!("源文件不存在: {}", op.from.display()),
                });
            }

            // 检查目标路径是否在源路径内（避免循环）
            if op.to.starts_with(&op.from) {
                errors.push(PlanValidationError {
                    operation_index: i,
                    error_type: ValidationErrorType::CircularPath,
                    message: format!("目标路径在源路径内: {} -> {}", op.from.display(), op.to.display()),
                });
            }

            // 检查是否有冲突（多个文件移动到同一位置）
            for (j, other_op) in plan.operations.iter().enumerate() {
                if i != j && op.to == other_op.to {
                    errors.push(PlanValidationError {
                        operation_index: i,
                        error_type: ValidationErrorType::TargetConflict,
                        message: format!(
                            "目标冲突: {} 和 {} 都要移动到 {}",
                            op.from.display(),
                            other_op.from.display(),
                            op.to.display()
                        ),
                    });
                }
            }
        }

        errors
    }

    /// 获取计划统计信息
    pub fn get_plan_stats(&self, plan: &MovePlan) -> PlanStats {
        let total_operations = plan.operations.len();
        
        let mut total_size: u64 = 0;
        let mut target_dirs = std::collections::HashSet::new();

        for op in &plan.operations {
            if let Ok(metadata) = std::fs::metadata(&op.from) {
                total_size += metadata.len();
            }
            if let Some(parent) = op.to.parent() {
                target_dirs.insert(parent.to_path_buf());
            }
        }

        PlanStats {
            total_operations,
            total_size,
            target_directories: target_dirs.len(),
        }
    }
}

/// 计划验证错误
#[derive(Debug)]
pub struct PlanValidationError {
    /// 操作索引
    pub operation_index: usize,
    /// 错误类型
    pub error_type: ValidationErrorType,
    /// 错误消息
    pub message: String,
}

/// 验证错误类型
#[derive(Debug)]
pub enum ValidationErrorType {
    /// 源文件不存在
    SourceNotFound,
    /// 循环路径
    CircularPath,
    /// 目标冲突
    TargetConflict,
    /// 权限不足
    PermissionDenied,
}

/// 计划统计信息
#[derive(Debug)]
pub struct PlanStats {
    /// 总操作数
    pub total_operations: usize,
    /// 总文件大小（字节）
    pub total_size: u64,
    /// 目标目录数
    pub target_directories: usize,
}

impl PlanStats {
    /// 格式化文件大小
    pub fn format_size(&self) -> String {
        let size = self.total_size as f64;
        if size < 1024.0 {
            format!("{} B", self.total_size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.2} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_fuse_suggestions() {
        let planner = Planner::new(PathBuf::from("/output"), 0.5);

        let rule = MoveSuggestion {
            target_path: PathBuf::from("/output/Documents"),
            reason: "规则匹配".to_string(),
            source: SuggestionSource::Rule,
            confidence: 0.9,
        };

        let ai = MoveSuggestion {
            target_path: PathBuf::from("/output/Documents"),
            reason: "AI建议".to_string(),
            source: SuggestionSource::AI,
            confidence: 0.8,
        };

        let fused = planner.fuse_suggestions(Some(&rule), Some(&ai));
        assert!(fused.is_some());
        
        let fused = fused.unwrap();
        // 路径相同应该提高置信度
        assert!(fused.confidence > 0.9);
    }
}
