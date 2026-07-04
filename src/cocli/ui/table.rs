use super::width::{truncate_end, truncate_middle};

pub fn render_table(headers: &[&str], rows: &[Vec<String>], max_width: usize) -> String {
    if headers.is_empty() {
        return String::new();
    }
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (idx, value) in row.iter().take(cols).enumerate() {
            widths[idx] = widths[idx].max(value.chars().count());
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
        let value = if idx + 1 == widths.len() {
            truncate_end(cell, *width)
        } else {
            truncate_middle(cell, *width)
        };
        out.push_str(&format!("{value:<width$}"));
    }
    out.push('\n');
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
}
