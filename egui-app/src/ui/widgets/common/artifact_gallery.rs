use std::collections::{BTreeMap, HashMap};

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
use crate::ui::widgets::trial_detail_modal::{TrialDetailModal, TrialDetailTarget};

/// 1 ページに表示する artifact カード数（All モード）。
/// 一度に生成する egui::Image を絞り、テクスチャ生成コストを抑える。
const PAGE_SIZE: usize = 12;

/// サムネイルの表示サイズ（大中小）。一辺のワールド座標長を持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbSize {
    /// 小（既定）。
    Small,
    /// 中。
    Medium,
    /// 大。
    Large,
}

impl ThumbSize {
    fn label(&self) -> &'static str {
        match self {
            ThumbSize::Small => "Small",
            ThumbSize::Medium => "Medium",
            ThumbSize::Large => "Large",
        }
    }

    /// サムネイル一辺のサイズ（ワールド座標）。
    fn size(&self) -> f32 {
        match self {
            ThumbSize::Small => 140.0,
            ThumbSize::Medium => 220.0,
            ThumbSize::Large => 320.0,
        }
    }
}

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

/// 1 枚のカードクリックで要求されたアクション。
#[derive(Default)]
struct CardClick {
    /// タイトルクリック → trial をハイライト。
    highlight: bool,
    /// 画像クリック → トライアル詳細モーダルを開く。
    detail: bool,
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
    pub thumb_size: ThumbSize,
    /// 1 トライアルに複数アーティファクトがある場合に、何番目（0 始まり）を表示するか。
    pub artifact_index: usize,
    /// カードクリックで開くトライアル詳細モーダル（散布図等と共有）。
    pub detail_modal: TrialDetailModal,
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
            thumb_size: ThumbSize::Small,
            artifact_index: 0,
            detail_modal: TrialDetailModal::new(),
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

            ui.separator();
            ui.label("Size:");
            egui::ComboBox::from_id_salt("artifact_gallery_thumb_size")
                .selected_text(self.thumb_size.label())
                .show_ui(ui, |ui| {
                    for s in [ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
                        ui.selectable_value(&mut self.thumb_size, s, s.label());
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

        // 各 trial の目的関数値ラベルを事前計算する（カードに良し悪し判断材料として表示）。
        let obj_by_trial = build_objective_labels(app_state);

        // キャンバスの Area 内では available_width が実質無制限になり horizontal_wrapped が
        // 折り返さないため、ウィジェット本体の幅をここで確定して列数計算に使う。
        let content_w = ui.available_width();

        match self.mode {
            ArtifactViewMode::All => {
                self.show_all(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
            ArtifactViewMode::Cluster => {
                self.show_cluster(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
            ArtifactViewMode::Mcdm => {
                self.show_mcdm(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
        }

        // カードクリックで開いたトライアル詳細モーダルを描画する（散布図等と同一内容）。
        if self.detail_modal.is_open() {
            if let Some(ctx) = app_state.current_study.as_ref() {
                self.detail_modal.show(
                    ui,
                    &ctx.view,
                    ctx.view.param_names(),
                    ctx.view.objective_names(),
                    &app_state.artifact_map,
                );
            }
        }
    }

    /// カードグリッド描画後の結果（ハイライト要求 / 詳細モーダル要求）を適用する。
    fn apply_card_outcome(
        &mut self,
        app_state: &mut AppState,
        clicked: Option<u32>,
        detail: Option<TrialDetailTarget>,
    ) {
        if let Some(trial_id) = clicked {
            app_state.set_highlight(trial_id);
        }
        if let Some(target) = detail {
            self.detail_modal.open(target);
        }
    }

    /// All モード: artifact を持つ全 trial をページネーション表示（各 trial は選択番号の 1 枚）。
    fn show_all(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
    ) {
        // artifact_map は Journal 全体（全 Study）の trial を含むため、まず現在の Study に
        // 属する trial へ絞り込む（trial_id は Journal 全体で一意なので他 Study の artifact が
        // 混在しうる）。その上で選択フィルタ（PCP ブラシ等）を適用する（空 = 全件）。
        let study_trials = restrict_to_current_study(
            artifact_trials_with_index(&app_state.artifact_map, artifact_index),
            app_state,
        );
        let trials = filter_ids_by_selection(study_trials, &app_state.selected_indices);
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
            if !app_state.selected_indices.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("filtered by selection").small().weak());
            }
        });

        let page_trials = paginate(&trials, self.page, PAGE_SIZE);
        let thumb = self.thumb_size.size();
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
        let mut detail: Option<TrialDetailTarget> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (h, p) =
                    render_card_grid(ui, app_state, content_w, thumb, &cards, obj_by_trial);
                clicked = h;
                detail = p;
            });
        self.apply_card_outcome(app_state, clicked, detail);
    }

    /// Cluster モード: 設定 UI + クラスタ別セクション表示。
    fn show_cluster(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
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
        let Some(cr) = app_state.cluster_cache.get(&key) else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No clustering result for this setting. Press Run.").weak(),
                );
            });
            return;
        };

        let cmap = colormap_from_name(&app_state.selected_colormap);
        let trial_ids: &[u32] = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.as_slice())
            .unwrap_or(&[]);
        let sections = cluster_sections(cr, trial_ids, &app_state.artifact_map, artifact_index);
        if sections.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No clustered trials have artifacts.").weak());
            });
            return;
        }

        let thumb = self.thumb_size.size();
        let n_clusters = cr.n_clusters.max(1);
        let mut clicked: Option<u32> = None;
        let mut detail: Option<TrialDetailTarget> = None;
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
                            let (h, p) =
                                render_card_grid(ui, app_state, w, thumb, &cards, obj_by_trial);
                            if h.is_some() {
                                clicked = h;
                            }
                            if p.is_some() {
                                detail = p;
                            }
                        });
                }
            });
        self.apply_card_outcome(app_state, clicked, detail);
    }

    /// MCDM モード: 設定 UI + ランキング順表示。
    fn show_mcdm(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
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
        let Some(result) = app_state.mcdm_cache.get(&key) else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No MCDM result for this setting. Press Run.").weak());
            });
            return;
        };

        let trial_ids: &[u32] = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.as_slice())
            .unwrap_or(&[]);
        let ordered = mcdm_ordered(
            result,
            trial_ids,
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

        let thumb = self.thumb_size.size();
        let mut cards: Vec<(u32, String, &ArtifactEntry)> = Vec::new();
        for entry in &ordered {
            let badge = format!("#{} ({:.3})", entry.rank, entry.score);
            cards.push((entry.trial_id, badge, entry.entry));
        }
        let mut clicked: Option<u32> = None;
        let mut detail: Option<TrialDetailTarget> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (h, p) =
                    render_card_grid(ui, app_state, content_w, thumb, &cards, obj_by_trial);
                clicked = h;
                detail = p;
            });
        self.apply_card_outcome(app_state, clicked, detail);
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
    obj_by_trial: &HashMap<u32, String>,
) -> (Option<u32>, Option<TrialDetailTarget>) {
    let columns = card_columns(content_w, thumb);
    let mut highlight: Option<u32> = None;
    let mut detail: Option<TrialDetailTarget> = None;
    for row in cards.chunks(columns) {
        ui.horizontal_top(|ui| {
            for (trial_id, badge, entry) in row {
                let obj_text = obj_by_trial.get(trial_id).map(String::as_str).unwrap_or("");
                let click =
                    show_artifact_card(ui, app_state, *trial_id, entry, badge, obj_text, thumb);
                if click.highlight {
                    highlight = Some(*trial_id);
                }
                if click.detail {
                    if let Some(target) = detail_target_for(app_state, *trial_id, badge) {
                        detail = Some(target);
                    }
                }
            }
        });
    }
    (highlight, detail)
}

/// `trial_id` から散布図共有の詳細モーダル用ターゲットを組み立てる。
/// `StudyView` 上の行 index を逆引きし、カードのバッジ（クラスタ番号 / MCDM ランク等）が
/// あれば Chart Info として付加する。
fn detail_target_for(
    app_state: &AppState,
    trial_id: u32,
    badge: &str,
) -> Option<TrialDetailTarget> {
    let ctx = app_state.current_study.as_ref()?;
    let row_index = ctx.view.trial_ids.iter().position(|&id| id == trial_id)?;
    let context = if badge.is_empty() {
        Vec::new()
    } else {
        vec![("Group".to_string(), badge.to_string())]
    };
    Some(TrialDetailTarget {
        trial_id,
        row_index,
        context,
    })
}

/// 各 trial の目的関数値を `name: value` 改行区切りで整形したマップを返す。
fn build_objective_labels(app_state: &AppState) -> HashMap<u32, String> {
    let mut out: HashMap<u32, String> = HashMap::new();
    let Some(ctx) = app_state.current_study.as_ref() else {
        return out;
    };
    let obj_names = &ctx.meta.objective_names;
    if obj_names.is_empty() {
        return out;
    }
    let view = &ctx.view;
    let cols = view.numeric_columns(obj_names);
    // 表示され得るのは artifact を持つ trial のみ。文字列整形をそれらに限定する。
    let artifact_map = &app_state.artifact_map;
    for (idx, &trial_id) in view.trial_ids.iter().enumerate() {
        if !artifact_map.contains_key(&trial_id) {
            continue;
        }
        let text = obj_names
            .iter()
            .zip(cols.iter())
            .map(|(name, col)| {
                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(f64::NAN);
                format!("{name}: {v:.4}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.insert(trial_id, text);
    }
    out
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

/// 1 枚の artifact カードを描画する。
/// 画像クリックで拡大プレビュー、タイトルクリックで trial ハイライトを要求する。
#[allow(clippy::too_many_arguments)]
fn show_artifact_card(
    ui: &mut egui::Ui,
    app_state: &AppState,
    trial_id: u32,
    entry: &ArtifactEntry,
    badge: &str,
    obj_text: &str,
    thumb: f32,
) -> CardClick {
    let mut click = CardClick::default();
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
                            .interact(egui::Sense::click())
                            .on_hover_text("Click for trial details");
                        if resp.clicked() {
                            click.detail = true;
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
                    click.highlight = true;
                }
                ui.add(egui::Label::new(egui::RichText::new(fname).small().weak()).truncate());
                // 目的関数値（良し悪し判断の材料）。カード幅に合わせて折り返す。
                if !obj_text.is_empty() {
                    ui.add(egui::Label::new(
                        egui::RichText::new(obj_text)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY),
                    ));
                }
            });
        });
    click
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

/// `ids` を現在の Study に属する trial_id だけに絞り込む。
/// `artifact_map` は Journal 全体（全 Study）の trial を含むため、対象 Study の
/// `view.trial_ids` に含まれるものだけを残す。Study 未選択時は空を返す。
pub fn restrict_to_current_study(ids: Vec<u32>, app_state: &AppState) -> Vec<u32> {
    let Some(ctx) = app_state.current_study.as_ref() else {
        return Vec::new();
    };
    let set: std::collections::HashSet<u32> = ctx.view.trial_ids.iter().copied().collect();
    ids.into_iter().filter(|id| set.contains(id)).collect()
}

/// 選択フィルタ（PCP ブラシ等）に基づき trial_id リストを絞り込む。
/// `selected_indices` が空の場合は全件を返す（テーブル等と同じ「空 = 全件」規約）。
pub fn filter_ids_by_selection(ids: Vec<u32>, selected_indices: &[u32]) -> Vec<u32> {
    if selected_indices.is_empty() {
        return ids;
    }
    let set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    ids.into_iter().filter(|id| set.contains(id)).collect()
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

    fn study_ctx_with_trial_ids(ids: &[u32]) -> crate::state::types::StudyContext {
        use crate::state::types::{StudyContext, StudyMeta, TrialRow as UiRow, TrialState};
        let rows: Vec<UiRow> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| UiRow {
                trial_id: id,
                trial_number: i as u32,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            })
            .collect();
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![],
            completed_trials: ids.len(),
            total_trials: ids.len(),
            param_names: vec![],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, rows)
    }

    #[test]
    fn restrict_to_current_study_keeps_only_study_trials() {
        // artifact_map は Journal 全体（study A: 0,1 / study B: 100,101）を含む。
        let mut state = AppState::new();
        state.artifact_map = map_with(&[0, 1, 100, 101]);
        // 現在の Study は trial 0,1 のみを持つ。
        state.current_study = Some(study_ctx_with_trial_ids(&[0, 1]));

        let ids = artifact_trials_with_index(&state.artifact_map, 0);
        assert_eq!(restrict_to_current_study(ids, &state), vec![0, 1]);
    }

    #[test]
    fn restrict_to_current_study_empty_without_study() {
        let mut state = AppState::new();
        state.artifact_map = map_with(&[0, 1]);
        let ids = artifact_trials_with_index(&state.artifact_map, 0);
        assert!(restrict_to_current_study(ids, &state).is_empty());
    }

    #[test]
    fn filter_ids_by_selection_empty_returns_all() {
        let ids = vec![2u32, 5, 9];
        assert_eq!(filter_ids_by_selection(ids.clone(), &[]), ids);
    }

    #[test]
    fn filter_ids_by_selection_keeps_only_selected() {
        let ids = vec![2u32, 5, 9, 11];
        assert_eq!(filter_ids_by_selection(ids, &[5, 11, 99]), vec![5, 11]);
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
