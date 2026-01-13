//! 样式定义

use eframe::egui::{self, Color32, Rounding, Stroke};

/// 颜色主题
pub struct Theme {
    pub primary: Color32,
    pub secondary: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub atomic_highlight: Color32,
    pub selected_bg: Color32,
    pub unselected_bg: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color32::from_rgb(66, 133, 244),      // 蓝色
            secondary: Color32::from_rgb(156, 156, 156),   // 灰色
            success: Color32::from_rgb(52, 168, 83),       // 绿色
            warning: Color32::from_rgb(251, 188, 4),       // 黄色
            error: Color32::from_rgb(234, 67, 53),         // 红色
            atomic_highlight: Color32::from_rgb(255, 193, 7), // 琥珀色
            selected_bg: Color32::from_rgba_unmultiplied(66, 133, 244, 30),
            unselected_bg: Color32::TRANSPARENT,
        }
    }
}

impl Theme {
    /// 获取置信度对应的颜色
    pub fn confidence_color(&self, confidence: f32) -> Color32 {
        if confidence >= 0.8 {
            self.success
        } else if confidence >= 0.6 {
            self.warning
        } else {
            self.error
        }
    }
}

/// 圆角设置
pub fn default_rounding() -> Rounding {
    Rounding::same(4.0)
}

/// 按钮样式
pub fn button_style(visuals: &mut egui::Visuals) {
    visuals.widgets.inactive.rounding = default_rounding();
    visuals.widgets.hovered.rounding = default_rounding();
    visuals.widgets.active.rounding = default_rounding();
}

/// 面板边框
pub fn panel_stroke() -> Stroke {
    Stroke::new(1.0, Color32::from_gray(200))
}
