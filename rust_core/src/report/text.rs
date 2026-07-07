//! Key Finding（まとめ）の文章テンプレート（レンダラ非依存）。
//!
//! Markdown / HTML 双方のレンダラがここ 1 箇所のテンプレートを共有する
//! （文言の重複定義を避けるため）。テンプレートは推測や誇張を入れず、
//! モデルのファクトのみを言語化する。出力は「表示テキスト」と「強調フラグ」を
//! 持つ [`Span`] 列で、各レンダラが自前のエスケープ規則・強調記法
//! （Markdown は `**...**`、HTML は `<strong>`）で描画する。
//!
//! `Span::text` はエスケープ前の生文字列で、静的リテラル・数値・
//! ユーザー由来文字列（param 名 / 目的名）が混在する。レンダラ側で
//! 全 span を一律にエスケープすれば、リテラルは無変化・ユーザー文字列は
//! 安全化され、決定論的かつ安全な出力になる。

use super::model::{FindingKind, KeyFinding};
use super::{format_number, pct, ReportLang};

/// 文章を構成する 1 区間。
pub(crate) struct Span {
    /// 表示テキスト（エスケープ前の生文字列）。
    pub text: String,
    /// 強調するか（Markdown は `**`、HTML は `<strong>`）。
    pub emphasis: bool,
}

impl Span {
    /// 通常テキストの span。
    fn plain(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            emphasis: false,
        }
    }

    /// 強調テキストの span。
    fn strong(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            emphasis: true,
        }
    }
}

/// 制約フォールバック注記（feasible 解が 1 件も無いためパレート前面が
/// 全 trial の目的空間非劣解にフォールバックした旨）。Markdown / HTML
/// 両レンダラで文言を共有する（エスケープは呼び出し側の責務）。
pub(crate) fn infeasible_fallback_note(lang: ReportLang, n_infeasible: usize) -> String {
    match lang {
        ReportLang::En => format!(
            "Note: no trial satisfies all constraints, so the Pareto \
             front falls back to objective-space non-domination over \
             all trials; {n_infeasible} of these trials violate \
             constraints."
        ),
        ReportLang::Ja => format!(
            "注記: 全制約を満たす trial が無いため、パレート前面は\
             全 trial の目的空間非劣解にフォールバックしています。\
             うち {n_infeasible} 件は制約違反です。"
        ),
    }
}

/// 重複解（`duplicate_of`）の凡例注記。Markdown / HTML 両レンダラで
/// 文言を共有する（エスケープは呼び出し側の責務）。
pub(crate) fn duplicate_legend_note(lang: ReportLang) -> &'static str {
    match lang {
        ReportLang::En => {
            "Note: \"(= #N)\" marks a trial whose objective values are identical \
             to trial #N (e.g. a re-sampled parameter set)."
        }
        ReportLang::Ja => {
            "注記: 「(= #N)」は trial #N と目的値が完全一致する重複解を示します\
             （同一パラメータの再サンプル等）。"
        }
    }
}

/// unix 秒（UTC）を ISO-8601 文字列へ整形する（chrono 非依存・決定論的）。
pub(crate) fn format_unix_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{sec:02}Z")
}

/// 1970-01-01 からの経過日数を `(年, 月, 日)` に変換する（Howard Hinnant 方式）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // 0..=146096
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// [`KeyFinding`] を言語別の [`Span`] 列に展開する。
pub(crate) fn finding_spans(lang: ReportLang, f: &KeyFinding) -> Vec<Span> {
    let num = |k: &str| f.metrics.get(k).copied().unwrap_or(f64::NAN);
    let lab = |k: &str| f.labels.get(k).cloned().unwrap_or_default();

    match f.kind {
        FindingKind::BestSingle => {
            let best = format_number(num("best"));
            let trial = format_number(num("trial"));
            let fp = pct(num("found_pct"));
            match lang {
                ReportLang::En => vec![
                    Span::plain("Best objective "),
                    Span::strong(best),
                    Span::plain(format!(
                        " at trial #{trial} (found at {fp}% of the run)."
                    )),
                ],
                ReportLang::Ja => vec![
                    Span::plain("最良値 "),
                    Span::strong(best),
                    Span::plain(format!("（trial #{trial}、全体の {fp}% 時点で発見）。")),
                ],
            }
        }
        FindingKind::ParetoSummary => {
            let front = format_number(num("front_size"));
            let complete = format_number(num("complete"));
            match lang {
                ReportLang::En => vec![
                    Span::plain("Pareto front holds "),
                    Span::strong(front),
                    Span::plain(format!(
                        " non-dominated trials out of {complete} completed."
                    )),
                ],
                ReportLang::Ja => vec![
                    Span::plain(format!("パレート前面は COMPLETE {complete} 件中 ")),
                    Span::strong(front),
                    Span::plain(" 件の非支配解で構成される。"),
                ],
            }
        }
        FindingKind::ConvergenceStatus => match lab("status").as_str() {
            "converged" => match lang {
                ReportLang::En => vec![
                    Span::plain("Optimization appears "),
                    Span::strong("converged"),
                    Span::plain(" — no best-value updates in the final 20% of trials."),
                ],
                ReportLang::Ja => vec![
                    Span::plain("最適化は"),
                    Span::strong("収束"),
                    Span::plain("したとみられる（後半20%の試行で best 更新なし）。"),
                ],
            },
            "still_improving" => match lang {
                ReportLang::En => vec![
                    Span::plain("Optimization is "),
                    Span::strong("still improving"),
                    Span::plain(
                        " — best value was updated within the final 20% of trials; more trials may help.",
                    ),
                ],
                ReportLang::Ja => vec![Span::plain(
                    "最適化はなお改善中（直近20%の試行でも best が更新されており、追加試行で改善の余地がある）。",
                )],
            },
            _ => match lang {
                ReportLang::En => vec![
                    Span::strong("Insufficient data"),
                    Span::plain(" for a convergence verdict (fewer than 10 completed trials)."),
                ],
                ReportLang::Ja => vec![Span::plain(
                    "収束判定に十分なデータがない（COMPLETE が10件未満）。",
                )],
            },
        },
        FindingKind::TopImportance => {
            let method = lab("method");
            let mut spans = match lang {
                ReportLang::En => vec![Span::plain(format!(
                    "Most influential parameters ({method}): "
                ))],
                ReportLang::Ja => vec![Span::plain(format!("影響の大きいパラメータ（{method}）: "))],
            };
            let mut first = true;
            for i in 1..=3 {
                let name = lab(&format!("param{i}"));
                if name.is_empty() {
                    continue;
                }
                if !first {
                    spans.push(Span::plain(", "));
                }
                first = false;
                let score = format_number(num(&format!("score{i}")));
                spans.push(Span::plain(name));
                spans.push(Span::plain(format!(" ({score})")));
            }
            spans.push(Span::plain(match lang {
                ReportLang::En => ".",
                ReportLang::Ja => "。",
            }));
            spans
        }
        FindingKind::TradeOff => {
            let a = lab("obj_a");
            let b = lab("obj_b");
            let rho = format_number(num("rho"));
            match lang {
                ReportLang::En => vec![
                    Span::plain("Objectives "),
                    Span::strong(a),
                    Span::plain(" and "),
                    Span::strong(b),
                    Span::plain(format!(" trade off (Spearman ρ = {rho}).")),
                ],
                ReportLang::Ja => vec![
                    Span::plain("目的 "),
                    Span::strong(a),
                    Span::plain(" と "),
                    Span::strong(b),
                    Span::plain(format!(
                        " はトレードオフの関係にある（Spearman ρ = {rho}）。"
                    )),
                ],
            }
        }
        FindingKind::Feasibility => {
            let feasible = format_number(num("feasible"));
            let total = format_number(num("total"));
            let rate = pct(num("rate") * 100.0);
            let tail = if f.labels.contains_key("has_best") {
                let bt = format_number(num("best_trial"));
                match lang {
                    ReportLang::En => format!("; best feasible at trial #{bt}"),
                    ReportLang::Ja => format!("、最良は trial #{bt}"),
                }
            } else {
                String::new()
            };
            match lang {
                ReportLang::En => vec![
                    Span::plain("Feasible trials: "),
                    Span::strong(format!("{feasible}/{total}")),
                    Span::plain(format!(" ({rate}%){tail}.")),
                ],
                ReportLang::Ja => vec![
                    Span::plain("実行可能な試行: "),
                    Span::strong(format!("{feasible}/{total}")),
                    Span::plain(format!("（{rate}%）{tail}。")),
                ],
            }
        }
        FindingKind::PruningEfficiency => {
            let pruned = format_number(num("pruned"));
            let rate = pct(num("rate") * 100.0);
            let tail = if f.labels.contains_key("has_step") {
                let step = format_number(num("median_step"));
                match lang {
                    ReportLang::En => format!("; median prune step {step}"),
                    ReportLang::Ja => format!("、中央値 step {step}"),
                }
            } else {
                String::new()
            };
            match lang {
                ReportLang::En => vec![
                    Span::plain("Pruning removed "),
                    Span::strong(pruned),
                    Span::plain(format!(" trials ({rate}% of finished){tail}.")),
                ],
                ReportLang::Ja => vec![
                    Span::plain("枝刈りにより "),
                    Span::strong(pruned),
                    Span::plain(format!(
                        " 件の試行が早期終了（終了試行の {rate}%）{tail}。"
                    )),
                ],
            }
        }
        FindingKind::DataQuality => {
            let nan = format_number(num("nan_count"));
            let fail = format_number(num("fail_count"));
            match lang {
                ReportLang::En => vec![Span::plain(format!(
                    "Data quality note: {nan} trial(s) with non-finite objective values, {fail} FAILED trial(s)."
                ))],
                ReportLang::Ja => vec![Span::plain(format!(
                    "データ品質の注意: 目的値が非有限の試行 {nan} 件、FAIL 試行 {fail} 件。"
                ))],
            }
        }
    }
}
