//! 目录边界识别模块
//! 
//! 负责识别"原子目录"（Atomic Directory），防止破坏程序结构。
//! 
//! 设计原则：
//! - 标记为 atomic = true 的目录，内部文件禁止单独移动
//! - 只允许：整体移动、忽略、归档
//! - 使用启发式规则进行识别，不依赖AI

use crate::core::models::{DirectoryType, FileDescriptor};
use std::collections::HashSet;
use std::path::Path;

/// 目录边界分析器
pub struct BoundaryAnalyzer {
    /// 程序文件扩展名
    program_extensions: HashSet<String>,
    /// 程序目录标志文件
    program_markers: HashSet<String>,
    /// 开发项目标志文件
    dev_project_markers: HashSet<String>,
    /// 虚拟环境目录名
    venv_dir_names: HashSet<String>,
    /// 系统路径前缀（Windows）
    system_path_prefixes_windows: Vec<String>,
    /// 系统路径前缀（Unix）
    system_path_prefixes_unix: Vec<String>,
}

impl Default for BoundaryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundaryAnalyzer {
    /// 创建新的分析器
    pub fn new() -> Self {
        Self {
            program_extensions: [
                ".exe", ".dll", ".so", ".dylib", ".sys", ".ocx", ".msi",
                ".app", ".deb", ".rpm", ".dmg",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),

            program_markers: [
                // Windows程序目录标志
                "unins000.exe",
                "uninstall.exe",
                "setup.exe",
                // macOS应用标志
                "Contents",
                "MacOS",
                "Info.plist",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),

            dev_project_markers: [
                // Node.js
                "package.json",
                "package-lock.json",
                "yarn.lock",
                "pnpm-lock.yaml",
                // Python
                "requirements.txt",
                "setup.py",
                "pyproject.toml",
                "Pipfile",
                // Rust
                "Cargo.toml",
                "Cargo.lock",
                // C/C++
                "CMakeLists.txt",
                "Makefile",
                "configure",
                // .NET
                ".csproj",
                ".fsproj",
                ".sln",
                // Java
                "pom.xml",
                "build.gradle",
                "settings.gradle",
                // Go
                "go.mod",
                "go.sum",
                // Ruby
                "Gemfile",
                "Gemfile.lock",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),

            venv_dir_names: [
                // Python
                "venv",
                ".venv",
                "env",
                ".env",
                "virtualenv",
                "__pycache__",
                ".pyc",
                "site-packages",
                // Node.js
                "node_modules",
                // Rust
                "target",
                // Go
                "vendor",
                // .NET
                "bin",
                "obj",
                "packages",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),

            system_path_prefixes_windows: vec![
                "C:\\Windows".to_string(),
                "C:\\Program Files".to_string(),
                "C:\\Program Files (x86)".to_string(),
                "C:\\ProgramData".to_string(),
            ],

            system_path_prefixes_unix: vec![
                "/usr".to_string(),
                "/opt".to_string(),
                "/bin".to_string(),
                "/sbin".to_string(),
                "/lib".to_string(),
                "/etc".to_string(),
                "/var".to_string(),
                "/Applications".to_string(),
            ],
        }
    }

    /// 分析文件列表，标记原子目录
    pub fn analyze(&self, files: &mut Vec<FileDescriptor>) {
        // 首先收集需要分析的目录路径
        let dir_paths: Vec<(usize, std::path::PathBuf)> = files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_directory)
            .map(|(i, f)| (i, f.full_path.clone()))
            .collect();

        // 分析每个目录
        let results: Vec<(usize, DirectoryType, bool)> = dir_paths
            .iter()
            .map(|(i, path)| {
                let (dir_type, atomic) = self.analyze_directory(path, files);
                (*i, dir_type, atomic)
            })
            .collect();

        // 应用结果
        for (idx, dir_type, atomic) in results {
            files[idx].directory_type = dir_type;
            files[idx].atomic = atomic;
        }

        // 标记原子目录下的所有文件
        let atomic_dirs: Vec<_> = files
            .iter()
            .filter(|f| f.is_directory && f.atomic)
            .map(|f| f.full_path.clone())
            .collect();

        for file in files.iter_mut() {
            if !file.is_directory {
                for atomic_dir in &atomic_dirs {
                    if file.full_path.starts_with(atomic_dir) {
                        file.atomic = true;
                        file.directory_type = DirectoryType::ProgramRoot;
                        break;
                    }
                }
            }
        }
    }

    /// 分析单个目录
    fn analyze_directory(
        &self,
        path: &Path,
        all_files: &[FileDescriptor],
    ) -> (DirectoryType, bool) {
        let path_str = path.to_string_lossy().to_string();

        // 1. 检查系统路径
        if self.is_system_path(&path_str) {
            return (DirectoryType::System, true);
        }

        // 2. 获取目录下的直接子项
        let children: Vec<_> = all_files
            .iter()
            .filter(|f| {
                f.parent_dir == path
            })
            .collect();

        // 3. 检查是否为虚拟环境目录
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if self.venv_dir_names.contains(&dir_name.to_lowercase()) {
            return (DirectoryType::VirtualEnv, true);
        }

        // 4. 检查是否包含程序文件标志
        let has_program_markers = children.iter().any(|f| {
            // 检查可执行文件
            if self.program_extensions.contains(&f.extension.to_lowercase()) {
                return true;
            }
            // 检查程序目录标志文件
            if self.program_markers.contains(&f.name.to_lowercase()) {
                return true;
            }
            false
        });

        // 5. 检查是否同时有exe和dll（强信号）
        let has_exe = children.iter().any(|f| f.extension.to_lowercase() == ".exe");
        let has_dll = children.iter().any(|f| f.extension.to_lowercase() == ".dll");

        if has_exe && has_dll {
            return (DirectoryType::ProgramRoot, true);
        }

        // 6. 检查是否为开发项目目录
        let has_dev_markers = children.iter().any(|f| {
            self.dev_project_markers.contains(&f.name.to_lowercase())
                || self.dev_project_markers.iter().any(|m| f.name.ends_with(m))
        });

        if has_dev_markers {
            // 开发项目目录，但不一定是atomic
            // 只有当包含 node_modules 或 venv 时才标记为 atomic
            let has_venv_child = children.iter().any(|f| {
                f.is_directory && self.venv_dir_names.contains(&f.name.to_lowercase())
            });

            if has_venv_child || has_program_markers {
                return (DirectoryType::ProgramRoot, true);
            }
        }

        // 7. 检查标准目录结构 (bin + lib)
        let has_bin = children.iter().any(|f| f.is_directory && f.name.to_lowercase() == "bin");
        let has_lib = children.iter().any(|f| f.is_directory && f.name.to_lowercase() == "lib");

        if has_bin && has_lib {
            return (DirectoryType::ProgramRoot, true);
        }

        (DirectoryType::Normal, false)
    }

    /// 检查是否为系统路径
    fn is_system_path(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();

        // Windows路径检查
        for prefix in &self.system_path_prefixes_windows {
            if path_lower.starts_with(&prefix.to_lowercase()) {
                return true;
            }
        }

        // Unix路径检查
        for prefix in &self.system_path_prefixes_unix {
            if path_lower.starts_with(&prefix.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// 检查单个文件是否属于程序目录
    pub fn is_in_program_directory(&self, file: &FileDescriptor, all_files: &[FileDescriptor]) -> bool {
        // 向上遍历父目录
        let mut current = file.parent_dir.clone();
        
        while let Some(parent) = current.parent() {
            // 在已扫描的文件中查找此目录
            if let Some(dir_file) = all_files.iter().find(|f| f.is_directory && f.full_path == current) {
                if dir_file.atomic {
                    return true;
                }
            }
            current = parent.to_path_buf();
        }
        
        false
    }
}

/// 快速检查目录是否可能是原子目录（不需要完整扫描）
pub fn quick_check_atomic(path: &Path) -> bool {
    let entries: Vec<_> = match std::fs::read_dir(path) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return false,
    };

    // 快速检查标志
    let mut has_exe = false;
    let mut has_dll = false;
    let mut has_package_json = false;
    let mut has_cargo_toml = false;
    let mut has_node_modules = false;
    let mut has_venv = false;

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        
        if name.ends_with(".exe") {
            has_exe = true;
        } else if name.ends_with(".dll") {
            has_dll = true;
        } else if name == "package.json" {
            has_package_json = true;
        } else if name == "cargo.toml" {
            has_cargo_toml = true;
        } else if name == "node_modules" {
            has_node_modules = true;
        } else if name == "venv" || name == ".venv" || name == "__pycache__" {
            has_venv = true;
        }
    }

    // 判断条件
    (has_exe && has_dll) // Windows程序
        || has_node_modules // Node.js项目
        || has_venv // Python项目
        || (has_package_json && has_node_modules)
        || (has_cargo_toml && path.join("target").exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_path_detection() {
        let analyzer = BoundaryAnalyzer::new();
        
        assert!(analyzer.is_system_path("C:\\Windows\\System32"));
        assert!(analyzer.is_system_path("C:\\Program Files\\SomeApp"));
        assert!(!analyzer.is_system_path("D:\\MyDocuments"));
    }
}
