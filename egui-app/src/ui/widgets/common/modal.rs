//! モーダルダイアログの共通足場（D-4）。
//!
//! 各モーダル（CSV インポート / ライセンス / RDB URL / レポート / トライアル詳細）は
//! `egui::Modal::new(Id::new(..)).show(ctx, |ui| { 最小幅設定; 見出し; 本文 })` と
//! 末尾の `should_close()` 判定を重複して書いていた。この足場でその定型部分を 1 箇所に集約し、
//! 各モーダルは ID・タイトル・最小幅（＋任意で最大幅・最小高さ）と本文クロージャだけを渡す。
//!
//! `should_close()` を組み合わせる最終的な「閉じる」条件（バリデーション・生成中の抑止など）は
//! モーダルごとに異なるため、足場は `should_close` を [`ModalOutcome`] として返すだけに留める。

/// 足場の描画結果。各モーダルは Load / Export / Cancel 等の確定操作を本文クロージャ内で
/// 捕捉した可変変数へ記録するため、足場が返すのは `should_close` のみとする。
pub struct ModalOutcome {
    /// 背景クリック・Esc 等でモーダルが閉じられようとしているか。
    pub should_close: bool,
}

/// モーダル足場のビルダ。`new` で ID と最小幅を与え、必要に応じて最大幅・最小高さ・
/// 自動見出しを追加してから `show` で描画する。
pub struct ModalScaffold<'a> {
    id: &'a str,
    min_width: f32,
    max_width: Option<f32>,
    min_height: Option<f32>,
    heading: Option<String>,
}

impl<'a> ModalScaffold<'a> {
    /// ID と最小幅を指定して足場を作る。
    pub fn new(id: &'a str, min_width: f32) -> Self {
        Self {
            id,
            min_width,
            max_width: None,
            min_height: None,
            heading: None,
        }
    }

    /// 最大幅を設定する。
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }

    /// 最小高さを設定する。
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = Some(h);
        self
    }

    /// 本文の先頭に自動で描く見出しを設定する。独自ヘッダ（例: 見出し＋Close ボタンを
    /// 同一行に置くトライアル詳細）を持つモーダルでは指定せず、本文側で描く。
    pub fn heading(mut self, title: impl Into<String>) -> Self {
        self.heading = Some(title.into());
        self
    }

    /// 足場を描画する。設定済みの最小幅・最大幅・最小高さと見出しを適用してから
    /// `body` を呼び、`should_close` を返す。
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) -> ModalOutcome {
        let modal = egui::Modal::new(egui::Id::new(self.id)).show(ctx, |ui| {
            ui.set_min_width(self.min_width);
            if let Some(w) = self.max_width {
                ui.set_max_width(w);
            }
            if let Some(h) = self.min_height {
                ui.set_min_height(h);
            }
            if let Some(title) = &self.heading {
                ui.heading(title);
            }
            body(ui);
        });
        ModalOutcome {
            should_close: modal.should_close(),
        }
    }
}
