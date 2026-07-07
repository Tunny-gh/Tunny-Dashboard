//! クラスタリング設定 UI と計算キュー投入の共通ロジック（D-3）。
//!
//! 2D / 3D クラスタ散布図・クラスタテーブル・Artifact ギャラリーの 4 ウィジェットは、
//! いずれも同じクラスタリング設定（k / Max k / 対象空間 / k 選択モード / Init 戦略）と
//! 実行状態（computing / pending / error）を持ち、同一の制御行 UI と「Run 押下 →
//! バリデーション → キュー投入」フローを重複して実装していた。
//!
//! 各ウィジェットは（外部から直接フィールド参照・take される都合で）設定値と実行状態を
//! フラットなフィールドとして保持し続ける必要があるため、ここでは値をまとめて所有する
//! 代わりに、それらフィールドへの可変参照束 [`ClusterControls`] を受け取って UI 描画と
//! キュー投入判定だけを共通化する。

use crate::state::messages::ClusterUiError;
use crate::ui::widgets::cluster_scatter::{
    validate_cluster_request, ClusterComputeRequest, ClusterSpace, KMeansInitStrategy,
    KSelectionMode,
};

/// クラスタ制御 UI が編集する設定値と実行状態への可変参照束。
///
/// 4 ウィジェットは同一のフィールド構成を持つが、テストやワーカー側から個別フィールドを
/// 直接参照・`take` するためフィールドはフラットに保つ。各ウィジェットは呼び出しごとに
/// 自分のフィールドからこの束を組み立て、`show_controls` / `try_queue_compute` に委譲する。
pub struct ClusterControls<'a> {
    pub k: &'a mut usize,
    pub target_space: &'a mut ClusterSpace,
    pub k_mode: &'a mut KSelectionMode,
    pub init_strategy: &'a mut KMeansInitStrategy,
    /// Elbow（自動）モードで探索する k の上限。
    pub elbow_max_k: &'a mut usize,
    pub computing: &'a mut bool,
    pub pending_compute: &'a mut Option<ClusterComputeRequest>,
    pub last_error: &'a mut Option<ClusterUiError>,
}

impl ClusterControls<'_> {
    /// クラスタリング設定 UI（k / Max k / モード / 空間 / Init / Run）を 1 行で描画する。
    ///
    /// - `count` は k / Max k・実行可否の基準となる対象点数（多くはパレートフロント点数）。
    /// - `id_prefix` は 3 つの ComboBox の `id_salt` 衝突回避に使う（ウィジェットごとに一意）。
    ///   `"{id_prefix}_k_mode"` / `"{id_prefix}_space"` / `"{id_prefix}_init"` を割り当てる。
    /// - `inline_spinner` が true のとき、実行中は Run の右にスピナーを表示する
    ///   （2D 散布図は本体側で別途スピナー表示するため false を渡す）。
    ///
    /// Run 押下時は `try_queue_compute` を呼び、バリデーションを通れば pending に積む。
    pub fn show_controls(
        &mut self,
        ui: &mut egui::Ui,
        count: usize,
        id_prefix: &str,
        inline_spinner: bool,
    ) {
        ui.horizontal(|ui| {
            let k_editable = !*self.computing && *self.k_mode == KSelectionMode::Manual;
            ui.label("k:");
            ui.add_enabled(
                k_editable,
                egui::DragValue::new(&mut *self.k).range(2..=count.max(2)),
            );

            let elbow_max_k_editable =
                !*self.computing && *self.k_mode == KSelectionMode::ElbowDefault;
            ui.label("Max k:");
            ui.add_enabled(
                elbow_max_k_editable,
                egui::DragValue::new(&mut *self.elbow_max_k).range(2..=50),
            );

            egui::ComboBox::from_id_salt(format!("{id_prefix}_k_mode"))
                .selected_text(self.k_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.k_mode,
                        KSelectionMode::ElbowDefault,
                        KSelectionMode::ElbowDefault.label(),
                    );
                    ui.selectable_value(
                        self.k_mode,
                        KSelectionMode::Manual,
                        KSelectionMode::Manual.label(),
                    );
                });

            egui::ComboBox::from_id_salt(format!("{id_prefix}_space"))
                .selected_text(self.target_space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Objective,
                        ClusterSpace::Objective.label(),
                    );
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Variable,
                        ClusterSpace::Variable.label(),
                    );
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Combined,
                        ClusterSpace::Combined.label(),
                    );
                });

            ui.label("Init:");
            egui::ComboBox::from_id_salt(format!("{id_prefix}_init"))
                .selected_text(self.init_strategy.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.init_strategy,
                        KMeansInitStrategy::KMeansPlusPlus,
                        KMeansInitStrategy::KMeansPlusPlus.label(),
                    );
                    ui.selectable_value(
                        self.init_strategy,
                        KMeansInitStrategy::Deterministic,
                        KMeansInitStrategy::Deterministic.label(),
                    );
                });

            if ui
                .add_enabled(!*self.computing, egui::Button::new("Run"))
                .clicked()
            {
                self.try_queue_compute(count);
            }

            if inline_spinner && *self.computing {
                ui.spinner();
                ui.label("Running clustering...");
            }
        });
    }

    /// 現在の設定でクラスタリング要求を組み立ててバリデーションし、通れば pending に積む。
    /// バリデーション失敗時は pending を空にしてエラーを立てる（従来の各実装と同一挙動）。
    pub fn try_queue_compute(&mut self, count: usize) {
        let request = ClusterComputeRequest {
            k: *self.k,
            target_space: *self.target_space,
            k_mode: *self.k_mode,
            init_strategy: *self.init_strategy,
            elbow_max_k: *self.elbow_max_k,
        };

        match validate_cluster_request(&request, count) {
            Ok(()) => {
                *self.pending_compute = Some(request);
                *self.computing = true;
                *self.last_error = None;
            }
            Err(err) => {
                *self.pending_compute = None;
                *self.last_error = Some(err);
            }
        }
    }
}
