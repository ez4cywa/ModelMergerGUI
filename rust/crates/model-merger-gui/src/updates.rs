//! Opt-in release checks. Downloads and installation always remain user initiated.
use crate::theme;
use eframe::egui;
use model_merger_app_core::AppLanguage;
use serde::Deserialize;
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

const API: &str = "https://api.github.com/repos/ez4cywa/ModelMergerGUI";
const RELEASE_PAGE: &str = "https://github.com/ez4cywa/ModelMergerGUI/releases/latest";
const PACKAGE: &str = "CastModelMerger-win-x64.zip";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    state: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CheckResult {
    Current,
    Available {
        version: String,
        same_version: bool,
        download: String,
    },
}

fn version(value: &str) -> Option<[u64; 3]> {
    let parts: Vec<_> = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|value| value.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

fn compare(
    release: Release,
    remote_commit: &str,
    local_version: &str,
    local_commit: &str,
) -> Result<CheckResult, String> {
    let remote = version(&release.tag_name).ok_or("Unsupported release version")?;
    let local = version(local_version).ok_or("Invalid application version")?;
    if release.draft || release.prerelease {
        return Err("No stable release available".into());
    }
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == PACKAGE && asset.state == "uploaded")
        .ok_or("Release package is not available yet; retry later")?;
    let prefix = format!(
        "https://github.com/ez4cywa/ModelMergerGUI/releases/download/{}/",
        release.tag_name
    );
    if asset.browser_download_url != format!("{prefix}{PACKAGE}") {
        return Err("Unexpected release download address".into());
    }
    if remote_commit.len() != 40 || !remote_commit.bytes().all(|value| value.is_ascii_hexdigit()) {
        return Err("Invalid release commit".into());
    }
    if remote == local
        && (local_commit.len() != 40
            || !local_commit.bytes().all(|value| value.is_ascii_hexdigit()))
    {
        return Err("Build identity unavailable; compare the release manually".into());
    }
    if remote > local
        || (remote == local && local_commit.len() == 40 && remote_commit != local_commit)
    {
        Ok(CheckResult::Available {
            version: release.tag_name,
            same_version: remote == local,
            download: asset.browser_download_url,
        })
    } else {
        Ok(CheckResult::Current)
    }
}

fn fetch() -> Result<CheckResult, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .build()
        .into();
    let release: Release = agent
        .get(&format!("{API}/releases/latest"))
        .header("User-Agent", "CastModelMerger")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_json()
        .map_err(|error| error.to_string())?;
    if version(&release.tag_name).is_none() {
        return Err("Unsupported release version".into());
    }
    let commit = agent
        .get(&format!("{API}/commits/{}", release.tag_name))
        .header("User-Agent", "CastModelMerger")
        .header("Accept", "application/vnd.github.sha")
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    compare(
        release,
        commit.trim(),
        env!("CARGO_PKG_VERSION"),
        env!("CAST_BUILD_COMMIT"),
    )
}

#[derive(Default)]
pub(crate) struct UpdateChecker {
    pending: Option<Receiver<Result<CheckResult, String>>>,
    result: Option<Result<CheckResult, String>>,
    startup_checked: bool,
}

fn text(language: AppLanguage, choices: [&'static str; 5]) -> &'static str {
    choices[language as usize]
}

impl UpdateChecker {
    fn start(&mut self, context: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let context = context.clone();
        self.pending = Some(rx);
        self.result = None;
        std::thread::spawn(move || {
            let _ = tx.send(fetch());
            context.request_repaint();
        });
    }

    pub(crate) fn poll(&mut self, context: &egui::Context, automatic: bool) {
        if !self.startup_checked {
            self.startup_checked = true;
            if automatic {
                self.start(context);
            }
        }
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err("Update check stopped".into()));
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    context.request_repaint_after(Duration::from_millis(100))
                }
            }
        }
    }

    pub(crate) fn available(&self) -> bool {
        matches!(self.result, Some(Ok(CheckResult::Available { .. })))
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        language: AppLanguage,
        automatic: &mut bool,
    ) -> bool {
        let palette = theme::palette(ui);
        ui.label(
            egui::RichText::new(text(
                language,
                [
                    "软件更新",
                    "Software updates",
                    "Mises à jour",
                    "Обновления",
                    "Actualizaciones",
                ],
            ))
            .size(17.0),
        );
        let changed = ui
            .checkbox(
                automatic,
                text(
                    language,
                    [
                        "启动时检查更新（默认关闭）",
                        "Check on startup (off by default)",
                        "Vérifier au démarrage (désactivé par défaut)",
                        "Проверять при запуске (по умолчанию выкл.)",
                        "Comprobar al iniciar (desactivado por defecto)",
                    ],
                ),
            )
            .changed();
        ui.label(egui::RichText::new(text(language,["仅检查 GitHub 发布信息，不会自动下载或替换程序。", "Checks GitHub releases only. Never downloads or replaces the app automatically.", "Vérifie les versions GitHub, sans téléchargement ni remplacement automatique.", "Только проверка выпусков GitHub. Без автоматической загрузки и замены программы.", "Solo consulta versiones de GitHub. No descarga ni reemplaza la aplicación automáticamente."])).size(13.0).color(palette.secondary));
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.pending.is_none(),
                    egui::Button::new(text(
                        language,
                        [
                            "检查更新",
                            "Check for updates",
                            "Rechercher des mises à jour",
                            "Проверить обновления",
                            "Buscar actualizaciones",
                        ],
                    ))
                    .min_size(egui::vec2(0.0, 40.0)),
                )
                .clicked()
            {
                self.start(ui.ctx());
            }
            if self.pending.is_some() {
                ui.spinner();
            }
        });
        match &self.result {
            Some(Ok(CheckResult::Current)) => {
                ui.label(text(
                    language,
                    [
                        "当前已是最新发布版。",
                        "You have the latest release.",
                        "La version est à jour.",
                        "Установлена последняя версия.",
                        "La versión está actualizada.",
                    ],
                ));
            }
            Some(Ok(CheckResult::Available {
                version,
                same_version,
                download,
            })) => {
                let label = if *same_version {
                    text(
                        language,
                        [
                            "发现同版本更新包",
                            "Refreshed package available",
                            "Paquet actualisé disponible",
                            "Доступна новая сборка",
                            "Paquete actualizado disponible",
                        ],
                    )
                } else {
                    text(
                        language,
                        [
                            "发现新版本",
                            "New version available",
                            "Nouvelle version disponible",
                            "Доступна новая версия",
                            "Nueva versión disponible",
                        ],
                    )
                };
                ui.label(format!("{label}: {version}"));
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to(
                        text(
                            language,
                            [
                                "下载更新包",
                                "Download update",
                                "Télécharger",
                                "Скачать обновление",
                                "Descargar actualización",
                            ],
                        ),
                        download,
                    );
                    ui.hyperlink_to(
                        text(
                            language,
                            [
                                "查看更新说明",
                                "Release notes",
                                "Notes de version",
                                "Описание выпуска",
                                "Notas de la versión",
                            ],
                        ),
                        RELEASE_PAGE,
                    );
                });
            }
            Some(Err(error)) => {
                ui.colored_label(
                    palette.destructive,
                    text(
                        language,
                        [
                            "更新检查失败，可重试或前往 GitHub 下载。",
                            "Could not check updates. Retry or download from GitHub.",
                            "Échec de la vérification. Réessayer ou télécharger sur GitHub.",
                            "Не удалось проверить обновления. Повторите или откройте GitHub.",
                            "No se pudo comprobar. Reintente o descargue desde GitHub.",
                        ],
                    ),
                )
                .on_hover_text(error);
            }
            None => {}
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: vec![Asset {
                name: PACKAGE.into(),
                state: "uploaded".into(),
                browser_download_url: format!(
                    "https://github.com/ez4cywa/ModelMergerGUI/releases/download/{tag}/{PACKAGE}"
                ),
            }],
        }
    }
    #[test]
    fn compares_versions_and_same_version_builds_without_downgrades() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        assert_eq!(
            compare(release("v2.2.1"), &a, "2.2.1", &a).unwrap(),
            CheckResult::Current
        );
        assert!(matches!(
            compare(release("v2.2.1"), &b, "2.2.1", &a).unwrap(),
            CheckResult::Available {
                same_version: true,
                ..
            }
        ));
        assert!(matches!(
            compare(release("v2.3.0"), &b, "2.2.1", &a).unwrap(),
            CheckResult::Available {
                same_version: false,
                ..
            }
        ));
        assert_eq!(
            compare(release("v2.1.9"), &b, "2.2.1", &a).unwrap(),
            CheckResult::Current
        );
        let mut invalid = release("v2.2.1");
        invalid.assets[0].browser_download_url = "https://example.com/app.exe".into();
        assert!(compare(invalid, &b, "2.2.1", &a).is_err());
        let mut incomplete = release("v2.2.1");
        incomplete.assets.clear();
        assert!(compare(incomplete, &b, "2.2.1", &a).is_err());
    }
    #[test]
    fn default_poll_does_not_start_network_work() {
        let mut checker = UpdateChecker::default();
        checker.poll(&egui::Context::default(), false);
        assert!(checker.pending.is_none());
        assert!(checker.result.is_none());
    }
    #[test]
    #[ignore = "manual live GitHub API smoke test"]
    fn live_github_check() {
        println!("{:?}", fetch().unwrap());
    }
}
