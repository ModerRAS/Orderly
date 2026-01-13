//! 执行器模块
//! 
//! 负责执行移动计划、记录历史、支持回滚。
//! 
//! 设计原则：
//! - 默认 Dry Run 模式
//! - 所有操作可回滚
//! - 详细记录每一步操作

use crate::core::models::{HistoryEntry, MoveOperation, MovePlan, OperationStatus};
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

/// 执行器
pub struct Executor {
    /// 历史记录
    history: Vec<HistoryEntry>,
    /// 历史文件路径
    history_file: PathBuf,
}

impl Executor {
    /// 创建新的执行器
    pub fn new(data_dir: PathBuf) -> Self {
        let history_file = data_dir.join("history.json");
        let history = Self::load_history(&history_file).unwrap_or_default();
        
        Self {
            history,
            history_file,
        }
    }

    /// 从文件加载历史记录
    fn load_history(path: &PathBuf) -> Result<Vec<HistoryEntry>> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Vec::new())
        }
    }

    /// 保存历史记录到文件
    fn save_history(&self) -> Result<()> {
        if let Some(parent) = self.history_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.history)?;
        fs::write(&self.history_file, content)?;
        Ok(())
    }

    /// Dry Run - 预览执行结果
    pub fn dry_run(&self, plan: &MovePlan) -> DryRunResult {
        let mut result = DryRunResult {
            would_create_dirs: Vec::new(),
            would_move_files: Vec::new(),
            potential_errors: Vec::new(),
        };

        let mut dirs_to_create = std::collections::HashSet::new();

        for op in &plan.operations {
            // 检查源文件
            if !op.from.exists() {
                result.potential_errors.push(format!(
                    "源文件不存在: {}",
                    op.from.display()
                ));
                continue;
            }

            // 检查目标目录
            if let Some(parent) = op.to.parent() {
                if !parent.exists() {
                    dirs_to_create.insert(parent.to_path_buf());
                }
            }

            // 检查目标文件是否已存在
            if op.to.exists() {
                result.potential_errors.push(format!(
                    "目标文件已存在: {}",
                    op.to.display()
                ));
            }

            result.would_move_files.push((op.from.clone(), op.to.clone()));
        }

        result.would_create_dirs = dirs_to_create.into_iter().collect();
        result
    }

    /// 执行移动计划
    pub fn execute(&mut self, plan: &mut MovePlan) -> ExecutionResult {
        let mut result = ExecutionResult {
            successful: 0,
            failed: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        for op in plan.operations.iter_mut() {
            op.status = OperationStatus::InProgress;

            match self.execute_single_operation(op) {
                Ok(()) => {
                    op.status = OperationStatus::Completed;
                    result.successful += 1;
                }
                Err(e) => {
                    op.status = OperationStatus::Failed;
                    op.error = Some(e.to_string());
                    result.failed += 1;
                    result.errors.push(format!(
                        "移动 {} 失败: {}",
                        op.from.display(),
                        e
                    ));
                }
            }
        }

        // 记录历史
        let entry = HistoryEntry {
            batch_id: plan.batch_id.clone(),
            executed_at: Utc::now(),
            operations: plan.operations.clone(),
            rolled_back: false,
        };
        self.history.push(entry);

        // 保存历史
        if let Err(e) = self.save_history() {
            tracing::warn!("保存历史记录失败: {}", e);
        }

        result
    }

    /// 执行单个移动操作
    fn execute_single_operation(&self, op: &MoveOperation) -> Result<()> {
        // 创建目标目录
        if let Some(parent) = op.to.parent() {
            fs::create_dir_all(parent)?;
        }

        // 检查目标是否已存在
        if op.to.exists() {
            return Err(anyhow::anyhow!("目标文件已存在"));
        }

        // 执行移动
        fs::rename(&op.from, &op.to)?;

        tracing::info!("已移动: {} -> {}", op.from.display(), op.to.display());
        Ok(())
    }

    /// 回滚指定批次的操作
    pub fn rollback(&mut self, batch_id: &str) -> RollbackResult {
        let mut result = RollbackResult {
            successful: 0,
            failed: 0,
            errors: Vec::new(),
        };

        // 查找历史记录索引
        let entry_idx = self.history.iter().position(|e| e.batch_id == batch_id);
        
        let entry_idx = match entry_idx {
            Some(idx) => idx,
            None => {
                result.errors.push(format!("未找到批次: {}", batch_id));
                return result;
            }
        };

        if self.history[entry_idx].rolled_back {
            result.errors.push("该批次已回滚".to_string());
            return result;
        }

        // 逆序回滚 - 先收集需要回滚的操作
        let ops_to_rollback: Vec<(usize, std::path::PathBuf, std::path::PathBuf)> = self.history[entry_idx]
            .operations
            .iter()
            .enumerate()
            .filter(|(_, op)| op.status == OperationStatus::Completed)
            .map(|(i, op)| (i, op.from.clone(), op.to.clone()))
            .collect();

        // 执行回滚
        for (op_idx, from, to) in ops_to_rollback.into_iter().rev() {
            match Self::rollback_operation_static(&from, &to) {
                Ok(()) => {
                    self.history[entry_idx].operations[op_idx].status = OperationStatus::RolledBack;
                    result.successful += 1;
                }
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!(
                        "回滚 {} 失败: {}",
                        to.display(),
                        e
                    ));
                }
            }
        }

        self.history[entry_idx].rolled_back = true;

        // 保存历史
        if let Err(e) = self.save_history() {
            tracing::warn!("保存历史记录失败: {}", e);
        }

        result
    }

    /// 静态回滚操作（避免借用冲突）
    fn rollback_operation_static(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
        // 检查新位置是否存在
        if !to.exists() {
            return Err(anyhow::anyhow!("新位置文件不存在"));
        }

        // 创建原始目录（如果需要）
        if let Some(parent) = from.parent() {
            fs::create_dir_all(parent)?;
        }

        // 移回原位置
        fs::rename(to, from)?;

        // 尝试清理空目录
        if let Some(parent) = to.parent() {
            let _ = fs::remove_dir(parent); // 忽略错误（目录可能不为空）
        }

        tracing::info!("已回滚: {} -> {}", to.display(), from.display());
        Ok(())
    }

    /// 回滚单个操作
    #[allow(dead_code)]
    fn rollback_single_operation(&self, op: &MoveOperation) -> Result<()> {
        Self::rollback_operation_static(&op.from, &op.to)
    }

    /// 获取历史记录
    pub fn get_history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// 获取最近的历史记录
    pub fn get_recent_history(&self, count: usize) -> Vec<&HistoryEntry> {
        self.history.iter().rev().take(count).collect()
    }

    /// 清理旧历史记录
    pub fn cleanup_old_history(&mut self, keep_count: usize) {
        if self.history.len() > keep_count {
            let remove_count = self.history.len() - keep_count;
            self.history.drain(0..remove_count);
            let _ = self.save_history();
        }
    }
}

/// Dry Run 结果
#[derive(Debug)]
pub struct DryRunResult {
    /// 将要创建的目录
    pub would_create_dirs: Vec<PathBuf>,
    /// 将要移动的文件 (源, 目标)
    pub would_move_files: Vec<(PathBuf, PathBuf)>,
    /// 潜在错误
    pub potential_errors: Vec<String>,
}

impl DryRunResult {
    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        !self.potential_errors.is_empty()
    }
    
    /// 获取摘要
    pub fn summary(&self) -> String {
        format!(
            "将创建 {} 个目录，移动 {} 个文件，{} 个潜在问题",
            self.would_create_dirs.len(),
            self.would_move_files.len(),
            self.potential_errors.len()
        )
    }
}

/// 执行结果
#[derive(Debug)]
pub struct ExecutionResult {
    /// 成功数量
    pub successful: usize,
    /// 失败数量
    pub failed: usize,
    /// 跳过数量
    pub skipped: usize,
    /// 错误信息
    pub errors: Vec<String>,
}

impl ExecutionResult {
    /// 是否全部成功
    pub fn is_all_successful(&self) -> bool {
        self.failed == 0
    }
    
    /// 获取摘要
    pub fn summary(&self) -> String {
        format!(
            "成功: {}, 失败: {}, 跳过: {}",
            self.successful, self.failed, self.skipped
        )
    }
}

/// 回滚结果
#[derive(Debug)]
pub struct RollbackResult {
    /// 成功数量
    pub successful: usize,
    /// 失败数量
    pub failed: usize,
    /// 错误信息
    pub errors: Vec<String>,
}

impl RollbackResult {
    /// 是否全部成功
    pub fn is_all_successful(&self) -> bool {
        self.failed == 0
    }
    
    /// 获取摘要
    pub fn summary(&self) -> String {
        format!("回滚成功: {}, 失败: {}", self.successful, self.failed)
    }
}
