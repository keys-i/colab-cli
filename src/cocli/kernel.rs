use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelLanguage {
    Python,
    Julia,
    R,
    Bash,
    Scala,
    Unknown(String),
}

impl KernelLanguage {
    pub fn detect(value: &str) -> Self {
        let raw = value.trim();
        match raw.to_ascii_lowercase().as_str() {
            "python" | "python3" => Self::Python,
            "julia" => Self::Julia,
            "r" | "ir" => Self::R,
            "bash" | "sh" => Self::Bash,
            "scala" => Self::Scala,
            "" => Self::Unknown("unknown".to_string()),
            _ => Self::Unknown(raw.to_string()),
        }
    }

    pub fn as_config_value(&self) -> String {
        match self {
            Self::Python => "python".to_string(),
            Self::Julia => "julia".to_string(),
            Self::R => "r".to_string(),
            Self::Bash => "bash".to_string(),
            Self::Scala => "scala".to_string(),
            Self::Unknown(value) => value.clone(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Python => "Python".to_string(),
            Self::Julia => "Julia".to_string(),
            Self::R => "R".to_string(),
            Self::Bash => "Bash".to_string(),
            Self::Scala => "Scala".to_string(),
            Self::Unknown(value) => {
                let mut chars = value.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => "Unknown".to_string(),
                }
            }
        }
    }

    pub fn repl_prompt(&self) -> &'static str {
        match self {
            Self::Python => ">>> ",
            Self::Julia => "julia> ",
            Self::R => "R> ",
            _ => "> ",
        }
    }

    pub fn continuation_prompt(&self) -> &'static str {
        match self {
            Self::Python => "... ",
            _ => "  ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelInfoSummary {
    pub language: KernelLanguage,
    pub version: Option<String>,
}

impl KernelInfoSummary {
    pub fn unknown() -> Self {
        Self {
            language: KernelLanguage::Unknown("unknown".to_string()),
            version: None,
        }
    }

    pub fn from_language_info(language: Option<&str>, version: Option<&str>) -> Self {
        Self {
            language: language
                .map(KernelLanguage::detect)
                .unwrap_or_else(|| KernelLanguage::Unknown("unknown".to_string())),
            version: version.map(str::to_string).filter(|v| !v.is_empty()),
        }
    }

    pub fn display(&self) -> String {
        match &self.version {
            Some(version) => format!("{} {version}", self.language.display_name()),
            None => self.language.display_name(),
        }
    }
}

pub fn package_code(language: &KernelLanguage, action: &str, args: &[String]) -> Option<String> {
    match language {
        KernelLanguage::Python => python_pkg_code(action, args),
        KernelLanguage::Julia => julia_pkg_code(action, args),
        KernelLanguage::R => r_pkg_code(action, args),
        _ => None,
    }
}

fn python_pkg_code(action: &str, args: &[String]) -> Option<String> {
    let joined = shell_words(args);
    let code = match action {
        "add" => format!(
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'install', *{joined:?}])"
        ),
        "remove" => format!(
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'uninstall', '-y', *{joined:?}])"
        ),
        "list" => {
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'list'])"
                .to_string()
        }
        "status" => {
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'list'])"
                .to_string()
        }
        "update" => format!(
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'install', '--upgrade', *{joined:?}])"
        ),
        "restore" => format!(
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'install', '-r', {}])",
            python_string(
                args.first()
                    .map(String::as_str)
                    .unwrap_or("requirements.txt")
            )
        ),
        "check" => {
            "import sys, subprocess\nsubprocess.check_call([sys.executable, '-m', 'pip', 'check'])"
                .to_string()
        }
        _ => return None,
    };
    Some(code)
}

fn julia_pkg_code(action: &str, args: &[String]) -> Option<String> {
    let pkgs = julia_vec(args);
    let code = match action {
        "add" => format!("import Pkg\nPkg.add({pkgs})"),
        "remove" => format!("import Pkg\nPkg.rm({pkgs})"),
        "list" | "status" => "import Pkg\nPkg.status()".to_string(),
        "update" => "import Pkg\nPkg.update()".to_string(),
        "restore" => "import Pkg\nPkg.instantiate()".to_string(),
        "check" => "import Pkg\nPkg.status()".to_string(),
        "precompile" => "import Pkg\nPkg.precompile()".to_string(),
        "test" => "import Pkg\nPkg.test()".to_string(),
        _ => return None,
    };
    Some(code)
}

fn r_pkg_code(action: &str, args: &[String]) -> Option<String> {
    let pkgs = r_vec(args);
    let code = match action {
        "add" => format!("install.packages({pkgs})"),
        "remove" => format!("remove.packages({pkgs})"),
        "list" => "installed.packages()[, c('Package', 'Version')]".to_string(),
        "status" => "sessionInfo()".to_string(),
        "update" => "update.packages(ask = FALSE)".to_string(),
        "restore" => "if (requireNamespace('renv', quietly = TRUE)) renv::restore(prompt = FALSE) else stop('renv is not installed')".to_string(),
        "snapshot" => "if (requireNamespace('renv', quietly = TRUE)) renv::snapshot(prompt = FALSE) else stop('renv is not installed')".to_string(),
        "check" => "sessionInfo()".to_string(),
        _ => return None,
    };
    Some(code)
}

fn shell_words(args: &[String]) -> Vec<String> {
    args.to_vec()
}

fn python_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"requirements.txt\"".to_string())
}

fn julia_vec(args: &[String]) -> String {
    format!(
        "[{}]",
        args.iter()
            .map(|arg| python_string(arg))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn r_vec(args: &[String]) -> String {
    if args.len() == 1 {
        return python_string(&args[0]);
    }
    format!("c({})", julia_vec(args).trim_matches(['[', ']']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_kernel_languages() {
        assert_eq!(KernelLanguage::detect("python"), KernelLanguage::Python);
        assert_eq!(KernelLanguage::detect("julia"), KernelLanguage::Julia);
        assert_eq!(KernelLanguage::detect("R"), KernelLanguage::R);
        assert_eq!(
            KernelLanguage::detect("octave"),
            KernelLanguage::Unknown("octave".to_string())
        );
    }

    #[test]
    fn package_routing_is_language_specific() {
        assert!(
            package_code(&KernelLanguage::Python, "add", &["numpy".into()])
                .unwrap()
                .contains("pip")
        );
        assert!(
            package_code(&KernelLanguage::Julia, "add", &["CSV".into()])
                .unwrap()
                .contains("Pkg.add")
        );
        assert!(
            package_code(&KernelLanguage::R, "add", &["dplyr".into()])
                .unwrap()
                .contains("install.packages")
        );
        assert!(package_code(&KernelLanguage::Unknown("x".into()), "add", &[]).is_none());
    }
}
