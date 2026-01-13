//! 规则管理面板

use crate::core::models::RuleDefinition;
use eframe::egui::{self, RichText, Ui};

/// 规则面板
pub struct RulePanel {
    /// 是否显示内置规则
    show_builtin: bool,
    /// 选中的规则ID
    selected_rule_id: Option<String>,
    /// 是否处于编辑模式
    editing: bool,
    /// 编辑中的规则名称
    edit_name: String,
    /// 编辑中的目标路径
    edit_target: String,
    /// 编辑中的扩展名（逗号分隔）
    edit_extensions: String,
    /// 编辑中的关键词（逗号分隔）
    edit_keywords: String,
    /// 编辑中的标签（逗号分隔）
    edit_tags: String,
    /// 编辑中的优先级
    edit_priority: u8,
}

impl Default for RulePanel {
    fn default() -> Self {
        Self {
            show_builtin: true,
            selected_rule_id: None,
            editing: false,
            edit_name: String::new(),
            edit_target: String::new(),
            edit_extensions: String::new(),
            edit_keywords: String::new(),
            edit_tags: String::new(),
            edit_priority: 50,
        }
    }
}

impl RulePanel {
    /// 创建新的规则面板
    pub fn new() -> Self {
        Self::default()
    }

    /// 渲染规则面板
    pub fn render(&mut self, ui: &mut Ui, rules: &mut Vec<RuleDefinition>) -> RulePanelAction {
        let mut action = RulePanelAction::None;

        ui.horizontal(|ui| {
            ui.heading("📋 规则管理");
            ui.separator();
            ui.checkbox(&mut self.show_builtin, "显示内置规则");
            
            if ui.button("➕ 新建规则").clicked() {
                action = RulePanelAction::CreateNew;
            }
        });

        ui.separator();

        // 规则列表
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for rule in rules.iter_mut() {
                    // 过滤内置规则
                    if !self.show_builtin && rule.origin == crate::core::models::RuleOrigin::BuiltIn {
                        continue;
                    }

                    let is_selected = self.selected_rule_id.as_ref() == Some(&rule.id);
                    
                    egui::Frame::none()
                        .fill(if is_selected {
                            egui::Color32::from_rgba_unmultiplied(66, 133, 244, 30)
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // 启用开关
                                ui.checkbox(&mut rule.enabled, "");

                                // 规则名称
                                let name_color = if rule.enabled {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::GRAY
                                };
                                
                                if ui.selectable_label(is_selected, 
                                    RichText::new(&rule.name).color(name_color)
                                ).clicked() {
                                    self.selected_rule_id = Some(rule.id.clone());
                                    self.load_rule_for_edit(rule);
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // 优先级
                                    ui.label(
                                        RichText::new(format!("P{}", rule.priority))
                                            .small()
                                            .color(egui::Color32::GRAY)
                                    );

                                    // 命中次数
                                    ui.label(
                                        RichText::new(format!("×{}", rule.hit_count))
                                            .small()
                                            .color(egui::Color32::GRAY)
                                    );

                                    // 来源标签
                                    let origin_text = match rule.origin {
                                        crate::core::models::RuleOrigin::BuiltIn => "内置",
                                        crate::core::models::RuleOrigin::UserConfirmed => "用户",
                                    };
                                    ui.label(
                                        RichText::new(origin_text)
                                            .small()
                                            .color(egui::Color32::LIGHT_BLUE)
                                    );
                                });
                            });
                        });
                }
            });

        ui.separator();

        // 选中规则的详情/编辑
        if let Some(ref rule_id) = self.selected_rule_id.clone() {
            if let Some(rule) = rules.iter().find(|r| &r.id == rule_id) {
                ui.group(|ui| {
                    ui.heading("规则详情");
                    
                    ui.horizontal(|ui| {
                        ui.label("名称:");
                        if self.editing {
                            ui.text_edit_singleline(&mut self.edit_name);
                        } else {
                            ui.label(&rule.name);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("目标路径:");
                        if self.editing {
                            ui.text_edit_singleline(&mut self.edit_target);
                        } else {
                            ui.label(&rule.action.move_to);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("扩展名:");
                        if self.editing {
                            ui.text_edit_singleline(&mut self.edit_extensions);
                        } else {
                            ui.label(rule.condition.file_extensions.join(", "));
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("关键词:");
                        if self.editing {
                            ui.text_edit_singleline(&mut self.edit_keywords);
                        } else {
                            ui.label(rule.condition.filename_keywords.join(", "));
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("语义标签:");
                        if self.editing {
                            ui.text_edit_singleline(&mut self.edit_tags);
                        } else {
                            ui.label(rule.condition.semantic_tags.join(", "));
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("优先级:");
                        if self.editing {
                            ui.add(egui::Slider::new(&mut self.edit_priority, 0..=100));
                        } else {
                            ui.label(format!("{}", rule.priority));
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if self.editing {
                            if ui.button("💾 保存").clicked() {
                                action = RulePanelAction::SaveEdit(rule_id.clone());
                                self.editing = false;
                            }
                            if ui.button("❌ 取消").clicked() {
                                self.editing = false;
                            }
                        } else {
                            if rule.origin == crate::core::models::RuleOrigin::UserConfirmed {
                                if ui.button("✏️ 编辑").clicked() {
                                    self.editing = true;
                                }
                                if ui.button("🗑️ 删除").clicked() {
                                    action = RulePanelAction::Delete(rule_id.clone());
                                }
                            }
                        }
                    });
                });
            }
        }

        action
    }

    /// 加载规则到编辑字段
    fn load_rule_for_edit(&mut self, rule: &RuleDefinition) {
        self.edit_name = rule.name.clone();
        self.edit_target = rule.action.move_to.clone();
        self.edit_extensions = rule.condition.file_extensions.join(", ");
        self.edit_keywords = rule.condition.filename_keywords.join(", ");
        self.edit_tags = rule.condition.semantic_tags.join(", ");
        self.edit_priority = rule.priority;
    }

    /// 获取编辑后的规则数据
    pub fn get_edited_rule(&self) -> EditedRuleData {
        EditedRuleData {
            name: self.edit_name.clone(),
            target: self.edit_target.clone(),
            extensions: self.edit_extensions
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            keywords: self.edit_keywords
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            tags: self.edit_tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            priority: self.edit_priority,
        }
    }

    /// 重置选择
    pub fn reset_selection(&mut self) {
        self.selected_rule_id = None;
        self.editing = false;
    }
}

/// 规则面板操作
#[derive(Debug)]
pub enum RulePanelAction {
    None,
    CreateNew,
    SaveEdit(String),
    Delete(String),
}

/// 编辑后的规则数据
pub struct EditedRuleData {
    pub name: String,
    pub target: String,
    pub extensions: Vec<String>,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub priority: u8,
}
