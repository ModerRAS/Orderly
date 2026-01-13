//! 预览表格组件
//! 
//! 显示文件列表，支持：
//! - 勾选/取消勾选
//! - 显示当前路径和建议路径
//! - 置信度颜色
//! - 原子目录高亮

use crate::core::models::{FileDescriptor, SuggestionSource};
use crate::ui::styles::Theme;
use eframe::egui::{self, RichText, Ui};
use std::path::{Path, PathBuf};

/// 预览表格
pub struct PreviewTable {
    /// 主题
    theme: Theme,
    /// 排序列
    sort_column: SortColumn,
    /// 排序方向
    sort_ascending: bool,
    /// 搜索过滤
    filter_text: String,
    /// 是否只显示有建议的文件
    show_only_with_suggestion: bool,
    /// 是否隐藏原子目录内的文件
    hide_atomic_children: bool,
}

/// 排序列
#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    Name,
    Path,
    Target,
    Confidence,
    Source,
}

fn effective_target_path(file: &FileDescriptor, suggested: &Path) -> PathBuf {
    // 与执行层保持一致：只做“分类移动”，最终目标必须使用原文件名。
    // 如果 suggested 看起来已经包含文件名（等于原名 / 以扩展名结尾），则取其 parent 作为目录。
    let leaf = suggested
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let ext_lower = file.extension.to_lowercase();
    let looks_like_file_path = (!leaf.is_empty() && leaf == file.name)
        || (!ext_lower.is_empty() && leaf.to_lowercase().ends_with(&ext_lower));

    let target_dir = if looks_like_file_path {
        suggested.parent().unwrap_or(suggested)
    } else {
        suggested
    };

    target_dir.join(&file.name)
}

impl Default for PreviewTable {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            sort_column: SortColumn::Name,
            sort_ascending: true,
            filter_text: String::new(),
            show_only_with_suggestion: false,
            hide_atomic_children: true,
        }
    }
}

impl PreviewTable {
    /// 创建新的预览表格
    pub fn new() -> Self {
        Self::default()
    }

    /// 渲染工具栏
    pub fn render_toolbar(&mut self, ui: &mut Ui, files: &mut [FileDescriptor]) {
        ui.horizontal(|ui| {
            // 搜索框
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_text)
                    .hint_text("搜索文件...")
                    .desired_width(200.0)
            );

            ui.separator();

            // 过滤选项
            ui.checkbox(&mut self.show_only_with_suggestion, "只显示有建议的");
            ui.checkbox(&mut self.hide_atomic_children, "隐藏程序目录内文件");

            ui.separator();

            // 批量操作
            if ui.button("✓ 全选").clicked() {
                for file in files.iter_mut() {
                    if !file.atomic || file.is_directory {
                        file.selected = true;
                    }
                }
            }
            if ui.button("✗ 全不选").clicked() {
                for file in files.iter_mut() {
                    file.selected = false;
                }
            }
            if ui.button("↔ 反选").clicked() {
                for file in files.iter_mut() {
                    if !file.atomic || file.is_directory {
                        file.selected = !file.selected;
                    }
                }
            }
        });
    }

    /// 渲染表格
    pub fn render(&mut self, ui: &mut Ui, files: &mut [FileDescriptor]) {
        // 表头
        ui.horizontal(|ui| {
            ui.set_min_height(30.0);
            
            // 选择列
            ui.allocate_ui_with_layout(
                egui::vec2(30.0, 20.0),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| { ui.label(""); }
            );

            // 文件名列
            if ui.selectable_label(
                self.sort_column == SortColumn::Name,
                format!("文件名 {}", self.sort_indicator(SortColumn::Name))
            ).clicked() {
                self.toggle_sort(SortColumn::Name);
            }

            ui.separator();

            // 当前路径列
            if ui.selectable_label(
                self.sort_column == SortColumn::Path,
                format!("当前路径 {}", self.sort_indicator(SortColumn::Path))
            ).clicked() {
                self.toggle_sort(SortColumn::Path);
            }

            ui.separator();

            // 建议路径列
            if ui.selectable_label(
                self.sort_column == SortColumn::Target,
                format!("建议路径 {}", self.sort_indicator(SortColumn::Target))
            ).clicked() {
                self.toggle_sort(SortColumn::Target);
            }

            ui.separator();

            // 置信度列
            if ui.selectable_label(
                self.sort_column == SortColumn::Confidence,
                format!("置信度 {}", self.sort_indicator(SortColumn::Confidence))
            ).clicked() {
                self.toggle_sort(SortColumn::Confidence);
            }

            ui.separator();

            // 来源列
            if ui.selectable_label(
                self.sort_column == SortColumn::Source,
                format!("来源 {}", self.sort_indicator(SortColumn::Source))
            ).clicked() {
                self.toggle_sort(SortColumn::Source);
            }
        });

        ui.separator();

        // 表格内容
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for file in files.iter_mut() {
                    // 过滤
                    if !self.should_show_file(file) {
                        continue;
                    }

                    self.render_row(ui, file);
                }
            });
    }

    /// 判断是否应该显示此文件
    fn should_show_file(&self, file: &FileDescriptor) -> bool {
        // 搜索过滤
        if !self.filter_text.is_empty() {
            let filter = self.filter_text.to_lowercase();
            if !file.name.to_lowercase().contains(&filter)
                && !file.full_path.to_string_lossy().to_lowercase().contains(&filter)
            {
                return false;
            }
        }

        // 只显示有建议的
        if self.show_only_with_suggestion && file.suggested_action.is_none() {
            return false;
        }

        // 隐藏原子目录内的文件
        if self.hide_atomic_children && file.atomic && !file.is_directory {
            return false;
        }

        true
    }

    /// 渲染单行
    fn render_row(&mut self, ui: &mut Ui, file: &mut FileDescriptor) {
        let is_atomic = file.atomic;
        let is_directory = file.is_directory;

        // 行背景色
        let bg_color = if is_atomic {
            self.theme.atomic_highlight.gamma_multiply(0.2)
        } else if file.selected {
            self.theme.selected_bg
        } else {
            self.theme.unselected_bg
        };

        egui::Frame::none()
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 选择框
                    let checkbox_enabled = !is_atomic || is_directory;
                    ui.add_enabled(
                        checkbox_enabled,
                        egui::Checkbox::without_text(&mut file.selected)
                    );

                    // 文件图标和名称
                    let icon = if is_directory {
                        if is_atomic { "🔒" } else { "📁" }
                    } else {
                        self.get_file_icon(&file.extension)
                    };

                    ui.label(format!("{} {}", icon, file.name));

                    ui.separator();

                    // 当前路径（截断显示）
                    let current_path = file.parent_dir.to_string_lossy();
                    let truncated_path = Self::truncate_path(&current_path, 40);
                    ui.label(&truncated_path).on_hover_text(&*current_path);

                    ui.separator();

                    // 建议路径
                    if let Some(ref suggestion) = file.suggested_action {
                        let target_path = effective_target_path(file, &suggestion.target_path);
                        let target = target_path.to_string_lossy();
                        let truncated_target = Self::truncate_path(&target, 40);
                        ui.label(&truncated_target).on_hover_text(&*target);

                        ui.separator();

                        // 置信度
                        let confidence_color = self.theme.confidence_color(suggestion.confidence);
                        ui.label(
                            RichText::new(format!("{:.0}%", suggestion.confidence * 100.0))
                                .color(confidence_color)
                        );

                        ui.separator();

                        // 来源
                        let source_text = match suggestion.source {
                            SuggestionSource::AI => "🤖 AI",
                            SuggestionSource::Rule => "📋 规则",
                            SuggestionSource::Memory => "💾 记忆",
                        };
                        ui.label(source_text);
                    } else if is_atomic {
                        ui.label(
                            RichText::new("🔒 原子目录")
                                .color(self.theme.atomic_highlight)
                        );
                    } else {
                        ui.label(
                            RichText::new("无建议")
                                .color(self.theme.secondary)
                        );
                    }
                });
            });
    }

    /// 获取文件图标
    fn get_file_icon(&self, extension: &str) -> &'static str {
        match extension.to_lowercase().as_str() {
            ".jpg" | ".jpeg" | ".png" | ".gif" | ".bmp" | ".webp" => "🖼️",
            ".mp4" | ".avi" | ".mkv" | ".mov" | ".wmv" => "🎬",
            ".mp3" | ".wav" | ".flac" | ".aac" | ".ogg" => "🎵",
            ".pdf" => "📕",
            ".doc" | ".docx" => "📝",
            ".xls" | ".xlsx" => "📊",
            ".ppt" | ".pptx" => "📽️",
            ".zip" | ".rar" | ".7z" | ".tar" | ".gz" => "📦",
            ".exe" | ".msi" => "⚙️",
            ".txt" | ".md" | ".log" => "📄",
            ".html" | ".css" | ".js" | ".ts" => "🌐",
            ".py" | ".rs" | ".go" | ".java" | ".c" | ".cpp" => "💻",
            ".json" | ".xml" | ".yaml" | ".yml" => "📋",
            _ => "📄",
        }
    }

    /// 截断路径显示
    fn truncate_path(path: &str, max_len: usize) -> String {
        if path.len() <= max_len {
            path.to_string()
        } else {
            format!("...{}", &path[path.len() - max_len + 3..])
        }
    }

    /// 获取排序指示器
    fn sort_indicator(&self, column: SortColumn) -> &'static str {
        if self.sort_column == column {
            if self.sort_ascending { "▲" } else { "▼" }
        } else {
            ""
        }
    }

    /// 切换排序
    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
    }

    /// 对文件列表排序
    pub fn sort_files(&self, files: &mut [FileDescriptor]) {
        files.sort_by(|a, b| {
            let ord = match self.sort_column {
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Path => a.parent_dir.cmp(&b.parent_dir),
                SortColumn::Target => {
                    let a_target = a
                        .suggested_action
                        .as_ref()
                        .map(|s| effective_target_path(a, &s.target_path).to_string_lossy().to_string())
                        .unwrap_or_default();
                    let b_target = b
                        .suggested_action
                        .as_ref()
                        .map(|s| effective_target_path(b, &s.target_path).to_string_lossy().to_string())
                        .unwrap_or_default();
                    a_target.cmp(&b_target)
                }
                SortColumn::Confidence => {
                    let a_conf = a.suggested_action.as_ref().map(|s| (s.confidence * 100.0) as i32).unwrap_or(0);
                    let b_conf = b.suggested_action.as_ref().map(|s| (s.confidence * 100.0) as i32).unwrap_or(0);
                    a_conf.cmp(&b_conf)
                }
                SortColumn::Source => {
                    let a_src = a.suggested_action.as_ref().map(|s| format!("{:?}", s.source));
                    let b_src = b.suggested_action.as_ref().map(|s| format!("{:?}", s.source));
                    a_src.cmp(&b_src)
                }
            };

            if self.sort_ascending { ord } else { ord.reverse() }
        });
    }
}

/// 获取统计信息
pub struct TableStats {
    pub total_files: usize,
    pub selected_files: usize,
    pub with_suggestion: usize,
    pub atomic_files: usize,
}

impl TableStats {
    pub fn from_files(files: &[FileDescriptor]) -> Self {
        Self {
            total_files: files.len(),
            selected_files: files.iter().filter(|f| f.selected).count(),
            with_suggestion: files.iter().filter(|f| f.suggested_action.is_some()).count(),
            atomic_files: files.iter().filter(|f| f.atomic).count(),
        }
    }
}
