use super::width::{truncate_end, truncate_middle};

pub fn render_table(headers: &[&str], rows: &[Vec<String>], max_width: usize) -> String {
    if headers.is_empty() {
        return String::new();
    }
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| visible_width(h)).collect();
    for row in rows {
        for (idx, value) in row.iter().take(cols).enumerate() {
            widths[idx] = widths[idx].max(visible_width(value));
        }
    }

    let gutters = cols.saturating_sub(1) * 2;
    let available = max_width.saturating_sub(gutters).max(cols);
    while widths.iter().sum::<usize>() > available {
        if let Some((idx, _)) = widths.iter().enumerate().max_by_key(|(_, width)| *width) {
            if widths[idx] <= 8 {
                break;
            }
            widths[idx] -= 1;
        } else {
            break;
        }
    }

    let mut out = String::new();
    write_row(&mut out, headers.iter().copied(), &widths);
    write_separator(&mut out, &widths);
    for row in rows {
        write_row(&mut out, row.iter().map(String::as_str), &widths);
    }
    out
}

fn write_row<'a>(out: &mut String, cells: impl Iterator<Item = &'a str>, widths: &[usize]) {
    let cells: Vec<&str> = cells.collect();
    for (idx, width) in widths.iter().enumerate() {
        if idx > 0 {
            out.push_str("  ");
        }
        let cell = cells.get(idx).copied().unwrap_or_default();
        let value = truncate_cell(cell, *width, idx + 1 == widths.len());
        out.push_str(&value);
        let pad = width.saturating_sub(visible_width(&value));
        out.push_str(&" ".repeat(pad));
    }
    out.push('\n');
}

fn write_separator(out: &mut String, widths: &[usize]) {
    for (idx, width) in widths.iter().enumerate() {
        if idx > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*width));
    }
    out.push('\n');
}

fn truncate_cell(cell: &str, width: usize, end: bool) -> String {
    if visible_width(cell) <= width {
        return cell.to_string();
    }
    let plain = strip_ansi(cell);
    if end {
        truncate_end(&plain, width)
    } else {
        truncate_middle(&plain, width)
    }
}

fn visible_width(value: &str) -> usize {
    strip_ansi(value).chars().count()
}

fn strip_ansi(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render_table;

    #[test]
    fn renders_requested_widths() {
        let rows = vec![vec![
            "very.long.endpoint.example.proxy.googleusercontent.com".to_string(),
            "ready".to_string(),
            "summary text".to_string(),
        ]];
        for width in [60, 80, 100, 140] {
            let table = render_table(&["Endpoint", "State", "Summary"], &rows, width);
            assert!(table.lines().all(|line| line.chars().count() <= width));
        }
    }

    #[test]
    fn renders_ansi_without_width_bloat() {
        let rows = vec![vec!["\x1b[32mready\x1b[0m".to_string(), "ok".to_string()]];
        let table = render_table(&["State", "Summary"], &rows, 20);
        assert!(table.lines().all(|line| line.chars().count() <= 32));
        assert!(!table.contains('\t'));
    }
}
