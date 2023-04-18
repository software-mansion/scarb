use cairo_lang_filesystem::cfg::Cfg as CairoCfg;

use scarb_metadata::Cfg;

#[test]
fn cfg_is_transmutable_via_serde() {
    for cfg in [CairoCfg::name("foo"), CairoCfg::kv("a", "b")] {
        serde_json::to_value(cfg)
            .and_then(serde_json::from_value::<Cfg>)
            .expect("Cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`.");
    }
}
