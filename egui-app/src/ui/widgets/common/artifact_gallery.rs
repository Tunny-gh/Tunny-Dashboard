use std::collections::BTreeMap;

use crate::io::artifacts::{ArtifactEntry, ArtifactFileType};
use crate::state::app_state::AppState;
use crate::state::results::{ClusterResult, McdmResult};
use crate::theme::chart_colors::COLOR_LINK;
use crate::theme::colormap::ColorMap;
use crate::theme::colormap_name::colormap_from_name;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::cluster_scatter::{
    validate_cluster_request, ClusterCacheKey, ClusterComputeRequest, ClusterSpace,
    KMeansInitStrategy, KSelectionMode,
};
use crate::ui::widgets::mcdm_chart::McdmControls;

/// 1 ページに表示する artifact カード数（All モード）。
/// 一度に生成する egui::Image を絞り、テクスチャ生成コストを抑える。
const PAGE_SIZE: usize = 12;
/// サムネイル一辺の既定サイズ（ワールド座標）。
const DEFAULT_THUMB: f32 = 140.0;

/// Artifact ギャラリーの表示モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactViewMode {
    /// 全 artifact をページネーション表示（設定不要）。
    All,
    /// クラスタリング結果でグルーピング表示。
    Cluster,
    /// MCDM ランキング順に表示。
    Mcdm,
}

impl ArtifactViewMode {
    fn label(&self) -> &'static str {
        match self {
            ArtifactViewMode::All => "All",
            ArtifactViewMode::Cluster => "By Cluster",
            ArtifactViewMode::Mcdm => "By MCDM Rank",
        }
    }
}

/// Artifact ギャラリーウィジェット。
///
/// `app_state.artifact_map`（trial_id → ファイルパス）を、クラスタリング / MCDM の結果と
/// 関連付けて表示する。クラスタ・MCDM の設定は本ウィジェットが自前で持ち、計算結果は
/// 設定キーごとに `cluster_cache` / `mcdm_cache` で共有・キャッシュされる。
///
/// これにより、設定を切り替えれば対応する結果が表示され、ギャラリーを複数配置して
/// それぞれ別設定にすれば「設定 A vs 設定 B」の比較ができる（他の Cluster/MCDM
/// ウィジェットと同じ比較スタイル）。
pub struct ArtifactGallery {
    pub mode: ArtifactViewMode,
    pub page: usize,
    pub thumb_size: f32,
    /// 1 トライアルに複数アーティファクトがある場合に、何番目（0 始まり）を表示するか。
    pub artifact_index: usize,
    // ── Cluster 設定（ClusterTable と同一構成）──────────────────
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    pub cluster_computing: bool,
    pub cluster_pending: Option<ClusterComputeRequest>,
    pub cluster_error: Option<crate::state::messages::ClusterUiError>,
    // ── MCDM 設定（既存 McdmControls を再利用）──────────────────
    pub mcdm: McdmControls,
}

impl Default for ArtifactGallery {
    fn default() -> Self {
        Self {
            mode: ArtifactViewMode::All,
            page: 0,
            thumb_size: DEFAULT_THUMB,
            artifact_index: 0,
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            cluster_computing: false,
            cluster_pending: None,
            cluster_error: None,
            mcdm: McdmControls::default(),
        }
    }
}

impl ArtifactGallery {
    pub fn new() -> Self {
        Self::default()
    }

    /// 現在のクラスタ設定に対応するキャッシュキー。
    pub fn cluster_cache_key(&self) -> ClusterCacheKey {
        ClusterCacheKey::new(self.target_space, self.k_mode, self.k, self.init_strategy)
    }

    /// クラスタリング実行をキューに積む（ClusterTable と同等）。
    fn try_queue_cluster_compute(&mut self, pareto_count: usize) {
        let request = ClusterComputeRequest {
            k: self.k,
            target_space: self.target_space,
            k_mode: self.k_mode,
            init_strategy: self.init_strategy,
        };
        match validate_cluster_request(&request, pareto_count) {
            Ok(()) => {
                self.cluster_pending = Some(request);
                self.cluster_computing = true;
                self.cluster_error = None;
            }
            Err(err) => {
                self.cluster_pending = None;
                self.cluster_error = Some(err);
            }
        }
    }

    pub fn set_cluster_error(&mut self, err: crate::state::messages::ClusterUiError) {
        self.cluster_computing = false;
        self.cluster_error = Some(err);
    }

    pub fn clear_cluster_runtime(&mut self) {
        self.cluster_computing = false;
        self.cluster_pending = None;
        self.cluster_error = None;
    }

    /// 共有のクラスタリング実行状態（computing / pending / error）を取り込む。
    /// キャンバスの各アイテムへ完了状態を反映するために使う（表示設定は維持）。
    pub fn adopt_cluster_runtime(&mut self, src: &Self) {
        self.cluster_computing = src.cluster_computing;
        self.cluster_pending = src.cluster_pending.clone();
        self.cluster_error = src.cluster_error.clone();
    }

    /// ギャラリーを描画する。
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        if app_state.current_study.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        }

        if app_state.artifact_map.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "No artifacts loaded. Use the \"Artifacts\" button to select a folder.",
                    )
                    .weak(),
                );
            });
            return;
        }

        // 1 トライアルあたりの最大アーティファクト数（インデックスセレクタの範囲に使う）。
        let max_artifacts = app_state
            .artifact_map
            .values()
            .map(|v| v.len())
            .max()
            .unwrap_or(1)
            .max(1);
        if self.artifact_index >= max_artifacts {
            self.artifact_index = max_artifacts - 1;
        }

        // モードセレクタ + アーティファクト番号セレクタ。
        ui.horizontal(|ui| {
            ui.label("View:");
            egui::ComboBox::from_id_salt("artifact_gallery_mode")
                .selected_text(self.mode.label())
                .show_ui(ui, |ui| {
                    for m in [
                        ArtifactViewMode::All,
                        ArtifactViewMode::Cluster,
                        ArtifactViewMode::Mcdm,
                    ] {
                        if ui.selectable_value(&mut self.mode, m, m.label()).clicked() {
                            self.page = 0;
                        }
                    }
                });

            // 1 トライアルに複数アーティファクトがある場合のみ表示する。
            if max_artifacts > 1 {
                ui.separator();
                ui.label("Artifact #:");
                ui.add(egui::DragValue::new(&mut self.artifact_index).range(0..=max_artifacts - 1))
                    .on_hover_text(
                        "Which artifact to show per trial (0-based). \
                     Trials without this index are skipped.",
                    );
                ui.label(format!("of up to {max_artifacts}"));
            }
        });
        ui.separator();

        let artifact_index = self.artifact_index;

        // キャンバスの Area 内では available_width が実質無制限になり horizontal_wrapped が
        // 折り返さないため、ウィジェット本体の幅をここで確定して列数計算に使う。
        let content_w = ui.available_width();

        match self.mode {
            ArtifactViewMode::All => self.show_all(ui, app_state, content_w, artifact_index),
            ArtifactViewMode::Cluster => {
                self.show_cluster(ui, app_state, content_w, artifact_index)
            }
            ArtifactViewMode::Mcdm => self.show_mcdm(ui, app_state, content_w, artifact_index),
        }
    }

    /// All モード: artifact を持つ全 trial をページネーション表示（各 trial は選択番号の 1 枚）。
    fn show_all(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
    ) {
        let trials = artifact_trials_with_index(&app_state.artifact_map, artifact_index);
        let total_pages = trials.len().div_ceil(PAGE_SIZE).max(1);
        if self.page >= total_pages {
            self.page = total_pages - 1;
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.page > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                self.page -= 1;
            }
            ui.label(format!("Page {}/{}", self.page + 1, total_pages));
            if ui
                .add_enabled(self.page + 1 < total_pages, egui::Button::new("Next ▶"))
                .clicked()
            {
                self.page += 1;
            }
            ui.separator();
            ui.label(format!("{} trials with artifacts", trials.len()));
        });

        let page_trials = paginate(&trials, self.page, PAGE_SIZE);
        let thumb = self.thumb_size;
        let mut cards: Vec<(u32, String, &ArtifactEntry)> = Vec::new();
        for &trial_id in page_trials {
            if let Some(entry) = app_state
                .artifact_map
                .get(&trial_id)
                .and_then(|entries| entries.get(artifact_index))
            {
                cards.push((trial_id, String::new(), entry));
            }
        }
        let mut clicked: Option<u32> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                clicked = render_card_grid(ui, app_state, content_w, thumb, &cards);
            });
        if let Some(trial_id) = clicked {
            app_state.set_highlight(trial_id);
        }
    }

    /// Cluster モード: 設定 UI + クラスタ別セクション表示。
    fn show_cluster(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
    ) {
        let pareto_count = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.pareto_rank.iter().filter(|&&r| r == 0).count())
            .unwrap_or(0);

        self.show_cluster_controls(ui, pareto_count);

        if let Some(err) = self.cluster_error.clone() {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR));
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_cluster_compute(pareto_count);
            }
            ui.separator();
        }

        if self.cluster_computing {
            return;
        }

        let key = self.cluster_cache_key();
        let Some(cr) = app_state.cluster_cache.get(&key).cloned() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No clustering result for this setting. Press Run.").weak(),
                );
            });
            return;
        };

        let cmap = colormap_from_name(&app_state.selected_colormap);
        let trial_ids = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.clone())
            .unwrap_or_default();
        let sections = cluster_sections(&cr, &trial_ids, &app_state.artifact_map, artifact_index);
        if sections.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No clustered trials have artifacts.").weak());
            });
            return;
        }

        let thumb = self.thumb_size;
        let n_clusters = cr.n_clusters.max(1);
        let mut clicked: Option<u32> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (label, members) in &sections {
                    let count = members.len();
                    let title = if *label < 0 {
                        format!("Unclustered ({count})")
                    } else {
                        format!("Cluster {label} ({count})")
                    };
                    let color = cluster_color(*label, n_clusters, &cmap);
                    let badge = if *label < 0 {
                        "·".to_string()
                    } else {
                        format!("C{label}")
                    };
                    let cards: Vec<(u32, String, &ArtifactEntry)> = members
                        .iter()
                        .map(|(trial_id, entry)| (*trial_id, badge.clone(), *entry))
                        .collect();
                    egui::CollapsingHeader::new(egui::RichText::new(title).color(color))
                        .id_salt(("artifact_cluster_section", *label))
                        .default_open(true)
                        .show(ui, |ui| {
                            // ヘッダーのインデント分を差し引いた幅で列数を決める。
                            let w = (content_w - 24.0).max(thumb);
                            if let Some(t) = render_card_grid(ui, app_state, w, thumb, &cards) {
                                clicked = Some(t);
                            }
                        });
                }
            });
        if let Some(trial_id) = clicked {
            app_state.set_highlight(trial_id);
        }
    }

    /// MCDM モード: 設定 UI + ランキング順表示。
    fn show_mcdm(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
    ) {
        let obj_names = app_state
            .current_study
            .as_ref()
            .map(|c| c.meta.objective_names.clone())
            .unwrap_or_default();

        if !self
            .mcdm
            .show_controls(ui, &obj_names, "artifact_gallery_mcdm")
        {
            return;
        }
        if self.mcdm.computing {
            return;
        }

        let key = self.mcdm.cache_key();
        let Some(result) = app_state.mcdm_cache.get(&key).cloned() else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No MCDM result for this setting. Press Run.").weak());
            });
            return;
        };

        let trial_ids = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.clone())
            .unwrap_or_default();
        let ordered = mcdm_ordered(
            &result,
            &trial_ids,
            &app_state.artifact_map,
            artifact_index,
            self.mcdm.top_n.value(),
        );
        if ordered.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No ranked trials have artifacts.").weak());
            });
            return;
        }

        let thumb = self.thumb_size;
        let mut cards: Vec<(u32, String, &ArtifactEntry)> = Vec::new();
        for entry in &ordered {
            let badge = format!("#{} ({:.3})", entry.rank, entry.score);
            cards.push((entry.trial_id, badge, entry.entry));
        }
        let mut clicked: Option<u32> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                clicked = render_card_grid(ui, app_state, content_w, thumb, &cards);
            });
        if let Some(trial_id) = clicked {
            app_state.set_highlight(trial_id);
        }
    }

    /// クラスタリング設定 UI（ClusterTable の show_controls と同操作感）。
    fn show_cluster_controls(&mut self, ui: &mut egui::Ui, pareto_count: usize) {
        ui.horizontal(|ui| {
            let k_editable = !self.cluster_computing && self.k_mode == KSelectionMode::Manual;
            ui.label("k:");
            ui.add_enabled(
                k_editable,
                egui::DragValue::new(&mut self.k).range(2..=pareto_count.max(2)),
            );

            egui::ComboBox::from_id_salt("artifact_gallery_k_mode")
                .selected_text(self.k_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.k_mode,
                        KSelectionMode::ElbowDefault,
                        KSelectionMode::ElbowDefault.label(),
                    );
                    ui.selectable_value(
                        &mut self.k_mode,
                        KSelectionMode::Manual,
                        KSelectionMode::Manual.label(),
                    );
                });

            egui::ComboBox::from_id_salt("artifact_gallery_space")
                .selected_text(self.target_space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Objective,
                        ClusterSpace::Objective.label(),
                    );
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Variable,
                        ClusterSpace::Variable.label(),
                    );
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Combined,
                        ClusterSpace::Combined.label(),
                    );
                });

            ui.label("Init:");
            egui::ComboBox::from_id_salt("artifact_gallery_init")
                .selected_text(self.init_strategy.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.init_strategy,
                        KMeansInitStrategy::KMeansPlusPlus,
                        KMeansInitStrategy::KMeansPlusPlus.label(),
                    );
                    ui.selectable_value(
                        &mut self.init_strategy,
                        KMeansInitStrategy::Deterministic,
                        KMeansInitStrategy::Deterministic.label(),
                    );
                });

            if ui
                .add_enabled(!self.cluster_computing, egui::Button::new("Run"))
                .clicked()
            {
                self.try_queue_cluster_compute(pareto_count);
            }

            if self.cluster_computing {
                ui.spinner();
                ui.label("Running clustering...");
            }
        });
        ui.separator();
    }
}

/// 1 枚のカードが占める概算幅（サムネ + 枠線・内側余白・カード間隔）。
const CARD_PADDING: f32 = 24.0;

/// `content_w` に収まるカード列数を返す（最低 1 列）。
pub fn card_columns(content_w: f32, thumb: f32) -> usize {
    let col_w = thumb + CARD_PADDING;
    if col_w <= 0.0 {
        return 1;
    }
    ((content_w / col_w).floor() as usize).max(1)
}

/// カード群を `content_w` に収まる列数で折り返して描画する。
/// キャンバスの Area 内では `horizontal_wrapped` が折り返さないため、列数を明示計算して
/// 行ごとに `horizontal` で並べる。クリックされたカードの trial_id を返す。
fn render_card_grid(
    ui: &mut egui::Ui,
    app_state: &AppState,
    content_w: f32,
    thumb: f32,
    cards: &[(u32, String, &ArtifactEntry)],
) -> Option<u32> {
    let columns = card_columns(content_w, thumb);
    let mut clicked: Option<u32> = None;
    for row in cards.chunks(columns) {
        ui.horizontal_top(|ui| {
            for (trial_id, badge, entry) in row {
                if show_artifact_card(ui, app_state, *trial_id, entry, badge, thumb) {
                    clicked = Some(*trial_id);
                }
            }
        });
    }
    clicked
}

/// クラスタラベルから色を求める（ClusterTable と同じ規則）。
fn cluster_color(label: i32, n_clusters: usize, colormap: &ColorMap) -> egui::Color32 {
    if label < 0 {
        return crate::theme::TEXT_SECONDARY;
    }
    let t = if n_clusters <= 1 {
        0.5
    } else {
        label as f32 / (n_clusters - 1) as f32
    };
    colormap.interpolate(t)
}

/// 1 枚の artifact カードを描画する。クリックされたら true。
fn show_artifact_card(
    ui: &mut egui::Ui,
    app_state: &AppState,
    trial_id: u32,
    entry: &ArtifactEntry,
    badge: &str,
    thumb: f32,
) -> bool {
    let mut clicked = false;
    let is_highlighted = app_state.highlighted_trial == Some(trial_id);
    let stroke = if is_highlighted {
        egui::Stroke::new(2.0, COLOR_LINK)
    } else {
        egui::Stroke::new(1.0, crate::theme::BORDER_COLOR)
    };

    egui::Frame::group(ui.style())
        .stroke(stroke)
        .inner_margin(egui::Margin::same(4.0))
        .show(ui, |ui| {
            ui.set_width(thumb);
            ui.vertical(|ui| {
                match entry.file_type() {
                    ArtifactFileType::Image => {
                        let uri = format!("file://{}", entry.path.to_string_lossy());
                        let resp = ui
                            .add(
                                egui::Image::from_uri(uri)
                                    .fit_to_exact_size(egui::vec2(thumb, thumb)),
                            )
                            .interact(egui::Sense::click());
                        if resp.clicked() {
                            clicked = true;
                        }
                    }
                    other => {
                        let icon = if matches!(other, ArtifactFileType::Csv) {
                            "📊"
                        } else {
                            "📦"
                        };
                        ui.vertical_centered(|ui| {
                            ui.add_space(thumb * 0.2);
                            ui.label(egui::RichText::new(icon).size(thumb * 0.4));
                            ui.add_space(thumb * 0.2);
                            if ui.small_button("Open").clicked() {
                                let _ = open::that(&entry.path);
                            }
                        });
                    }
                }
                let fname = entry.filename.clone();
                let header = if badge.is_empty() {
                    format!("Trial {trial_id}")
                } else {
                    format!("Trial {trial_id} · {badge}")
                };
                let label_resp = ui.add(
                    egui::Label::new(egui::RichText::new(header).small().strong())
                        .truncate()
                        .sense(egui::Sense::click()),
                );
                if label_resp.clicked() {
                    clicked = true;
                }
                ui.add(egui::Label::new(egui::RichText::new(fname).small().weak()).truncate());
            });
        });
    clicked
}

/// 指定インデックスのアーティファクトを持つ trial_id を昇順で返す。
pub fn artifact_trials_with_index(
    artifact_map: &std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    index: usize,
) -> Vec<u32> {
    let mut ids: Vec<u32> = artifact_map
        .iter()
        .filter(|(_, entries)| entries.len() > index)
        .map(|(&id, _)| id)
        .collect();
    ids.sort_unstable();
    ids
}

/// `items` のうち `page` ページ目（0 始まり, `per_page` 件）のスライスを返す。
pub fn paginate<T>(items: &[T], page: usize, per_page: usize) -> &[T] {
    if per_page == 0 || items.is_empty() {
        return &[];
    }
    let start = page.saturating_mul(per_page).min(items.len());
    let end = (start + per_page).min(items.len());
    &items[start..end]
}

/// クラスタ別に artifact を振り分ける。
/// 戻り値は (ラベル, [(trial_id, &paths)]) をラベル昇順（未クラスタ -1 は末尾）で並べたもの。
/// artifact を持たない trial は除外する。
/// `artifact_index` 番目のアーティファクトを持たない trial は除外する。
#[allow(clippy::type_complexity)]
pub fn cluster_sections<'a>(
    cluster_result: &ClusterResult,
    trial_ids: &[u32],
    artifact_map: &'a std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    artifact_index: usize,
) -> Vec<(i32, Vec<(u32, &'a ArtifactEntry)>)> {
    let mut by_label: BTreeMap<i32, Vec<(u32, &ArtifactEntry)>> = BTreeMap::new();
    for (idx, &label) in cluster_result.labels.iter().enumerate() {
        let Some(&trial_id) = trial_ids.get(idx) else {
            continue;
        };
        let Some(entry) = artifact_map
            .get(&trial_id)
            .and_then(|entries| entries.get(artifact_index))
        else {
            continue;
        };
        by_label.entry(label).or_default().push((trial_id, entry));
    }
    // BTreeMap は昇順。未クラスタ(-1)を末尾へ移す。
    let mut sections: Vec<(i32, Vec<(u32, &ArtifactEntry)>)> = Vec::new();
    let mut unclustered: Option<(i32, Vec<(u32, &ArtifactEntry)>)> = None;
    for (label, members) in by_label {
        if label < 0 {
            unclustered = Some((label, members));
        } else {
            sections.push((label, members));
        }
    }
    if let Some(u) = unclustered {
        sections.push(u);
    }
    sections
}

/// MCDM ランキング順のエントリ。
pub struct McdmArtifactEntry<'a> {
    pub rank: usize,
    pub score: f64,
    pub trial_id: u32,
    pub entry: &'a ArtifactEntry,
}

/// MCDM 結果をランク順に並べ、`artifact_index` 番目のアーティファクトを持つ trial を
/// 最大 `top_n` 件返す。
pub fn mcdm_ordered<'a>(
    result: &McdmResult,
    trial_ids: &[u32],
    artifact_map: &'a std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    artifact_index: usize,
    top_n: usize,
) -> Vec<McdmArtifactEntry<'a>> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut out: Vec<McdmArtifactEntry<'a>> = Vec::new();
    for (rank0, &row_idx) in ranked.iter().enumerate() {
        let idx = row_idx as usize;
        let Some(&trial_id) = trial_ids.get(idx) else {
            continue;
        };
        let Some(entry) = artifact_map
            .get(&trial_id)
            .and_then(|entries| entries.get(artifact_index))
        else {
            continue;
        };
        out.push(McdmArtifactEntry {
            rank: rank0 + 1,
            score: scores.get(idx).copied().unwrap_or(0.0),
            trial_id,
            entry,
        });
        if out.len() >= top_n {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::TopsisResult;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn entry(name: &str) -> ArtifactEntry {
        ArtifactEntry {
            path: PathBuf::from(name),
            filename: format!("{name}.png"),
            mimetype: "image/png".into(),
        }
    }

    fn map_with(ids: &[u32]) -> HashMap<u32, Vec<ArtifactEntry>> {
        ids.iter()
            .map(|&id| (id, vec![entry(&format!("{id}"))]))
            .collect()
    }

    #[test]
    fn artifact_trials_with_index_filters_and_sorts() {
        let mut m = map_with(&[5, 2, 9]);
        m.insert(3, vec![]); // 空は除外
        assert_eq!(artifact_trials_with_index(&m, 0), vec![2, 5, 9]);
        // index 1 を持つ trial のみ。
        m.insert(7, vec![entry("a"), entry("b")]);
        assert_eq!(artifact_trials_with_index(&m, 1), vec![7]);
    }

    #[test]
    fn paginate_clamps_range() {
        let v = vec![0, 1, 2, 3, 4];
        assert_eq!(paginate(&v, 0, 2), &[0, 1]);
        assert_eq!(paginate(&v, 2, 2), &[4]);
        assert_eq!(paginate(&v, 9, 2), &[] as &[i32]);
        assert_eq!(paginate(&v, 0, 0), &[] as &[i32]);
    }

    #[test]
    fn cluster_sections_groups_and_puts_unclustered_last() {
        // 行 index と trial_id を別物にして変換を検証する。
        let trial_ids = vec![10, 11, 12, 13];
        let cr = ClusterResult {
            labels: vec![1, 0, -1, 0],
            n_clusters: 2,
        };
        let m = map_with(&[10, 11, 12, 13]);
        let sections = cluster_sections(&cr, &trial_ids, &m, 0);
        let labels: Vec<i32> = sections.iter().map(|(l, _)| *l).collect();
        assert_eq!(labels, vec![0, 1, -1]); // 未クラスタ末尾
                                            // cluster 0 は trial 11, 13
        let c0: Vec<u32> = sections[0].1.iter().map(|(t, _)| *t).collect();
        assert_eq!(c0, vec![11, 13]);
    }

    #[test]
    fn cluster_sections_excludes_trials_without_artifacts() {
        let trial_ids = vec![10, 11, 12];
        let cr = ClusterResult {
            labels: vec![0, 0, 1],
            n_clusters: 2,
        };
        let m = map_with(&[10]); // 11, 12 は artifact 無し
        let sections = cluster_sections(&cr, &trial_ids, &m, 0);
        let total: usize = sections.iter().map(|(_, v)| v.len()).sum();
        assert_eq!(total, 1);
    }

    #[test]
    fn cluster_sections_selects_requested_artifact_index() {
        let trial_ids = vec![10, 11];
        let cr = ClusterResult {
            labels: vec![0, 0],
            n_clusters: 1,
        };
        let mut m = HashMap::new();
        m.insert(10, vec![entry("a"), entry("b")]); // index 1 あり
        m.insert(11, vec![entry("c")]); // index 1 なし → 除外
        let sections = cluster_sections(&cr, &trial_ids, &m, 1);
        let members = &sections[0].1;
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].0, 10);
        assert_eq!(members[0].1.filename, "b.png"); // 2 番目を選択
    }

    #[test]
    fn mcdm_ordered_respects_rank_and_topn() {
        let trial_ids = vec![10, 11, 12];
        // ranked_indices は行 index。スコアは行 index 基準。
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.1, 0.9, 0.5],
            ranked_indices: vec![1, 2, 0],
            positive_ideal: vec![],
            negative_ideal: vec![],
            duration_ms: 0.0,
        });
        let m = map_with(&[10, 11, 12]);
        let ordered = mcdm_ordered(&result, &trial_ids, &m, 0, 2);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].rank, 1);
        assert_eq!(ordered[0].trial_id, 11); // 行 index 1 -> trial 11
        assert!((ordered[0].score - 0.9).abs() < 1e-9);
        assert_eq!(ordered[1].trial_id, 12);
    }

    #[test]
    fn mcdm_ordered_skips_trials_without_artifacts() {
        let trial_ids = vec![10, 11, 12];
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.1, 0.9, 0.5],
            ranked_indices: vec![1, 2, 0],
            positive_ideal: vec![],
            negative_ideal: vec![],
            duration_ms: 0.0,
        });
        let m = map_with(&[12]); // 行 index 2 -> trial 12 のみ
        let ordered = mcdm_ordered(&result, &trial_ids, &m, 0, 10);
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].trial_id, 12);
        assert_eq!(ordered[0].rank, 2); // 全体ランクは 2 位
    }

    #[test]
    fn card_columns_fits_width_and_min_one() {
        // thumb=140 → 列幅 164。
        assert_eq!(card_columns(700.0, 140.0), 4); // 700/164 = 4.26 → 4
        assert_eq!(card_columns(164.0, 140.0), 1);
        assert_eq!(card_columns(10.0, 140.0), 1); // 最低 1 列
    }

    #[test]
    fn adopt_cluster_runtime_clears_stuck_computing() {
        let mut item = ArtifactGallery {
            cluster_computing: true,
            ..Default::default()
        };
        let global = ArtifactGallery::default();
        item.adopt_cluster_runtime(&global);
        assert!(!item.cluster_computing);
    }
}
