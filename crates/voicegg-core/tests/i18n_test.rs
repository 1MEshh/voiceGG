use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn collect_keys(val: &serde_json::Value, prefix: &str, acc: &mut HashSet<String>) {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let full_key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                acc.insert(full_key.clone());
                collect_keys(v, &full_key, acc);
            }
        }
        _ => {}
    }
}

#[test]
fn test_arabic_translation_has_all_english_keys() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // repo root is two levels up from crates/voicegg-core
    let root = manifest_dir.parent().unwrap().parent().unwrap();
    let en_path = root.join("app/src/lib/i18n/en.json");
    let ar_path = root.join("app/src/lib/i18n/ar.json");

    assert!(en_path.exists(), "en.json missing at {:?}", en_path);
    assert!(ar_path.exists(), "ar.json missing at {:?}", ar_path);

    let en_str = fs::read_to_string(&en_path).expect("Failed to read en.json");
    let ar_str = fs::read_to_string(&ar_path).expect("Failed to read ar.json");

    let en_json: serde_json::Value =
        serde_json::from_str(&en_str).expect("en.json is not valid JSON");
    let ar_json: serde_json::Value =
        serde_json::from_str(&ar_str).expect("ar.json is not valid JSON");

    let mut en_keys = HashSet::new();
    let mut ar_keys = HashSet::new();

    collect_keys(&en_json, "", &mut en_keys);
    collect_keys(&ar_json, "", &mut ar_keys);

    let mut missing = Vec::new();
    for key in &en_keys {
        if !ar_keys.contains(key) {
            missing.push(key.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "ar.json is missing the following keys found in en.json: {:?}",
        missing
    );
}
