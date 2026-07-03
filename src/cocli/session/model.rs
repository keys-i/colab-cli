use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
}

pub fn find_session<'a>(sessions: &'a [SessionSummary], name: &str) -> Option<&'a SessionSummary> {
    sessions.iter().find(|s| s.name == name || s.id == name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Cpu,
    Gpu,
    Tpu,
}

impl<'de> serde::Deserialize<'de> for Variant {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        use serde::de::{self, Visitor};
        struct V;
        impl Visitor<'_> for V {
            type Value = Variant;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "variant as string (\"DEFAULT\"/\"GPU\"/\"TPU\") or integer (0/1/2)"
                )
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Variant, E> {
                match v {
                    "DEFAULT" | "default" | "cpu" => Ok(Variant::Cpu),
                    "GPU" | "gpu" => Ok(Variant::Gpu),
                    "TPU" | "tpu" => Ok(Variant::Tpu),
                    other => Err(E::unknown_variant(other, &["DEFAULT", "GPU", "TPU"])),
                }
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Variant, E> {
                match v {
                    0 => Ok(Variant::Cpu),
                    1 => Ok(Variant::Gpu),
                    2 => Ok(Variant::Tpu),
                    other => Err(E::custom(format!("unknown variant integer: {other}"))),
                }
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Variant, E> {
                self.visit_u64(v as u64)
            }
        }
        d.deserialize_any(V)
    }
}

impl serde::Serialize for Variant {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Variant::Cpu => "DEFAULT",
            Variant::Gpu => "GPU",
            Variant::Tpu => "TPU",
        })
    }
}

impl Variant {
    #[inline]
    pub fn display_name(&self) -> &'static str {
        match self {
            Variant::Cpu => "CPU",
            Variant::Gpu => "GPU",
            Variant::Tpu => "TPU",
        }
    }
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProxyInfo {
    pub token: String,
    pub token_expires_in_seconds: i64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GetAssignmentResponse {
    #[serde(rename = "token")]
    pub xsrf_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Undefined,
    QuotaDeniedVariants,
    QuotaExceededUsageTime,
    Success,
    Denylisted,
}

impl<'de> serde::Deserialize<'de> for Outcome {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            0 => Ok(Outcome::Undefined),
            1 => Ok(Outcome::QuotaDeniedVariants),
            2 => Ok(Outcome::QuotaExceededUsageTime),
            4 => Ok(Outcome::Success),
            5 => Ok(Outcome::Denylisted),
            other => Err(serde::de::Error::custom(format!(
                "unknown outcome: {other}"
            ))),
        }
    }
}

impl serde::Serialize for Outcome {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let v: u8 = match self {
            Outcome::Undefined => 0,
            Outcome::QuotaDeniedVariants => 1,
            Outcome::QuotaExceededUsageTime => 2,
            Outcome::Success => 4,
            Outcome::Denylisted => 5,
        };
        s.serialize_u8(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(from = "u8", into = "u8")]
pub enum Shape {
    #[default]
    Standard,
    HighMem,
    Unknown(u8),
}

impl From<u8> for Shape {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0 => Shape::Standard,
            1 => Shape::HighMem,
            other => Shape::Unknown(other),
        }
    }
}

impl From<Shape> for u8 {
    #[inline]
    fn from(s: Shape) -> u8 {
        match s {
            Shape::Standard => 0,
            Shape::HighMem => 1,
            Shape::Unknown(v) => v,
        }
    }
}

impl Shape {
    #[inline]
    pub fn display_name(&self) -> &'static str {
        match self {
            Shape::Standard => "standard",
            Shape::HighMem => "high-ram",
            Shape::Unknown(_) => "unknown",
        }
    }
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub endpoint: String,
    pub variant: Variant,
    pub accelerator: Option<String>,
    pub machine_shape: Option<Shape>,
    pub runtime_proxy_info: RuntimeProxyInfo,
    #[serde(rename = "fit")]
    pub idle_timeout_sec: Option<u64>,
    pub outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedAssignment {
    pub endpoint: String,
    pub variant: Variant,
    pub accelerator: Option<String>,
    pub machine_shape: Option<Shape>,
    pub runtime_proxy_info: Option<RuntimeProxyInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ListAssignmentsResponse {
    pub assignments: Vec<ListedAssignment>,
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct JupyterTerminal {
    pub name: String,
}

/// Jupyter Contents API entry. Returned by `GET /api/contents/<path>`.
///
/// For `type = "file"` with `format = "base64"`, `content` is a base64
/// string. For `type = "directory"`, `content` is a JSON array of child
/// entries (each with their own name/path/type). With `content=0` query
/// the server omits `content` entirely and returns just metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentsEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

impl ContentsEntry {
    pub fn is_file(&self) -> bool {
        self.kind == "file" || self.kind == "notebook"
    }

    pub fn is_directory(&self) -> bool {
        self.kind == "directory"
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CcuInfo {
    pub current_balance: f64,
    pub consumption_rate_hourly: f64,
    pub assignments_count: u32,
    pub eligible_gpus: Vec<String>,
    pub eligible_tpus: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_deserialises_strings() {
        assert_eq!(
            serde_json::from_str::<Variant>("\"DEFAULT\"").unwrap(),
            Variant::Cpu
        );
        assert_eq!(
            serde_json::from_str::<Variant>("\"GPU\"").unwrap(),
            Variant::Gpu
        );
        assert_eq!(
            serde_json::from_str::<Variant>("\"TPU\"").unwrap(),
            Variant::Tpu
        );
    }

    #[test]
    fn variant_deserialises_integers() {
        assert_eq!(serde_json::from_str::<Variant>("0").unwrap(), Variant::Cpu);
        assert_eq!(serde_json::from_str::<Variant>("1").unwrap(), Variant::Gpu);
        assert_eq!(serde_json::from_str::<Variant>("2").unwrap(), Variant::Tpu);
    }

    #[test]
    fn variant_rejects_unknown() {
        assert!(serde_json::from_str::<Variant>("\"QUANTUM\"").is_err());
        assert!(serde_json::from_str::<Variant>("99").is_err());
    }

    #[test]
    fn variant_serialises_to_canonical_string() {
        assert_eq!(serde_json::to_string(&Variant::Cpu).unwrap(), "\"DEFAULT\"");
        assert_eq!(serde_json::to_string(&Variant::Gpu).unwrap(), "\"GPU\"");
        assert_eq!(serde_json::to_string(&Variant::Tpu).unwrap(), "\"TPU\"");
    }

    #[test]
    fn outcome_deserialises_known_codes() {
        assert_eq!(
            serde_json::from_str::<Outcome>("4").unwrap(),
            Outcome::Success
        );
        assert_eq!(
            serde_json::from_str::<Outcome>("1").unwrap(),
            Outcome::QuotaDeniedVariants
        );
        assert_eq!(
            serde_json::from_str::<Outcome>("5").unwrap(),
            Outcome::Denylisted
        );
    }

    #[test]
    fn outcome_rejects_unknown_code() {
        assert!(serde_json::from_str::<Outcome>("42").is_err());
    }

    #[test]
    fn assignment_parses_real_payload() {
        let json = r#"{
            "endpoint": "abc-123",
            "variant": "GPU",
            "accelerator": "T4",
            "machineShape": 1,
            "runtimeProxyInfo": {
                "token": "tok",
                "tokenExpiresInSeconds": 3600,
                "url": "https://example.com"
            },
            "fit": 5400,
            "outcome": 4
        }"#;
        let a: Assignment = serde_json::from_str(json).unwrap();
        assert_eq!(a.endpoint, "abc-123");
        assert_eq!(a.variant, Variant::Gpu);
        assert_eq!(a.accelerator.as_deref(), Some("T4"));
        assert_eq!(a.machine_shape, Some(Shape::HighMem));
        assert_eq!(a.idle_timeout_sec, Some(5400));
        assert_eq!(a.outcome, Some(Outcome::Success));
    }

    #[test]
    fn listed_assignment_allows_missing_proxy_info() {
        let json = r#"{"endpoint":"e","variant":0}"#;
        let la: ListedAssignment = serde_json::from_str(json).unwrap();
        assert_eq!(la.variant, Variant::Cpu);
        assert!(la.runtime_proxy_info.is_none());
    }

    #[test]
    fn shape_round_trip_known_and_unknown() {
        assert_eq!(Shape::from(0u8), Shape::Standard);
        assert_eq!(Shape::from(1u8), Shape::HighMem);
        assert_eq!(Shape::from(7u8), Shape::Unknown(7));
        assert_eq!(u8::from(Shape::Standard), 0);
        assert_eq!(u8::from(Shape::HighMem), 1);
        assert_eq!(u8::from(Shape::Unknown(7)), 7);
    }

    #[test]
    fn shape_json_round_trip() {
        let s: Shape = serde_json::from_str("0").unwrap();
        assert_eq!(s, Shape::Standard);
        let s: Shape = serde_json::from_str("1").unwrap();
        assert_eq!(s, Shape::HighMem);
        assert_eq!(serde_json::to_string(&Shape::HighMem).unwrap(), "1");
    }

    #[test]
    fn ccu_info_parses() {
        let json = r#"{
            "currentBalance": 42.25,
            "consumptionRateHourly": 1.76,
            "assignmentsCount": 1,
            "eligibleGpus": ["T4", "A100"],
            "eligibleTpus": ["v2-8"]
        }"#;
        let c: CcuInfo = serde_json::from_str(json).unwrap();
        assert_eq!(c.current_balance, 42.25);
        assert_eq!(c.consumption_rate_hourly, 1.76);
        assert_eq!(c.assignments_count, 1);
        assert_eq!(c.eligible_gpus, vec!["T4".to_string(), "A100".to_string()]);
        assert_eq!(c.eligible_tpus, vec!["v2-8".to_string()]);
    }
}
