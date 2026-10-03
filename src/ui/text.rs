use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 依顯示寬度截斷，超出時以 `…` 結尾。
pub fn clip(text: &str, width: u16) -> String {
    if UnicodeWidthStr::width(text) <= usize::from(width) {
        return text.into();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let next = UnicodeWidthStr::width(grapheme);
        if used + next > usize::from(width - 1) {
            break;
        }
        result.push_str(grapheme);
        used += next;
    }
    result.push('…');
    result
}

/// 依顯示寬度自動換行，保留原有換行。
pub fn fold(text: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let mut output = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        if grapheme == "\n" {
            output.push('\n');
            used = 0;
            continue;
        }
        let size = UnicodeWidthStr::width(grapheme);
        if used + size > usize::from(width) {
            output.push('\n');
            used = 0;
        }
        output.push_str(grapheme);
        used += size;
    }
    output
}
