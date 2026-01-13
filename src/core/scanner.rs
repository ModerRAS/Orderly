//! 文件扫描模块
//! 
//! 负责递归扫描指定目录，生成 FileDescriptor 列表。
//! 此模块只做IO操作，不做任何智能判断。

use crate::core::models::FileDescriptor;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 文件扫描器
pub struct FileScanner {
    /// 扫描根路径
    root_path: PathBuf,
    /// 是否包含隐藏文件
    include_hidden: bool,
    /// 最大扫描深度（0表示无限）
    max_depth: usize,
    /// 排除的目录名称
    exclude_dirs: Vec<String>,
}

impl FileScanner {
    /// 创建新的扫描器
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            include_hidden: false,
            max_depth: 0, // 无限深度
            exclude_dirs: vec![
                "$RECYCLE.BIN".to_string(),
                "System Volume Information".to_string(),
            ],
        }
    }

    /// 设置是否包含隐藏文件
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// 设置最大扫描深度
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// 添加排除的目录
    pub fn exclude_dir(mut self, dir: String) -> Self {
        self.exclude_dirs.push(dir);
        self
    }

    /// 执行扫描
    pub fn scan(&self) -> Result<Vec<FileDescriptor>> {
        let mut files = Vec::new();
        
        let walker = if self.max_depth > 0 {
            WalkDir::new(&self.root_path).max_depth(self.max_depth)
        } else {
            WalkDir::new(&self.root_path)
        };

        for entry in walker.into_iter().filter_entry(|e| self.should_include(e)) {
            match entry {
                Ok(entry) => {
                    if let Some(descriptor) = self.create_descriptor(&entry) {
                        files.push(descriptor);
                    }
                }
                Err(e) => {
                    tracing::warn!("扫描文件时出错: {}", e);
                }
            }
        }

        tracing::info!("扫描完成，共发现 {} 个文件/目录", files.len());
        Ok(files)
    }

    /// 判断是否应该包含此条目
    fn should_include(&self, entry: &walkdir::DirEntry) -> bool {
        // 根目录必须允许遍历，否则 filter_entry 会直接阻止深入扫描
        if entry.path() == self.root_path {
            return true;
        }

        let name = entry.file_name().to_string_lossy();
        
        // 检查隐藏文件
        if !self.include_hidden && name.starts_with('.') {
            return false;
        }

        // 检查排除目录
        if entry.file_type().is_dir() {
            if self.exclude_dirs.iter().any(|d| name.eq_ignore_ascii_case(d)) {
                return false;
            }
        }

        true
    }

    /// 创建文件描述符
    fn create_descriptor(&self, entry: &walkdir::DirEntry) -> Option<FileDescriptor> {
        let metadata = entry.metadata().ok()?;
        let full_path = entry.path().to_path_buf();
        
        // 跳过根目录本身
        if full_path == self.root_path {
            return None;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let is_directory = metadata.is_dir();
        
        let extension = if is_directory {
            String::new()
        } else {
            full_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default()
        };

        let size = if is_directory { 0 } else { metadata.len() };
        
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| DateTime::<Utc>::from(t))
            .unwrap_or_else(Utc::now);

        Some(FileDescriptor::new(
            full_path,
            name,
            extension,
            size,
            modified_at,
            is_directory,
        ))
    }
}

/// 辅助函数：获取文件的内容摘要（用于AI分析）
pub fn get_content_summary(path: &Path, max_chars: usize) -> Result<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut content = String::new();
    let mut chars_read = 0;

    for line in reader.lines() {
        if chars_read >= max_chars {
            break;
        }
        if let Ok(line) = line {
            let remaining = max_chars - chars_read;
            if line.len() <= remaining {
                content.push_str(&line);
                content.push('\n');
                chars_read += line.len() + 1;
            } else {
                content.push_str(&line[..remaining]);
                break;
            }
        }
    }

    Ok(content)
}

/// 辅助函数：获取文件类型（基于magic number）
pub fn detect_file_type(path: &Path) -> Option<String> {
    infer::get_from_path(path)
        .ok()
        .flatten()
        .map(|t| t.mime_type().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scanner_basic() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let scanner = FileScanner::new(dir.path().to_path_buf());
        let files = scanner.scan().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "test.txt");
        assert_eq!(files[0].extension, ".txt");
    }
}
