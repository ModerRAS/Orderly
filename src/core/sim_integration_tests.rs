use crate::core::executor::Executor;
use crate::core::models::{FileDescriptor, MoveSuggestion, SuggestionSource};
use crate::core::planner::Planner;
use crate::core::rule_engine::RuleEngine;
use crate::core::scanner::FileScanner;
use chrono::{TimeZone, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn make_fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap()
}

fn find_file<'a>(files: &'a [FileDescriptor], name: &str) -> FileDescriptor {
    files
        .iter()
        .find(|f| !f.is_directory && f.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("expected file not found: {name}"))
}

#[test]
fn sim_rule_to_plan_keeps_filename_and_is_deterministic() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    let output = dir.path().join("output");

    write_file(&input.join("photo.jpg"), "jpg-bytes");
    write_file(&input.join("note.txt"), "hello");

    let scanner = FileScanner::new(input.clone());
    let mut files = scanner.scan().unwrap();

    // 为了可重复性：把 modified_at 固定下来（规则模板会用到 year/month）
    for f in files.iter_mut() {
        if !f.is_directory {
            f.modified_at = make_fixed_time();
        }
    }

    let mut engine = RuleEngine::new(output.clone());
    for f in files.iter_mut() {
        if let Some(s) = engine.match_file(f) {
            f.suggested_action = Some(s);
        }
        f.selected = true;
    }

    let planner = Planner::new(output.clone(), 0.0);
    let plan = planner.generate_plan(&files);

    // photo.jpg 应该进 Pictures/2024/06，并且文件名保持不变
    let expected_dir = output.join("Pictures").join("2024").join("06");
    let op = plan
        .operations
        .iter()
        .find(|op| op.from.ends_with("photo.jpg"))
        .unwrap();

    assert_eq!(op.to, expected_dir.join("photo.jpg"));

    // note.txt 是文档，应该进 Documents/2024
    let expected_dir2 = output.join("Documents").join("2024");
    let op2 = plan
        .operations
        .iter()
        .find(|op| op.from.ends_with("note.txt"))
        .unwrap();

    assert_eq!(op2.to, expected_dir2.join("note.txt"));
}

#[test]
fn sim_planner_ignores_suggestion_filename_part() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    let output = dir.path().join("output");

    write_file(&input.join("keepname.pdf"), "pdf-bytes");

    let scanner = FileScanner::new(input.clone());
    let mut files = scanner.scan().unwrap();

    let mut f = find_file(&files, "keepname.pdf");
    f.modified_at = make_fixed_time();
    f.selected = true;

    // 模拟 AI/规则误返回了“带文件名”的 target_path
    let wrong_target = output
        .join("Documents")
        .join("2024")
        .join("renamed.pdf");

    f.suggested_action = Some(MoveSuggestion {
        target_path: wrong_target,
        reason: "simulated".to_string(),
        source: SuggestionSource::AI,
        confidence: 1.0,
    });

    files.clear();
    files.push(f);

    let planner = Planner::new(output.clone(), 0.0);
    let plan = planner.generate_plan(&files);

    assert_eq!(plan.operations.len(), 1);
    // 必须保持原文件名 keepname.pdf
    assert!(plan.operations[0].to.ends_with(PathBuf::from("keepname.pdf")));
    assert_eq!(
        plan.operations[0].to,
        output.join("Documents").join("2024").join("keepname.pdf")
    );
}

#[test]
fn sim_execute_and_rollback_roundtrip() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    let output = dir.path().join("output");
    let data = dir.path().join("data");

    write_file(&input.join("a.jpg"), "a");
    write_file(&input.join("b.txt"), "b");

    let scanner = FileScanner::new(input.clone());
    let mut files = scanner.scan().unwrap();

    for f in files.iter_mut() {
        if !f.is_directory {
            f.modified_at = make_fixed_time();
        }
        f.selected = true;
    }

    // 走一遍规则引擎生成建议
    let mut engine = RuleEngine::new(output.clone());
    for f in files.iter_mut() {
        if let Some(s) = engine.match_file(f) {
            f.suggested_action = Some(s);
        }
    }

    let planner = Planner::new(output.clone(), 0.0);
    let mut plan = planner.generate_plan(&files);

    // 执行
    let mut exec = Executor::new(data);
    let result = exec.execute(&mut plan);
    assert!(result.is_all_successful());

    // 确认文件都移动到了“目录 + 原文件名”的最终位置
    let a_target = output.join("Pictures").join("2024").join("06").join("a.jpg");
    let b_target = output.join("Documents").join("2024").join("b.txt");

    assert!(!input.join("a.jpg").exists());
    assert!(!input.join("b.txt").exists());
    assert!(a_target.exists());
    assert!(b_target.exists());

    // 回滚
    let batch_id = plan.batch_id.clone();
    let rb = exec.rollback(&batch_id);
    assert_eq!(rb.failed, 0);

    assert!(input.join("a.jpg").exists());
    assert!(input.join("b.txt").exists());
    assert!(!a_target.exists());
    assert!(!b_target.exists());
}
