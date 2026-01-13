//! Orderly - AI增强型本地文件整理工具
//! 
//! 核心设计原则：
//! - AI负责理解，不负责破坏
//! - 所有学习必须显性
//! - 目录边界高于一切
//! - 人类永远有最终裁决权

pub mod core;
pub mod ui;
pub mod storage;

use anyhow::Result;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 配置中文字体
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // 尝试加载系统中文字体
    // Windows: 微软雅黑
    let font_paths = [
        "C:/Windows/Fonts/msyh.ttc",      // 微软雅黑
        "C:/Windows/Fonts/simsun.ttc",    // 宋体
        "C:/Windows/Fonts/simhei.ttf",    // 黑体
    ];

    let mut font_loaded = false;
    for path in &font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "chinese_font".to_owned(),
                FontData::from_owned(font_data).into(),
            );
            
            // 将中文字体设为首选
            fonts.families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese_font".to_owned());
            
            fonts.families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "chinese_font".to_owned());
            
            font_loaded = true;
            tracing::info!("已加载中文字体: {}", path);
            break;
        }
    }

    if !font_loaded {
        tracing::warn!("未能加载中文字体，界面可能显示乱码");
    }

    ctx.set_fonts(fonts);
}

fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("启动 Orderly - AI增强型文件整理工具");

    // 启动GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Orderly - AI增强型文件整理工具"),
        ..Default::default()
    };

    eframe::run_native(
        "Orderly",
        options,
        Box::new(|cc| {
            // 加载中文字体
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ui::app::OrderlyApp::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI启动失败: {}", e))?;

    Ok(())
}
