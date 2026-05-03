// SPDX-FileCopyrightText: 2026 aptu-coder contributors
// SPDX-License-Identifier: Apache-2.0
//! File read utilities for the edit tools.

use crate::types::{EditFileOutput, ReadFileOutput, WriteFileOutput};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid range: start ({start}) > end ({end}); file has {total} lines")]
    InvalidRange {
        start: usize,
        end: usize,
        total: usize,
    },
    #[error("path is a directory, not a file: {0}")]
    NotAFile(PathBuf),
    #[error(
        "old_text not found in {path} — verify the text matches exactly, including whitespace and newlines"
    )]
    NotFound { path: String },
    #[error(
        "old_text appears {count} times in {path} — make old_text longer and more specific to uniquely identify the block"
    )]
    Ambiguous { count: usize, path: String },
}

pub fn read_file_range(
    path: &Path,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> Result<ReadFileOutput, EditError> {
    if path.is_dir() {
        return Err(EditError::NotAFile(path.to_path_buf()));
    }
    let raw = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();
    if total == 0 {
        return Ok(ReadFileOutput {
            path: path.display().to_string(),
            total_lines: 0,
            start_line: 0,
            end_line: 0,
            content: String::new(),
        });
    }
    let start = start_line.unwrap_or(1).max(1).min(total.max(1));
    let end = end_line.unwrap_or(total).min(total).max(1);
    if start > end {
        return Err(EditError::InvalidRange { start, end, total });
    }
    let width = end.to_string().len();
    let content = lines[start - 1..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>width$}: {}", start + i, line, width = width))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(ReadFileOutput {
        path: path.display().to_string(),
        total_lines: total,
        start_line: start,
        end_line: end,
        content,
    })
}

pub fn write_file_content(path: &Path, content: &str) -> Result<WriteFileOutput, EditError> {
    if path.is_dir() {
        return Err(EditError::NotAFile(path.to_path_buf()));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(WriteFileOutput {
        path: path.display().to_string(),
        bytes_written: content.len(),
    })
}

pub fn edit_file_replace(
    path: &Path,
    old_text: &str,
    new_text: &str,
) -> Result<EditFileOutput, EditError> {
    if path.is_dir() {
        return Err(EditError::NotAFile(path.to_path_buf()));
    }
    let content = std::fs::read_to_string(path)?;
    let count = content.matches(old_text).count();
    match count {
        0 => {
            return Err(EditError::NotFound {
                path: path.display().to_string(),
            });
        }
        1 => {}
        n => {
            return Err(EditError::Ambiguous {
                count: n,
                path: path.display().to_string(),
            });
        }
    }
    let bytes_before = content.len();
    let updated = content.replacen(old_text, new_text, 1);
    let bytes_after = updated.len();
    std::fs::write(path, &updated)?;
    Ok(EditFileOutput {
        path: path.display().to_string(),
        bytes_before,
        bytes_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_temp_file(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        f
    }

    #[test]
    fn test_read_full_file() {
        let f = make_temp_file("line1\nline2\nline3\n");
        let out = read_file_range(f.path(), None, None).unwrap();
        assert_eq!(out.total_lines, 3);
        assert_eq!(out.start_line, 1);
        assert_eq!(out.end_line, 3);
        assert!(out.content.contains("line1"));
        assert!(out.content.contains("line3"));
    }

    #[test]
    fn test_read_partial_range() {
        let f = make_temp_file("a\nb\nc\nd\ne\n");
        let out = read_file_range(f.path(), Some(2), Some(4)).unwrap();
        assert_eq!(out.start_line, 2);
        assert_eq!(out.end_line, 4);
        assert!(out.content.contains("b"));
        assert!(out.content.contains("d"));
        assert!(!out.content.contains("a"));
        assert!(!out.content.contains("e"));
    }

    #[test]
    fn test_read_invalid_range() {
        let f = make_temp_file("a\nb\nc\n");
        let err = read_file_range(f.path(), Some(3), Some(1)).unwrap_err();
        assert!(matches!(err, EditError::InvalidRange { .. }));
    }

    #[test]
    fn test_read_clamped_range() {
        let f = make_temp_file("x\ny\nz\n");
        // end_line beyond total should clamp
        let out = read_file_range(f.path(), Some(1), Some(999)).unwrap();
        assert_eq!(out.end_line, 3);
        assert_eq!(out.total_lines, 3);
    }

    #[test]
    fn test_read_empty_file() {
        let f = make_temp_file("");
        let out = read_file_range(f.path(), None, None).unwrap();
        assert_eq!(out.total_lines, 0);
        assert_eq!(out.content, "");
    }

    #[test]
    fn write_file_content_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");
        let result = write_file_content(&path, "hello world").unwrap();
        assert_eq!(result.bytes_written, 11);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
    }

    #[test]
    fn write_file_content_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.txt");
        std::fs::write(&path, "old content").unwrap();
        let result = write_file_content(&path, "new content").unwrap();
        assert_eq!(result.bytes_written, 11);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn write_file_content_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("c.txt");
        let result = write_file_content(&path, "nested").unwrap();
        assert_eq!(result.bytes_written, 6);
        assert!(path.exists());
    }

    #[test]
    fn write_file_content_directory_guard() {
        let dir = tempfile::tempdir().unwrap();
        let err = write_file_content(dir.path(), "content").unwrap_err();
        assert!(matches!(err, EditError::NotAFile(_)));
    }

    #[test]
    fn edit_file_replace_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        std::fs::write(&path, "foo bar baz").unwrap();
        let result = edit_file_replace(&path, "bar", "qux").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "foo qux baz");
        assert_eq!(result.bytes_before, 11);
        assert_eq!(result.bytes_after, 11);
    }

    #[test]
    fn edit_file_replace_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        std::fs::write(&path, "foo bar baz").unwrap();
        let err = edit_file_replace(&path, "missing", "x").unwrap_err();
        assert!(matches!(err, EditError::NotFound { .. }));
    }

    #[test]
    fn edit_file_replace_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        std::fs::write(&path, "foo foo baz").unwrap();
        let err = edit_file_replace(&path, "foo", "x").unwrap_err();
        assert!(matches!(err, EditError::Ambiguous { count: 2, .. }));
    }

    #[test]
    fn edit_file_replace_directory_guard() {
        let dir = tempfile::tempdir().unwrap();
        let err = edit_file_replace(dir.path(), "old", "new").unwrap_err();
        assert!(matches!(err, EditError::NotAFile(_)));
    }
}
