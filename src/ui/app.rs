//! 主应用程序
//! 
//! 整合所有模块，提供完整的用户界面。

use crate::core::boundary::BoundaryAnalyzer;
use crate::core::executor::{DryRunResult, Executor};
use crate::core::models::{
    AppConfig, FileDescriptor, MovePlan, RuleAction, RuleCondition, RuleDefinition,
};
use crate::core::planner::Planner;
use crate::core::rule_engine::RuleEngine;
use crate::core::scanner::FileScanner;
use crate::core::semantic::mock_semantic_analysis;
use crate::ui::dialogs::{
    ErrorClusterDialog, ErrorClusterResult, ExecuteConfirmDialog, ExecuteConfirmResult,
    PromptDialog, PromptDialogResult, RuleConfirmDialog, RuleConfirmResult,
    SettingsDialog, SettingsResult,
};
use crate::ui::preview_table::{PreviewTable, TableStats};
use crate::ui::rule_panel::{RulePanel, RulePanelAction};
use crate::ui::styles::Theme;
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// 应用状态
#[derive(PartialEq)]
enum AppState {
    /// 初始状态，等待选择目录
    Initial,
    /// 扫描中
    Scanning,
    /// 分析中
    Analyzing,
    /// 预览状态
    Preview,
    /// 执行中
    Executing,
}

/// 主应用程序
#[allow(dead_code)]
pub struct OrderlyApp {
    /// 应用状态
    state: AppState,
    /// 配置
    config: AppConfig,
    /// 主题
    theme: Theme,
    /// 扫描路径
    scan_path: String,
    /// 输出路径
    output_path: String,
    /// 文件列表
    files: Vec<FileDescriptor>,
    /// 规则引擎
    rule_engine: Option<RuleEngine>,
    /// 计划生成器
    planner: Option<Planner>,
    /// 执行器
    executor: Option<Executor>,
    /// 当前移动计划
    current_plan: Option<MovePlan>,
    /// Dry Run 结果
    dry_run_result: Option<DryRunResult>,
    /// 预览表格
    preview_table: PreviewTable,
    /// 规则面板
    rule_panel: RulePanel,
    /// 提示词对话框
    prompt_dialog: PromptDialog,
    /// 规则确认对话框
    rule_confirm_dialog: RuleConfirmDialog,
    /// 执行确认对话框
    execute_confirm_dialog: ExecuteConfirmDialog,
    /// 错误聚类对话框
    error_cluster_dialog: ErrorClusterDialog,
    /// 设置对话框
    settings_dialog: SettingsDialog,
    /// 状态消息
    status_message: String,
    /// 是否显示规则面板
    show_rule_panel: bool,
    /// 是否显示历史面板
    show_history_panel: bool,
    /// 错误计数器（用于触发错误聚类检测）
    correction_counter: std::collections::HashMap<String, u32>,
    /// 待确认的规则
    pending_rule: Option<RuleDefinition>,
}

impl OrderlyApp {
    /// 创建新的应用实例
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // 获取数据目录
        let data_dir = directories::ProjectDirs::from("com", "orderly", "Orderly")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            state: AppState::Initial,
            config: AppConfig::default(),
            theme: Theme::default(),
            scan_path: String::new(),
            output_path: String::new(),
            files: Vec::new(),
            rule_engine: None,
            planner: None,
            executor: Some(Executor::new(data_dir)),
            current_plan: None,
            dry_run_result: None,
            preview_table: PreviewTable::new(),
            rule_panel: RulePanel::new(),
            prompt_dialog: PromptDialog::default(),
            rule_confirm_dialog: RuleConfirmDialog::default(),
            execute_confirm_dialog: ExecuteConfirmDialog::default(),
            error_cluster_dialog: ErrorClusterDialog::default(),
            settings_dialog: SettingsDialog::default(),
            status_message: "请选择要整理的目录".to_string(),
            show_rule_panel: false,
            show_history_panel: false,
            correction_counter: std::collections::HashMap::new(),
            pending_rule: None,
        }
    }

    /// 开始扫描
    fn start_scan(&mut self) {
        let scan_path = PathBuf::from(&self.scan_path);
        if !scan_path.exists() {
            self.status_message = "扫描路径不存在".to_string();
            return;
        }

        self.state = AppState::Scanning;
        self.status_message = "正在扫描目录...".to_string();

        // 创建扫描器并扫描
        let scanner = FileScanner::new(scan_path);
        match scanner.scan() {
            Ok(mut files) => {
                // 分析目录边界
                let analyzer = BoundaryAnalyzer::new();
                analyzer.analyze(&mut files);

                self.files = files;
                self.status_message = format!("扫描完成，共 {} 个文件/目录", self.files.len());
                
                // 初始化规则引擎
                let output_base = if self.output_path.is_empty() {
                    PathBuf::from(&self.scan_path)
                } else {
                    PathBuf::from(&self.output_path)
                };
                
                self.rule_engine = Some(RuleEngine::new(output_base.clone()));
                self.planner = Some(Planner::new(output_base, self.config.confidence_threshold));

                // 进入分析阶段
                self.start_analysis();
            }
            Err(e) => {
                self.status_message = format!("扫描失败: {}", e);
                self.state = AppState::Initial;
            }
        }
    }

    /// 开始分析
    fn start_analysis(&mut self) {
        self.state = AppState::Analyzing;
        self.status_message = "正在分析文件...".to_string();

        // 使用规则引擎匹配
        if let Some(ref mut engine) = self.rule_engine {
            engine.match_files(&mut self.files);
        }

        // 对没有规则匹配的文件使用模拟AI分析
        for file in self.files.iter_mut() {
            if file.suggested_action.is_none() && !file.atomic && !file.is_directory {
                // 模拟语义分析
                let semantic = mock_semantic_analysis(file);
                file.semantic = Some(semantic);
                
                // 尝试再次规则匹配
                if let Some(ref mut engine) = self.rule_engine {
                    if let Some(suggestion) = engine.match_file(file) {
                        file.suggested_action = Some(suggestion);
                    }
                }
            }
        }

        // 排序文件列表
        self.preview_table.sort_files(&mut self.files);

        self.state = AppState::Preview;
        let stats = TableStats::from_files(&self.files);
        self.status_message = format!(
            "分析完成: {} 个文件, {} 个有建议, {} 个原子目录",
            stats.total_files, stats.with_suggestion, stats.atomic_files
        );
    }

    /// 生成移动计划
    fn generate_plan(&mut self) {
        if let Some(ref planner) = self.planner {
            let plan = planner.generate_plan(&self.files);
            
            // 执行 Dry Run
            if let Some(ref executor) = self.executor {
                let dry_run = executor.dry_run(&plan);
                self.dry_run_result = Some(dry_run);
            }
            
            self.current_plan = Some(plan);
        }
    }

    /// 显示执行确认
    fn show_execute_confirm(&mut self) {
        if let Some(ref plan) = self.current_plan {
            if let Some(ref planner) = self.planner {
                let stats = planner.get_plan_stats(plan);
                let warnings = self.dry_run_result
                    .as_ref()
                    .map(|r| r.potential_errors.clone())
                    .unwrap_or_default();
                
                self.execute_confirm_dialog.show(
                    stats.total_operations,
                    stats.format_size(),
                    stats.target_directories,
                    warnings,
                );
            }
        }
    }

    /// 执行移动
    fn execute_move(&mut self) {
        if let Some(ref mut plan) = self.current_plan {
            if let Some(ref mut executor) = self.executor {
                self.state = AppState::Executing;
                let result = executor.execute(plan);
                
                self.status_message = format!(
                    "执行完成: {}",
                    result.summary()
                );
                
                // 清理
                self.current_plan = None;
                self.dry_run_result = None;
                
                // 重新扫描
                self.start_scan();
            }
        }
    }

    /// 检测错误聚类
    #[allow(dead_code)]
    fn check_error_cluster(&mut self, file: &FileDescriptor) {
        if let Some(ref _suggestion) = file.suggested_action {
            // 记录用户取消选择的模式
            for tag in file.semantic.as_ref().map(|s| &s.tags).unwrap_or(&vec![]) {
                let counter = self.correction_counter.entry(tag.clone()).or_insert(0);
                *counter += 1;
                
                // 触发阈值
                if *counter >= 3 {
                    let related_files: Vec<String> = self.files
                        .iter()
                        .filter(|f| {
                            f.semantic.as_ref()
                                .map(|s| s.tags.contains(tag))
                                .unwrap_or(false)
                        })
                        .take(5)
                        .map(|f| f.name.clone())
                        .collect();
                    
                    self.error_cluster_dialog.show(
                        &format!("多次取消带有 \"{}\" 标签的文件", tag),
                        related_files,
                    );
                    
                    // 重置计数器
                    self.correction_counter.remove(tag);
                    break;
                }
            }
        }
    }

    /// 处理提示词输入
    fn handle_prompt_input(&mut self, input: String) {
        // 这里应该调用AI来抽取规则
        // 目前使用简单的模拟逻辑
        let new_rule = RuleDefinition::new(
            format!("用户规则: {}", &input[..input.len().min(20)]),
            RuleCondition::default(),
            RuleAction {
                move_to: "UserDefined/{year}".to_string(),
            },
        );
        
        self.pending_rule = Some(new_rule.clone());
        
        // 显示规则确认对话框
        self.rule_confirm_dialog.show(
            &new_rule.name,
            "基于用户反馈",
            &new_rule.action.move_to,
            0,
        );
    }

    /// 保存规则
    fn save_pending_rule(&mut self) {
        if let Some(rule) = self.pending_rule.take() {
            if let Some(ref mut engine) = self.rule_engine {
                engine.add_rule(rule);
                self.status_message = "规则已保存".to_string();
            }
        }
    }
}

impl eframe::App for OrderlyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("文件", |ui| {
                    if ui.button("📂 打开目录...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.scan_path = path.to_string_lossy().to_string();
                        }
                        ui.close_menu();
                    }
                    if ui.button("⚙️ 设置").clicked() {
                        self.settings_dialog.visible = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ 退出").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("视图", |ui| {
                    if ui.checkbox(&mut self.show_rule_panel, "规则面板").clicked() {
                        ui.close_menu();
                    }
                    if ui.checkbox(&mut self.show_history_panel, "历史记录").clicked() {
                        ui.close_menu();
                    }
                });

                ui.menu_button("帮助", |ui| {
                    if ui.button("📖 关于").clicked() {
                        ui.close_menu();
                    }
                });
            });
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 状态消息
                ui.label(&self.status_message);
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 统计信息
                    if !self.files.is_empty() {
                        let stats = TableStats::from_files(&self.files);
                        ui.label(format!(
                            "已选: {}/{}", 
                            stats.selected_files, 
                            stats.total_files
                        ));
                    }
                });
            });
        });

        // 左侧规则面板（可选）
        if self.show_rule_panel {
            egui::SidePanel::left("rule_panel")
                .default_width(300.0)
                .show(ctx, |ui| {
                    if let Some(ref mut engine) = self.rule_engine {
                        let action = self.rule_panel.render(ui, engine.get_rules_mut());
                        
                        match action {
                            RulePanelAction::CreateNew => {
                                self.prompt_dialog.show(
                                    "创建新规则",
                                    "请用自然语言描述您的分类规则：",
                                    "",
                                );
                            }
                            RulePanelAction::Delete(id) => {
                                engine.remove_rule(&id);
                                self.rule_panel.reset_selection();
                            }
                            RulePanelAction::SaveEdit(id) => {
                                let data = self.rule_panel.get_edited_rule();
                                if let Some(rule) = engine.get_rules_mut().iter_mut().find(|r| r.id == id) {
                                    rule.name = data.name;
                                    rule.action.move_to = data.target;
                                    rule.condition.file_extensions = data.extensions;
                                    rule.condition.filename_keywords = data.keywords;
                                    rule.condition.semantic_tags = data.tags;
                                    rule.priority = data.priority;
                                }
                            }
                            RulePanelAction::None => {}
                        }
                    }
                });
        }

        // 主内容区域
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state {
                AppState::Initial => {
                    self.render_initial_view(ui);
                }
                AppState::Scanning | AppState::Analyzing => {
                    self.render_loading_view(ui);
                }
                AppState::Preview => {
                    self.render_preview_view(ui);
                }
                AppState::Executing => {
                    self.render_executing_view(ui);
                }
            }
        });

        // 渲染对话框
        self.render_dialogs(ctx);
    }
}

impl OrderlyApp {
    /// 渲染初始视图
    fn render_initial_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            
            ui.heading(RichText::new("📁 Orderly").size(48.0));
            ui.label("AI增强型本地文件整理工具");
            
            ui.add_space(30.0);

            ui.group(|ui| {
                ui.set_min_width(400.0);
                
                ui.horizontal(|ui| {
                    ui.label("扫描目录:");
                    ui.text_edit_singleline(&mut self.scan_path);
                    if ui.button("📂 浏览").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.scan_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("输出目录:");
                    ui.text_edit_singleline(&mut self.output_path);
                    if ui.button("📂 浏览").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.label(
                    RichText::new("（留空则在原目录内整理）")
                        .small()
                        .color(egui::Color32::GRAY)
                );
            });

            ui.add_space(20.0);

            let can_scan = !self.scan_path.is_empty();
            if ui.add_enabled(can_scan, egui::Button::new("🚀 开始扫描")).clicked() {
                self.start_scan();
            }
        });
    }

    /// 渲染加载视图
    fn render_loading_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(200.0);
            ui.spinner();
            ui.add_space(20.0);
            ui.label(&self.status_message);
        });
    }

    /// 渲染预览视图
    fn render_preview_view(&mut self, ui: &mut egui::Ui) {
        // 工具栏
        ui.horizontal(|ui| {
            if ui.button("📂 重新扫描").clicked() {
                self.start_scan();
            }
            
            ui.separator();
            
            if ui.button("✏️ 提示词修正").clicked() {
                self.prompt_dialog.show(
                    "修正分类规则",
                    "请描述您希望如何修改分类逻辑：",
                    &self.status_message,
                );
            }
            
            ui.separator();
            
            let selected_count = self.files.iter().filter(|f| f.selected).count();
            let can_execute = selected_count > 0;
            
            if ui.add_enabled(can_execute, egui::Button::new("▶️ 预览执行")).clicked() {
                self.generate_plan();
                self.show_execute_confirm();
            }
        });

        ui.separator();

        // 预览表格工具栏
        self.preview_table.render_toolbar(ui, &mut self.files);
        
        ui.separator();

        // 预览表格
        self.preview_table.render(ui, &mut self.files);
    }

    /// 渲染执行视图
    fn render_executing_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(200.0);
            ui.spinner();
            ui.add_space(20.0);
            ui.label("正在执行文件移动...");
        });
    }

    /// 渲染对话框
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        // 提示词对话框
        match self.prompt_dialog.render(ctx) {
            PromptDialogResult::Confirm(input) => {
                self.handle_prompt_input(input);
            }
            PromptDialogResult::Cancel => {}
            PromptDialogResult::None => {}
        }

        // 规则确认对话框
        match self.rule_confirm_dialog.render(ctx) {
            RuleConfirmResult::Accept => {
                self.save_pending_rule();
                // 重新分析
                self.start_analysis();
            }
            RuleConfirmResult::ApplyOnce => {
                // 仅本次应用，不保存
                self.pending_rule = None;
            }
            RuleConfirmResult::Cancel => {
                self.pending_rule = None;
            }
            RuleConfirmResult::None => {}
        }

        // 执行确认对话框
        match self.execute_confirm_dialog.render(ctx) {
            ExecuteConfirmResult::Execute => {
                self.execute_move();
            }
            ExecuteConfirmResult::Cancel => {
                self.current_plan = None;
                self.dry_run_result = None;
            }
            ExecuteConfirmResult::None => {}
        }

        // 错误聚类对话框
        match self.error_cluster_dialog.render(ctx) {
            ErrorClusterResult::WritePrompt => {
                self.prompt_dialog.show(
                    "修正分类规则",
                    "请描述您希望如何修改分类逻辑：",
                    &self.error_cluster_dialog.description,
                );
            }
            ErrorClusterResult::Ignore => {}
            ErrorClusterResult::None => {}
        }

        // 设置对话框
        match self.settings_dialog.render(ctx) {
            SettingsResult::Save => {
                // 保存设置
                self.config.ai_config.api_endpoint = self.settings_dialog.ai_endpoint.clone();
                self.config.ai_config.api_key = self.settings_dialog.ai_key.clone();
                self.config.ai_config.model_name = self.settings_dialog.model_name.clone();
                self.config.confidence_threshold = self.settings_dialog.confidence_threshold;
                self.config.ai_enabled = self.settings_dialog.ai_enabled;
                
                if !self.settings_dialog.default_scan_path.is_empty() {
                    self.config.default_scan_path = Some(PathBuf::from(&self.settings_dialog.default_scan_path));
                }
                if !self.settings_dialog.default_output_path.is_empty() {
                    self.config.default_output_base = Some(PathBuf::from(&self.settings_dialog.default_output_path));
                }
                
                self.status_message = "设置已保存".to_string();
            }
            SettingsResult::Cancel => {}
            SettingsResult::None => {}
        }
    }
}
