use std::fs;
use std::path::PathBuf;

fn attachments_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    let dir = home.join(".note").join("attachments");
    fs::create_dir_all(&dir).expect("Could not create ~/.note/attachments");
    dir
}

pub fn paste_image_from_clipboard() -> anyhow::Result<PathBuf> {
    let mut clipboard = arboard::Clipboard::new()?;
    let img_data = clipboard.get_image()?;
    let img = image::RgbaImage::from_raw(
        img_data.width as u32,
        img_data.height as u32,
        img_data.bytes.into_owned(),
    ).ok_or_else(|| anyhow::anyhow!("Invalid image data"))?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let path = attachments_dir().join(format!("screenshot_{ts}.png"));
    img.save(&path)?;
    Ok(path)
}

pub fn find_image_lines(lines: &[String]) -> Vec<(usize, PathBuf)> {
    let mut result = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(path) = parse_image_path(line) {
            if path.exists() {
                result.push((i, path));
            }
        }
    }
    result
}

fn parse_image_path(line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if !trimmed.starts_with("![") { return None; }
    let paren_start = trimmed.find("](")? + 2;
    let paren_end = trimmed[paren_start..].find(')')? + paren_start;
    let path_str = &trimmed[paren_start..paren_end];
    if path_str.starts_with("http://") || path_str.starts_with("https://") {
        return None;
    }
    Some(PathBuf::from(path_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_path() {
        assert_eq!(
            parse_image_path("![screenshot](/home/user/.note/attachments/test.png"),
            None, // missing closing paren
        );
        let p = parse_image_path("![screenshot](/home/user/.note/attachments/test.png)");
        assert_eq!(p, Some(PathBuf::from("/home/user/.note/attachments/test.png")));

        assert_eq!(parse_image_path("![img](https://example.com/img.png)"), None);
        assert_eq!(parse_image_path("just some text"), None);
    }

    #[test]
    fn test_find_image_lines_no_existing_files() {
        let lines = vec![
            "# Title".to_string(),
            "![img](/nonexistent/path.png)".to_string(),
        ];
        let result = find_image_lines(&lines);
        assert!(result.is_empty()); // file doesn't exist
    }
}
