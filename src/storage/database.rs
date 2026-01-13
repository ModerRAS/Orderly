//! 数据库存储模块
//! 
//! 使用SQLite存储规则和历史记录

use crate::core::models::{HistoryEntry, RuleDefinition};
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// 数据库管理器
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 打开或创建数据库
    pub fn open(path: &PathBuf) -> Result<Self> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// 初始化表结构
    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- 规则表
            CREATE TABLE IF NOT EXISTS rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 50,
                enabled INTEGER NOT NULL DEFAULT 1,
                condition_json TEXT NOT NULL,
                action_json TEXT NOT NULL,
                origin TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 0
            );

            -- 历史记录表
            CREATE TABLE IF NOT EXISTS history (
                batch_id TEXT PRIMARY KEY,
                executed_at TEXT NOT NULL,
                operations_json TEXT NOT NULL,
                rolled_back INTEGER NOT NULL DEFAULT 0
            );

            -- 记忆缓存表（文件特征 -> 路径映射）
            CREATE TABLE IF NOT EXISTS memory_cache (
                feature_hash TEXT PRIMARY KEY,
                target_path TEXT NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 1,
                last_hit TEXT NOT NULL
            );

            -- 创建索引
            CREATE INDEX IF NOT EXISTS idx_rules_priority ON rules(priority DESC);
            CREATE INDEX IF NOT EXISTS idx_rules_enabled ON rules(enabled);
            CREATE INDEX IF NOT EXISTS idx_history_executed ON history(executed_at DESC);
            "#,
        )?;
        Ok(())
    }

    /// 保存规则
    pub fn save_rule(&self, rule: &RuleDefinition) -> Result<()> {
        let condition_json = serde_json::to_string(&rule.condition)?;
        let action_json = serde_json::to_string(&rule.action)?;
        let origin = format!("{:?}", rule.origin);
        let created_at = rule.created_at.to_rfc3339();
        let updated_at = rule.updated_at.to_rfc3339();

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO rules 
            (id, name, priority, enabled, condition_json, action_json, origin, created_at, updated_at, hit_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                rule.id,
                rule.name,
                rule.priority,
                rule.enabled,
                condition_json,
                action_json,
                origin,
                created_at,
                updated_at,
                rule.hit_count,
            ],
        )?;
        Ok(())
    }

    /// 加载所有用户规则
    pub fn load_user_rules(&self) -> Result<Vec<RuleDefinition>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, priority, enabled, condition_json, action_json, origin, created_at, updated_at, hit_count
            FROM rules
            WHERE origin = 'UserConfirmed'
            ORDER BY priority DESC
            "#,
        )?;

        let rules = stmt.query_map([], |row| {
            let condition_json: String = row.get(4)?;
            let action_json: String = row.get(5)?;
            let origin_str: String = row.get(6)?;
            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            Ok(RuleDefinition {
                id: row.get(0)?,
                name: row.get(1)?,
                priority: row.get(2)?,
                enabled: row.get(3)?,
                condition: serde_json::from_str(&condition_json).unwrap_or_default(),
                action: serde_json::from_str(&action_json).unwrap_or_default(),
                origin: if origin_str == "BuiltIn" {
                    crate::core::models::RuleOrigin::BuiltIn
                } else {
                    crate::core::models::RuleOrigin::UserConfirmed
                },
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                hit_count: row.get(9)?,
            })
        })?;

        rules.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// 删除规则
    pub fn delete_rule(&self, rule_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM rules WHERE id = ?1", params![rule_id])?;
        Ok(())
    }

    /// 保存历史记录
    pub fn save_history(&self, entry: &HistoryEntry) -> Result<()> {
        let operations_json = serde_json::to_string(&entry.operations)?;
        let executed_at = entry.executed_at.to_rfc3339();

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO history (batch_id, executed_at, operations_json, rolled_back)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                entry.batch_id,
                executed_at,
                operations_json,
                entry.rolled_back,
            ],
        )?;
        Ok(())
    }

    /// 加载最近的历史记录
    pub fn load_recent_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT batch_id, executed_at, operations_json, rolled_back
            FROM history
            ORDER BY executed_at DESC
            LIMIT ?1
            "#,
        )?;

        let entries = stmt.query_map(params![limit], |row| {
            let operations_json: String = row.get(2)?;
            let executed_at_str: String = row.get(1)?;

            Ok(HistoryEntry {
                batch_id: row.get(0)?,
                executed_at: chrono::DateTime::parse_from_rfc3339(&executed_at_str)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                operations: serde_json::from_str(&operations_json).unwrap_or_default(),
                rolled_back: row.get(3)?,
            })
        })?;

        entries.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// 保存记忆缓存
    pub fn save_memory(&self, feature_hash: &str, target_path: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO memory_cache (feature_hash, target_path, hit_count, last_hit)
            VALUES (?1, ?2, 1, ?3)
            ON CONFLICT(feature_hash) DO UPDATE SET
                hit_count = hit_count + 1,
                last_hit = ?3
            "#,
            params![feature_hash, target_path, now],
        )?;
        Ok(())
    }

    /// 查询记忆缓存
    pub fn query_memory(&self, feature_hash: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT target_path FROM memory_cache WHERE feature_hash = ?1",
        )?;

        let result = stmt.query_row(params![feature_hash], |row| row.get(0));
        
        match result {
            Ok(path) => Ok(Some(path)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 清理旧的历史记录
    pub fn cleanup_old_history(&self, keep_count: usize) -> Result<usize> {
        let affected = self.conn.execute(
            r#"
            DELETE FROM history
            WHERE batch_id NOT IN (
                SELECT batch_id FROM history
                ORDER BY executed_at DESC
                LIMIT ?1
            )
            "#,
            params![keep_count],
        )?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = Database::open(&db_path).unwrap();
        assert!(db_path.exists());
    }
}
