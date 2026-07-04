pub mod color;
pub mod error;
pub mod palette;
pub mod panel;
pub mod progress;
pub mod prompt;
pub mod table;
pub mod width;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use tabled::settings::Style;
use tabled::{Table, Tabled};

use crate::cocli::session::model::CcuInfo;
use crate::cocli::session::store::StoredServer;

#[derive(Clone, Copy)]
pub struct Ui {
    pub quiet: bool,
    pub plain: bool,
    pub interactive: bool,
}

impl Ui {
    pub fn new(quiet: bool, plain: bool, interactive: bool) -> Self {
        Self {
            quiet,
            plain,
            interactive,
        }
    }

    pub fn spinner(&self, msg: &str) -> Option<ProgressBar> {
        if self.quiet || !self.interactive || crate::cocli::debug::enabled(1) {
            return None;
        }
        let pb = ProgressBar::new_spinner();
        if let Ok(style) = ProgressStyle::with_template("{spinner:.cyan} {msg}") {
            pb.set_style(style.tick_strings(&[
                "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}",
                "\u{2827}", "\u{2807}", "\u{280f}",
            ]));
        }
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    }

    pub fn spinner_done(pb: Option<ProgressBar>, msg: &str) {
        if let Some(pb) = pb {
            pb.finish_with_message(msg.to_string());
        }
    }

    pub fn spinner_fail(pb: Option<ProgressBar>, msg: &str) {
        if let Some(pb) = pb {
            pb.finish_with_message(format!("\u{2717} {msg}"));
        }
    }

    pub fn success(&self, msg: &str) {
        if self.quiet || self.plain {
            println!("{msg}");
        } else {
            println!("{} {msg}", "\u{2713}".green().bold());
        }
    }

    pub fn info(&self, msg: &str) {
        if self.quiet || self.plain {
            println!("{msg}");
        } else {
            println!("{} {msg}", "\u{00b7}".dimmed());
        }
    }

    pub fn warn(&self, msg: &str) {
        if self.quiet || self.plain {
            eprintln!("warning: {msg}");
        } else {
            eprintln!("{} {msg}", "\u{26a0}".yellow().bold());
        }
    }

    pub fn error(&self, msg: &str) {
        if self.quiet || self.plain {
            eprintln!("error: {msg}");
        } else {
            eprintln!("{} {msg}", "error:".red().bold());
        }
    }

    pub fn print_auth_status(&self, email: &str, name: &str) {
        if self.quiet {
            println!("signed_in\t{email}\t{name}");
        } else {
            println!(
                "{} Signed in as {} ({})",
                "\u{2713}".green().bold(),
                name.bold(),
                email.dimmed()
            );
        }
    }

    pub fn print_auth_not_signed_in(&self) {
        if self.quiet {
            println!("not_signed_in");
        } else {
            println!(
                "{} Not signed in. Run {} to authenticate.",
                "\u{00b7}".dimmed(),
                "colab-cli auth login".cyan().bold()
            );
        }
    }

    pub fn print_server_list(&self, servers: &[StoredServer]) {
        if servers.is_empty() {
            if self.quiet {
                println!("no_servers");
            } else {
                println!(
                    "{} No servers assigned. Run {} to assign one.",
                    "\u{00b7}".dimmed(),
                    "colab-cli session new".cyan().bold()
                );
            }
            return;
        }

        if self.quiet {
            println!("name\tvariant\taccelerator\tshape\tendpoint\ttoken_expires_at\tassigned_at");
            for s in servers {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    s.label,
                    s.variant,
                    s.accelerator.as_deref().unwrap_or(""),
                    s.shape,
                    s.endpoint,
                    s.token_expires_at
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%dT%H:%M:%S%:z"),
                    s.date_assigned
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%dT%H:%M:%S%:z"),
                );
            }
            return;
        }

        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Type")]
            variant: String,
            #[tabled(rename = "Accelerator")]
            accelerator: String,
            #[tabled(rename = "Shape")]
            shape: String,
            #[tabled(rename = "Endpoint")]
            endpoint: String,
            #[tabled(rename = "Token Expires")]
            token_expires: String,
            #[tabled(rename = "Assigned")]
            assigned: String,
        }

        let now = chrono::Utc::now();
        let rows: Vec<Row> = servers
            .iter()
            .map(|s| {
                let remaining = s.token_expires_at - now;
                let expires = if remaining.num_minutes() < 10 {
                    format!("{}m \u{26a0}", remaining.num_minutes())
                } else {
                    format!("{}m", remaining.num_minutes())
                };
                Row {
                    name: s.label.clone(),
                    variant: s.variant.display_name().to_string(),
                    accelerator: s.accelerator.clone().unwrap_or_else(|| "\u{2014}".into()),
                    shape: s.shape.display_name().to_string(),
                    endpoint: truncate(&s.endpoint, 32),
                    token_expires: expires,
                    assigned: s
                        .date_assigned
                        .with_timezone(&chrono::Local)
                        .format("%b %d %H:%M")
                        .to_string(),
                }
            })
            .collect();

        let mut table = Table::new(rows);
        table.with(Style::sharp());
        println!("{table}");
    }

    pub fn print_server_status(&self, s: &StoredServer) {
        let now = chrono::Utc::now();
        let remaining = s.token_expires_at - now;

        if self.quiet {
            println!("name\t{}", s.label);
            println!("variant\t{}", s.variant);
            println!("accelerator\t{}", s.accelerator.as_deref().unwrap_or(""));
            println!("shape\t{}", s.shape);
            println!("endpoint\t{}", s.endpoint);
            println!("token_expires_in_minutes\t{}", remaining.num_minutes());
            println!(
                "assigned_at\t{}",
                s.date_assigned
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%dT%H:%M:%S%:z")
            );
            return;
        }

        let expiry_str = if remaining.num_minutes() < 10 {
            format!("{}m {}", remaining.num_minutes(), "(refresh soon)".yellow())
        } else {
            format!("{}m", remaining.num_minutes())
        };

        println!("{}", "Server".bold());
        kv("Name", &s.label.bold().to_string());
        kv(
            "Accelerator",
            s.accelerator
                .as_deref()
                .unwrap_or_else(|| s.variant.display_name()),
        );
        kv("Shape", s.shape.display_name());
        kv("Endpoint", &s.endpoint.dimmed().to_string());
        kv("Token", &format!("expires in {expiry_str}"));
        kv(
            "Assigned",
            &s.date_assigned
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        );
    }

    pub fn print_usage(&self, info: &CcuInfo) {
        if self.quiet {
            println!("balance\t{:.2}", info.current_balance);
            println!("rate_hourly\t{:.2}", info.consumption_rate_hourly);
            println!("assignments\t{}", info.assignments_count);
            println!("gpus\t{}", info.eligible_gpus.join(","));
            println!("tpus\t{}", info.eligible_tpus.join(","));
            return;
        }

        println!("{}", "Compute Usage".bold());
        kv("Balance", &format!("{:.2} CCU", info.current_balance));
        kv(
            "Burn rate",
            &format!("{:.2} CCU/hr", info.consumption_rate_hourly),
        );
        kv("Active servers", &info.assignments_count.to_string());
    }

    pub fn print_accelerators(&self, info: &CcuInfo) {
        if self.quiet {
            println!("balance\t{:.2}", info.current_balance);
            println!("rate_hourly\t{:.2}", info.consumption_rate_hourly);
            println!("type\tmodel\trate_ccu_per_hour\tstatus");
            println!("CPU\t-\t{:.2}\tonline", ccu_rate("CPU", ""));
            for gpu in &info.eligible_gpus {
                println!("GPU\t{gpu}\t{:.2}\tonline", ccu_rate("GPU", gpu));
            }
            for tpu in &info.eligible_tpus {
                println!("TPU\t{tpu}\t{:.2}\tonline", ccu_rate("TPU", tpu));
            }
            return;
        }

        #[derive(Tabled)]
        struct AccelRow {
            #[tabled(rename = "Type")]
            kind: String,
            #[tabled(rename = "Model")]
            model: String,
            #[tabled(rename = "~CCU/hr")]
            rate: String,
            #[tabled(rename = "Status")]
            status: String,
        }

        let mut rows = vec![AccelRow {
            kind: "CPU".to_string(),
            model: "\u{2014}".to_string(),
            rate: format!("{:.2}", ccu_rate("CPU", "")),
            status: "online".green().to_string(),
        }];
        for gpu in &info.eligible_gpus {
            rows.push(AccelRow {
                kind: "GPU".to_string(),
                model: gpu.clone(),
                rate: format!("{:.2}", ccu_rate("GPU", gpu)),
                status: "online".green().to_string(),
            });
        }
        for tpu in &info.eligible_tpus {
            rows.push(AccelRow {
                kind: "TPU".to_string(),
                model: tpu.clone(),
                rate: format!("{:.2}", ccu_rate("TPU", tpu)),
                status: "online".green().to_string(),
            });
        }

        let mut table = Table::new(rows);
        table.with(Style::sharp());
        println!("{table}");

        println!();
        println!("{}", "Compute Units".bold());
        kv("Balance", &format!("{:.2} CCU", info.current_balance));
        kv(
            "Usage rate",
            &format!(
                "~{:.2} CCU/hr ({} active server{})",
                info.consumption_rate_hourly,
                info.assignments_count,
                if info.assignments_count == 1 { "" } else { "s" }
            ),
        );
        println!();
        println!(
            "{}",
            "Rates are approximate and may vary by region / availability.".dimmed()
        );
    }

    pub fn print_system_info(&self, server_name: &str, raw: &str) {
        let mut sections = std::collections::HashMap::new();
        let mut current_key: Option<String> = None;
        let mut buf = String::new();
        for line in raw.lines() {
            if let Some(tag) = line.strip_prefix("<<<").and_then(|s| s.strip_suffix(">>>")) {
                if let Some(k) = current_key.take() {
                    sections.insert(k, buf.trim().to_string());
                }
                current_key = Some(tag.to_string());
                buf.clear();
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        if let Some(k) = current_key.take() {
            sections.insert(k, buf.trim().to_string());
        }

        let get = |k: &str| sections.get(k).cloned().unwrap_or_default();

        if self.quiet {
            println!("server\t{server_name}");
            println!("kernel\t{}", get("UNAME"));
            println!("cpu\t{}", get("CPU").replace('\n', " "));
            println!("mem\t{}", get("MEM"));
            println!("disk\t{}", get("DISK"));
            println!("gpu\t{}", get("GPU").replace('\n', " | "));
            println!("uptime\t{}", get("UPTIME"));
            return;
        }

        println!(
            "{} {}",
            "System".bold(),
            format!("({server_name})").dimmed()
        );
        println!();

        if !get("UNAME").is_empty() {
            kv("Kernel", &get("UNAME"));
        }

        let cpu = get("CPU");
        let cpu_lines: Vec<&str> = cpu.lines().collect();
        if cpu_lines.len() >= 2 {
            kv(
                "CPU",
                &format!("{} ({} cores)", cpu_lines[1].trim(), cpu_lines[0].trim()),
            );
        } else if !cpu_lines.is_empty() {
            kv("CPU", cpu_lines[0]);
        }

        let mem = get("MEM");
        let mem_parts: Vec<&str> = mem.split('\t').collect();
        if mem_parts.len() >= 3 {
            let used = mem_parts[1].trim();
            let total = mem_parts[0].trim();
            kv("Memory", &format!("{used} / {total} used"));
        }

        let disk = get("DISK");
        let disk_parts: Vec<&str> = disk.split('\t').collect();
        if disk_parts.len() >= 4 {
            kv(
                "Disk (/)",
                &format!(
                    "{} / {} used ({})",
                    disk_parts[1].trim(),
                    disk_parts[0].trim(),
                    disk_parts[3].trim()
                ),
            );
        }

        let gpu = get("GPU");
        if gpu.trim() != "none" && !gpu.is_empty() {
            for line in gpu.lines() {
                kv("GPU", line.trim());
            }
        } else {
            kv("GPU", &"none".dimmed().to_string());
        }

        if !get("UPTIME").is_empty() {
            kv("Uptime", &get("UPTIME"));
        }
    }
}

fn kv(key: &str, value: &str) {
    println!("  {:<18} {value}", format!("{key}:").dimmed());
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max - 1])
    }
}

/// Approximate CCU/hour burn rate for a given accelerator. The Colab API does
/// not expose these; values below are derived from the public pricing page
/// and may drift — shown as "~" in the UI.
pub fn ccu_rate(kind: &str, model: &str) -> f64 {
    match (kind, model) {
        ("CPU", _) => 0.08,
        ("GPU", "T4") => 1.76,
        ("GPU", "L4") => 4.82,
        ("GPU", "V100") => 4.91,
        ("GPU", "A100") => 11.77,
        ("GPU", "H100") => 14.43,
        ("GPU", _) => 2.00,
        ("TPU", "v2-8") => 1.96,
        ("TPU", "v5e-1") => 2.20,
        ("TPU", _) => 2.00,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("abc", 10), "abc");
    }

    #[test]
    fn truncate_long_string_ellipsized() {
        let t = truncate("abcdefghij", 5);
        assert!(t.ends_with('\u{2026}'));
        assert_eq!(t.chars().count(), 5);
    }

    #[test]
    fn ccu_rate_known_gpus() {
        assert!((ccu_rate("GPU", "T4") - 1.76).abs() < 1e-9);
        assert!((ccu_rate("GPU", "A100") - 11.77).abs() < 1e-9);
        assert!((ccu_rate("GPU", "L4") - 4.82).abs() < 1e-9);
    }

    #[test]
    fn ccu_rate_unknown_gpu_has_fallback() {
        assert_eq!(ccu_rate("GPU", "Quantum9000"), 2.00);
    }

    #[test]
    fn ccu_rate_cpu() {
        assert!(ccu_rate("CPU", "") > 0.0);
    }
}
