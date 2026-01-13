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
use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
        Box::new(|cc| Ok(Box::new(ui::app::OrderlyApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("GUI启动失败: {}", e))?;

    Ok(())
}
