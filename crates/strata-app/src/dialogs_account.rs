use std::path::Path;

use strata_secrets::accounts::{AccountKind, AccountSettings};
use strata_secrets::mask::SecretMask;
use strata_secrets::vworld::VworldSettings;

#[derive(Debug, Clone, Default)]
pub struct AccountDialogState {
    pub vworld_configured: bool,
    pub heritage_configured: bool,
    pub ngii_configured: bool,
    pub status_summary_ko: String,
}

impl AccountDialogState {
    #[must_use]
    pub fn probe_local_accounts(config_dir: &Path) -> Self {
        let vworld_ini = config_dir.join("ka-hgis-vworld.ini");
        let vworld_key = VworldSettings::load_from_ini(&vworld_ini)
            .ok()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(VworldSettings::load_api_key);
        let heritage = AccountSettings::read_ini(
            &config_dir.join(AccountKind::HeritageIntranet.file_name()),
            AccountKind::HeritageIntranet,
        )
        .ok();
        let ngii = AccountSettings::read_ini(
            &config_dir.join(AccountKind::NgiiTopographic.file_name()),
            AccountKind::NgiiTopographic,
        )
        .ok();

        let vworld_configured = !vworld_key.is_empty();
        let heritage_configured = heritage.as_ref().is_some_and(|a| !a.username.is_empty());
        let ngii_configured = ngii.as_ref().is_some_and(|a| !a.username.is_empty());

        let raw_summary = format!(
            "VWorld: {}, 국가유산청 인트라넷: {}, 국토지리정보원: {}",
            if vworld_configured { "연결됨" } else { "미설정" },
            if heritage_configured { "연결됨" } else { "미설정" },
            if ngii_configured { "연결됨" } else { "미설정" }
        );
        Self {
            vworld_configured,
            heritage_configured,
            ngii_configured,
            status_summary_ko: SecretMask::redact(&raw_summary, &[]),
        }
    }
}
