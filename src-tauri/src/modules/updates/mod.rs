//! قنوات التحديث من داخل التطبيق. الفحص يتم فقط عند طلب المستخدم، والتحديث موقّع ويُتحقق منه قبل التثبيت.

use std::collections::HashMap;

use serde::Serialize;

pub const REPO_DOWNLOADS: &str = "https://github.com/shawady6/cashier/releases/download";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Test,
    Stable,
}

impl Channel {
    pub fn from_settings(s: &HashMap<String, String>) -> Channel {
        match s.get("update_channel").map(|v| v.as_str()) {
            Some("stable") => Channel::Stable,
            _ => Channel::Test,
        }
    }

    pub fn endpoint(self) -> String {
        let tag = match self {
            Channel::Test => "channel-test",
            Channel::Stable => "channel-stable",
        };
        format!("{REPO_DOWNLOADS}/{tag}/latest.json")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_defaults_to_test_and_maps_endpoints() {
        let mut s = HashMap::new();
        assert_eq!(Channel::from_settings(&s), Channel::Test);
        s.insert("update_channel".into(), "stable".into());
        assert_eq!(Channel::from_settings(&s), Channel::Stable);
        s.insert("update_channel".into(), "weird".into());
        assert_eq!(Channel::from_settings(&s), Channel::Test);
        assert!(Channel::Test.endpoint().ends_with("/channel-test/latest.json"));
        assert!(Channel::Stable.endpoint().starts_with("https://"));
        assert!(Channel::Stable.endpoint().ends_with("/channel-stable/latest.json"));
    }
}
