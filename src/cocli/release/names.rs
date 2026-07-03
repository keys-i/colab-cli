use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseNote {
    pub title: String,
    pub sections: Vec<ReleaseSection>,
    pub animal_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSection {
    pub name: String,
    pub items: Vec<String>,
}

const NAMES: &[(&str, &str)] = &[
    ("Capybara", "Scheduler"),
    ("Otter", "Checkpoint"),
    ("Ferret", "Fleet"),
    ("Platypus", "Packet"),
    ("Stoat", "Slurp"),
    ("Pangolin", "Planner"),
    ("Quokka", "Queue"),
    ("Axolotl", "Artifact"),
    ("Wombat", "Worker"),
    ("Lemur", "Latency"),
];

pub fn release_name(version: &str, override_name: Option<&str>) -> String {
    if let Some(name) = override_name.filter(|s| !s.trim().is_empty()) {
        return format!("{version} - {name}");
    }
    let idx = stable_hash(version) as usize % NAMES.len();
    let (animal, idea) = NAMES[idx];
    format!("{version} - {animal} {idea}")
}

pub fn semver_bump(commits: &[&str], pre_1: bool) -> &'static str {
    if commits
        .iter()
        .any(|c| c.contains("BREAKING CHANGE") || c.contains("!:"))
    {
        if pre_1 { "minor" } else { "major" }
    } else if commits.iter().any(|c| c.starts_with("feat")) {
        "minor"
    } else {
        "patch"
    }
}

pub fn release_notes(version: &str, commits: &[&str]) -> ReleaseNote {
    let mut sections = [
        ("Added", Vec::new()),
        ("Changed", Vec::new()),
        ("Fixed", Vec::new()),
        ("Security", Vec::new()),
        ("Performance", Vec::new()),
        ("Docs", Vec::new()),
        ("Breaking changes", Vec::new()),
        ("Upgrade notes", Vec::new()),
    ];
    for commit in commits {
        let target = if commit.contains("BREAKING CHANGE") || commit.contains("!:") {
            "Breaking changes"
        } else if commit.starts_with("feat") {
            "Added"
        } else if commit.starts_with("fix") {
            "Fixed"
        } else if commit.starts_with("security") {
            "Security"
        } else if commit.starts_with("perf") {
            "Performance"
        } else if commit.starts_with("docs") {
            "Docs"
        } else {
            "Changed"
        };
        if let Some((_, items)) = sections.iter_mut().find(|(name, _)| *name == target) {
            items.push(clean_commit(commit));
        }
    }
    ReleaseNote {
        title: release_name(version, None),
        sections: sections
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(name, items)| ReleaseSection {
                name: name.into(),
                items,
            })
            .collect(),
        animal_note: "Animal note: Quokka Queue did not panic. It merely judged the scheduler."
            .into(),
    }
}

fn clean_commit(commit: &str) -> String {
    commit
        .split_once(':')
        .map_or(commit, |(_, rest)| rest)
        .trim()
        .to_string()
}

fn stable_hash(s: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for b in s.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_name_is_stable_and_overridable() {
        assert_eq!(release_name("v0.5.1", None), release_name("v0.5.1", None));
        assert_eq!(
            release_name("v0.4.3", Some("Otter Checkpoint")),
            "v0.4.3 - Otter Checkpoint"
        );
    }

    #[test]
    fn semver_mapping_follows_conventional_commits() {
        assert_eq!(semver_bump(&["fix: bug"], true), "patch");
        assert_eq!(semver_bump(&["feat: new"], true), "minor");
        assert_eq!(semver_bump(&["feat!: break"], false), "major");
        assert_eq!(semver_bump(&["feat!: break"], true), "minor");
    }

    #[test]
    fn release_notes_group_sections() {
        let notes = release_notes("v0.6.0", &["feat: slurp", "fix: redaction", "docs: fleet"]);
        let names: Vec<_> = notes.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Added"));
        assert!(names.contains(&"Fixed"));
        assert!(names.contains(&"Docs"));
    }
}
