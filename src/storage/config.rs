//! 配置文件管理模块

use crate::core::models::AppConfig;
use anyhow::Result;
use std::path::PathBuf;

/// 配置管理器
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// 创建配置管理器
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// 获取默认配置路径
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("com", "orderly", "Orderly")
            .map(|d| d.config_dir().join("config.json"))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    /// 加载配置
    pub fn load(&self) -> Result<AppConfig> {
        if self.config_path.exists() {
            let content = std::fs::read_to_string(&self.config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(AppConfig::default())
        }
    }

    /// 保存配置
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// 重置为默认配置
    pub fn reset(&self) -> Result<()> {
        self.save(&AppConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_save_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        
        let manager = ConfigManager::new(config_path);
        
        let mut config = AppConfig::default();
        config.confidence_threshold = 0.8;
        
        manager.save(&config).unwrap();
        
        let loaded = manager.load().unwrap();
        assert_eq!(loaded.confidence_threshold, 0.8);
    }
}
