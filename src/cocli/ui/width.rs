pub fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(cols, _)| usize::from(cols))
        .unwrap_or(100)
        .max(20)
}

pub fn truncate_middle(value: &str, max: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        return value.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    if max <= 3 {
        return "…".repeat(max);
    }
    let left = (max - 1) / 2;
    let right = max - 1 - left;
    let start: String = chars.iter().take(left).collect();
    let end: String = chars.iter().skip(chars.len() - right).collect();
    format!("{start}…{end}")
}

pub fn truncate_end(value: &str, max: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        return value.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let prefix: String = chars.iter().take(max - 1).collect();
    format!("{prefix}…")
}

#[cfg(test)]
mod tests {
    use super::truncate_middle;

    #[test]
    fn truncates_safely() {
        assert_eq!(truncate_middle("abcdef", 4), "a…ef");
        assert_eq!(truncate_middle("abc", 4), "abc");
        assert_eq!(super::truncate_end("abcdef", 4), "abc…");
    }
}
