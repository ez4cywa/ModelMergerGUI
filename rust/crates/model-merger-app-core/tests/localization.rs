use model_merger_app_core::{AppLanguage, Catalog, TextKey};

#[test]
fn all_native_gui_keys_exist_in_all_five_languages() {
    assert_eq!(5, AppLanguage::ALL.len());
    for language in AppLanguage::ALL {
        let catalog = Catalog::new(language);
        for &key in TextKey::ALL {
            assert!(
                !catalog.text(key).trim().is_empty(),
                "missing {language:?}/{key:?}"
            );
        }
    }
}

#[test]
fn language_names_are_native_and_switch_without_rebuilding_state() {
    assert_eq!(
        "中文",
        Catalog::new(AppLanguage::ChineseSimplified).language_name()
    );
    assert_eq!(
        "English",
        Catalog::new(AppLanguage::English).language_name()
    );
    assert_eq!(
        "Français",
        Catalog::new(AppLanguage::French).language_name()
    );
    assert_eq!(
        "Русский",
        Catalog::new(AppLanguage::Russian).language_name()
    );
    assert_eq!(
        "Español",
        Catalog::new(AppLanguage::Spanish).language_name()
    );
}
