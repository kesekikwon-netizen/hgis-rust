//! Tests for `strata-app` network launch sites and `@internet_check` snap behavior (`NET0c`).

use strata_net::curl_stdin::curl_url_config;
use strata_net::network::{allowed_launches, probe_dry_run};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral::{
    cadastral_tile_bbox_3857, cadastral_tile_path_in, cadastral_wms_url, fetch_cadastral_tile_in,
    CADASTRAL_TILE_PX,
};
use crate::basemap_fetch::{fetch_tile_in, tile_cache_path_in};
use crate::basemap_tiles::{tile_url, TileKind};
use crate::home_connection_model::InternetCheck;

const FAKE_KEY: &str = "FAKE-TEST-KEY-0000";

fn sample_png_bytes() -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    b.extend_from_slice(&[0u8; 24]);
    b
}

fn sample_jpeg_bytes() -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0];
    b.extend_from_slice(&[0u8; 24]);
    b
}

#[test]
fn basemap_fetch_launch_site_respects_switch_and_preserves_command() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let (kind, z, x, y) = (TileKind::Satellite, 15, 27_996, 12_713);

    // Switch off + cache miss: starts no process and creates nothing.
    let (off_launches, off_cmds) = probe_dry_run(false, || {
        assert_eq!(fetch_tile_in(root, kind, FAKE_KEY, z, x, y), None);
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    // Switch off + tile already in disk cache: returns cached file without starting curl.
    let cached = tile_cache_path_in(root, kind, z, x, y);
    std::fs::create_dir_all(cached.parent().expect("parent")).expect("mkdir");
    std::fs::write(&cached, sample_jpeg_bytes()).expect("write");
    let (hit_launches, _) = probe_dry_run(false, || {
        assert_eq!(fetch_tile_in(root, kind, FAKE_KEY, z, x, y), Some(cached.clone()));
    });
    assert_eq!(hit_launches, 0);
    std::fs::remove_file(&cached).expect("rm");

    // Switch on + cache miss: counter rises and exact command + stdin URL match today's tile_curl.
    let before = allowed_launches();
    let dest = tile_cache_path_in(root, kind, z, x, y);
    let expected_url = tile_url(kind, FAKE_KEY, z, x, y);
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        assert_eq!(fetch_tile_in(root, kind, FAKE_KEY, z, x, y), None);
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, "curl");
    assert_eq!(
        on_cmds[0].args,
        vec![
            "-sS",
            "-L",
            "--max-time",
            "12",
            "-H",
            "Referer: https://localhost",
            "-H",
            "User-Agent: Mozilla/5.0",
            "-o",
            &dest.to_string_lossy(),
            "--config",
            "-",
        ]
    );
    assert_eq!(on_cmds[0].stdin, curl_url_config(&expected_url).into_bytes());
}

#[test]
fn basemap_cadastral_launch_site_respects_switch_and_preserves_command() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let (z, x, y) = (17, 111_984, 50_852);

    // Switch off + cache miss: starts no process.
    let (off_launches, off_cmds) = probe_dry_run(false, || {
        assert_eq!(fetch_cadastral_tile_in(root, FAKE_KEY, z, x, y), None);
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    // Switch off + PNG already in disk cache: returns cached PNG without starting curl.
    let cached = cadastral_tile_path_in(root, z, x, y);
    std::fs::create_dir_all(cached.parent().expect("parent")).expect("mkdir");
    std::fs::write(&cached, sample_png_bytes()).expect("write");
    let (hit_launches, _) = probe_dry_run(false, || {
        assert_eq!(fetch_cadastral_tile_in(root, FAKE_KEY, z, x, y), Some(cached.clone()));
    });
    assert_eq!(hit_launches, 0);
    std::fs::remove_file(&cached).expect("rm");

    // Switch on + cache miss: counter rises and exact command + stdin URL match cadastral_curl.
    let before = allowed_launches();
    let dest = cadastral_tile_path_in(root, z, x, y);
    let bbox = cadastral_tile_bbox_3857(z, x, y);
    let expected_url = cadastral_wms_url(FAKE_KEY, bbox, CADASTRAL_TILE_PX);
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        assert_eq!(fetch_cadastral_tile_in(root, FAKE_KEY, z, x, y), None);
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, "curl");
    assert_eq!(
        on_cmds[0].args,
        vec![
            "-sS",
            "-L",
            "--max-time",
            "15",
            "-H",
            "Referer: https://localhost",
            "-H",
            "User-Agent: Mozilla/5.0",
            "-o",
            &dest.to_string_lossy(),
            "--config",
            "-",
        ]
    );
    assert_eq!(on_cmds[0].stdin, curl_url_config(&expected_url).into_bytes());
}

#[test]
fn snap_internet_check_keeps_prior_state_when_switch_is_off() {
    let (launches, cmds) = probe_dry_run(false, || {
        let mut app = StrataAppState::new_startup();
        assert!(matches!(app.home_map.internet, InternetCheck::NotChecked));
        app.snap_apply_action("@internet_check");
        assert!(
            matches!(app.home_map.internet, InternetCheck::NotChecked),
            "connection row must keep NotChecked when switch is off"
        );

        app.home_map.internet = InternetCheck::Done(true);
        app.snap_apply_action("@internet_check");
        assert!(
            matches!(app.home_map.internet, InternetCheck::Done(true)),
            "connection row must keep prior Done(true) state when switch is off"
        );
    });
    assert_eq!(launches, 0);
    assert!(cmds.is_empty());
}
