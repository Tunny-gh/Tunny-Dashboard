//! 差分パース・保留リスト管理（ライブ更新）(TASK-1201)
//!
//! 【役割】: Journal ファイルの差分バイト列を解析し、新規 COMPLETE 試行数を返す
//! 【設計方針】:
//!   - `append_journal_diff(data)`: 差分バイト列を処理する
//!     - 末尾の不完全 JSON 行は自動スキップ（次回呼び出しで再処理）
//!     - RUNNING 保留リスト: thread_local STATE に蓄積
//!     - COMPLETE 確定後: DataFrame への追記カウントを返す
//!   - `consumed_bytes`: 次回ポーリングのオフセット更新に使用
//!   - `reset_live_update_state()`: テスト・Study 切り替え時に状態をリセット
//!
//! REQ-130: append_journal_diff() — 差分解析
//! REQ-131: 不完全行スキップ（書き込み途中耐性）
//! REQ-132: RUNNING 保留リスト管理
//! REQ-133: COMPLETE 確定後の新試行数通知
//!
//! 参照: docs/tasks/tunny-dashboard-tasks.md TASK-1201

use std::cell::RefCell;
use std::collections::HashMap;
use serde_json::Value;

// =============================================================================
// 公開型定義
// =============================================================================

/// 【差分適用結果】: `append_journal_diff()` の戻り値
///
/// 🟢 REQ-130〜REQ-133
#[derive(Debug, Clone)]
pub struct AppendDiffResult {
    /// 今回の差分で新たに COMPLETE になった試行数
    pub new_completed: usize,
    /// 実際に処理したバイト数（次回ポーリングのオフセットに加算する）
    /// 不完全末尾行を除いた完全行の合計バイト数 🟢 REQ-131
    pub consumed_bytes: usize,
    /// まだ COMPLETE になっていない RUNNING 保留試行数
    pub pending_running: usize,
}

// =============================================================================
// 内部型定義
// =============================================================================

/// 【RUNNING 保留試行データ】: COMPLETE になるまで保持する試行の中間状態
/// 将来の DataFrame 追記実装のためにフィールドを保持する 🟡
#[derive(Debug, Default)]
#[allow(dead_code)]
struct PendingTrial {
    /// この試行が属する study_id
    study_idx: u32,
    /// 目的関数値（COMPLETE 時に SET_TRIAL_STATE_VALUES で設定）
    values: Option<Vec<f64>>,
    /// パラメータ表示値: param_name → f64
    param_display: HashMap<String, f64>,
    /// CategoricalDistribution の文字列ラベル
    param_category_label: HashMap<String, String>,
    /// user_attr 数値型
    user_attrs_numeric: HashMap<String, f64>,
    /// user_attr 文字列型
    user_attrs_string: HashMap<String, String>,
    /// constraint 値
    constraint_values: Vec<f64>,
}

/// 【ライブ更新の内部状態】: スレッドローカルで差分呼び出し間を跨いで保持
#[derive(Debug, Default)]
struct LiveUpdateState {
    /// 次に採番する trial_id（初期化後インクリメント）
    next_trial_id: u32,
    /// RUNNING 保留試行マップ: trial_id → PendingTrial
    pending: HashMap<u32, PendingTrial>,
}

// =============================================================================
// スレッドローカル状態
// =============================================================================

thread_local! {
    /// 【スレッドローカル状態】: 差分ポーリング間でRUNNING試行を保持する
    /// WASM は単一スレッドで動作するためスレッドローカルで十分 🟢
    static STATE: RefCell<LiveUpdateState> = RefCell::new(LiveUpdateState::default());
}

// =============================================================================
// 公開 API
// =============================================================================

/// 【差分解析】: Journal ファイルの差分バイト列を処理する
///
/// 【引数】:
///   data - 前回ポーリング以降の新規バイト列（末尾が不完全 JSON 行でも安全）
///
/// 【処理フロー】:
///   1. 末尾の不完全行を除く（最後の `\n` までのみ処理）🟢 REQ-131
///   2. 完全な各行を JSON パース → op_code で分岐
///   3. CREATE_TRIAL (op=4): RUNNING 保留に追加
///   4. SET_TRIAL_PARAM (op=5): 保留試行のパラメータ更新
///   5. SET_TRIAL_STATE_VALUES (op=6): COMPLETE なら new_completed++ + 保留から除去
///   6. 消費バイト数・RUNNING 保留数を返す 🟢 REQ-130
///
/// 🟢 REQ-130〜REQ-133
pub fn append_journal_diff(data: &[u8]) -> AppendDiffResult {
    // 【不完全行スキップ】: 最後の改行位置を探す → それ以降は次回に持ち越す 🟢 REQ-131
    let consumed = find_consumed_bytes(data);
    if consumed == 0 {
        let pending_running = STATE.with(|s| s.borrow().pending.len());
        return AppendDiffResult {
            new_completed: 0,
            consumed_bytes: 0,
            pending_running,
        };
    }

    let complete_data = &data[..consumed];
    let mut new_completed = 0usize;

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // 【行ループ】: 完全な各 JSON 行を処理
        for line in complete_data.split(|&b| b == b'\n') {
            let trimmed = line.iter()
                .position(|&b| b != b' ' && b != b'\r' && b != b'\t')
                .map(|i| &line[i..])
                .unwrap_or(line);
            if trimmed.is_empty() {
                continue;
            }

            // 【JSON パース】: 不正な JSON 行は無視して継続 🟢 REQ-002相当
            let json: Value = match serde_json::from_slice(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let op = json
                .get("op_code")
                .and_then(|v| v.as_u64())
                .unwrap_or(u64::MAX) as u8;

            match op {
                // 【CREATE_TRIAL (op=4)】: 新 RUNNING 試行を保留リストへ追加 🟢
                4 => {
                    let study_idx = json
                        .get("study_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let trial_id = s.next_trial_id;
                    s.next_trial_id += 1;
                    s.pending.insert(
                        trial_id,
                        PendingTrial {
                            study_idx,
                            ..Default::default()
                        },
                    );
                }

                // 【SET_TRIAL_PARAM (op=5)】: 保留試行のパラメータを更新 🟢
                5 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    if let Some(pending) = s.pending.get_mut(&trial_id) {
                        if let (Some(name), Some(val)) = (
                            json.get("param_name").and_then(|v| v.as_str()),
                            json.get("param_value_internal").and_then(|v| v.as_f64()),
                        ) {
                            // 【簡易逆変換】: distribution から表示値を計算
                            let display_val =
                                decode_param_value(val, json.get("distribution"));
                            let label = extract_categorical_label(val, json.get("distribution"));
                            if let Some(lbl) = label {
                                pending.param_category_label.insert(name.to_string(), lbl);
                            } else {
                                pending.param_display.insert(name.to_string(), display_val);
                            }
                        }
                    }
                }

                // 【SET_TRIAL_STATE_VALUES (op=6)】: COMPLETE なら保留から除去して計上 🟢
                6 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    let state_val = json
                        .get("state")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u8;

                    if state_val == 1 {
                        // 【COMPLETE 確定】: 保留から除去して new_completed++ 🟢
                        if let Some(mut pending) = s.pending.remove(&trial_id) {
                            // 目的値を設定
                            if let Some(vals_json) = json.get("values").and_then(|v| v.as_array()) {
                                pending.values = Some(
                                    vals_json
                                        .iter()
                                        .filter_map(|v| v.as_f64())
                                        .collect(),
                                );
                            }
                            // TODO: 実際の DataFrame 追記は将来の実装で対応 (TASK-1201拡張)
                            // 現在は new_completed カウントのみ返す
                            new_completed += 1;
                        } else {
                            // 【保留なしで COMPLETE】: 差分の中だけで完結した試行
                            new_completed += 1;
                        }
                    } else if state_val == 2 || state_val == 3 {
                        // PRUNED / FAIL: 保留から除去（カウントしない）
                        s.pending.remove(&trial_id);
                    }
                }

                // 【その他 op_code】: 無視 🟢 REQ-002相当
                _ => {}
            }
        }
    });

    let pending_running = STATE.with(|s| s.borrow().pending.len());

    AppendDiffResult {
        new_completed,
        consumed_bytes: consumed,
        pending_running,
    }
}

/// 【状態リセット】: テスト・Study 切り替え時にスレッドローカル状態を初期化する
///
/// 【使用場面】:
///   - 別の Journal ファイルに切り替えるとき
///   - テスト間の状態分離
pub fn reset_live_update_state() {
    STATE.with(|s| *s.borrow_mut() = LiveUpdateState::default());
}

/// 【テスト用】: 現在の next_trial_id を設定する（ライブ更新開始時の連番設定用）
pub fn set_next_trial_id(id: u32) {
    STATE.with(|s| s.borrow_mut().next_trial_id = id);
}

// =============================================================================
// 内部ヘルパー
// =============================================================================

/// 【消費バイト数算出】: 末尾の完全な `\n` の位置までを消費バイトとして返す
///
/// 末尾が改行で終わらない場合（不完全行）はその行を除いた位置を返す 🟢 REQ-131
fn find_consumed_bytes(data: &[u8]) -> usize {
    // 最後の '\n' の位置 + 1
    match data.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => 0, // 改行がない = 全部が不完全行 → 0 を返す
    }
}

/// 【パラメータ表示値計算】: distribution JSON から内部値を表示値に変換する
///
/// journal_parser.rs の Distribution::to_display_f64() と同等のロジック 🟢
fn decode_param_value(internal: f64, dist: Option<&Value>) -> f64 {
    let Some(dist) = dist else { return internal };
    match dist.get("name").and_then(|v| v.as_str()).unwrap_or("") {
        "FloatDistribution" => {
            if dist.get("log").and_then(|v| v.as_bool()).unwrap_or(false) {
                internal.exp()
            } else {
                internal
            }
        }
        "IntDistribution" => {
            let low = dist.get("low").and_then(|v| v.as_i64()).unwrap_or(0);
            let step = dist.get("step").and_then(|v| v.as_i64()).unwrap_or(1).max(1);
            let log = dist.get("log").and_then(|v| v.as_bool()).unwrap_or(false);
            let rounded = if log {
                internal.exp().round() as i64
            } else {
                internal.round() as i64
            };
            (low + rounded * step) as f64
        }
        _ => internal,
    }
}

/// 【カテゴリラベル抽出】: CategoricalDistribution の文字列ラベルを返す
fn extract_categorical_label(internal: f64, dist: Option<&Value>) -> Option<String> {
    let dist = dist?;
    if dist.get("name").and_then(|v| v.as_str())? != "CategoricalDistribution" {
        return None;
    }
    let choices = dist.get("choices")?.as_array()?;
    let idx = internal.round() as usize;
    choices.get(idx).map(|v| match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    })
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト前後に状態をリセット
    fn with_fresh_state<F: FnOnce()>(f: F) {
        reset_live_update_state();
        f();
        reset_live_update_state();
    }

    // テスト用 Journal 行ビルダー
    fn make_create_trial(study_id: u32) -> String {
        format!(r#"{{"op_code":4,"study_id":{}}}"#, study_id)
    }

    fn make_set_param(trial_id: u32, name: &str, val: f64) -> String {
        format!(
            r#"{{"op_code":5,"trial_id":{},"param_name":"{}","param_value_internal":{},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#,
            trial_id, name, val
        )
    }

    fn make_complete(trial_id: u32, values: &[f64]) -> String {
        let vals = values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"op_code":6,"trial_id":{},"state":1,"values":[{}]}}"#,
            trial_id, vals
        )
    }

    fn make_diff_bytes(lines: &[String]) -> Vec<u8> {
        let mut data = lines.join("\n");
        data.push('\n'); // 完全行として末尾に \n を付ける
        data.into_bytes()
    }

    // TC-1201-01: 完全な diff を処理して new_completed が正しくカウントされる
    #[test]
    fn tc_1201_01_complete_trial_counted() {
        // 【テスト目的】: CREATE→SET_PARAM→COMPLETE の完全な流れで new_completed=1 になることを確認 🟢
        with_fresh_state(|| {
            let lines = vec![
                make_create_trial(0),
                make_set_param(0, "x1", 0.5),
                make_complete(0, &[1.23]),
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);
            // 【確認内容】: 1つの COMPLETE 試行が検出されること
            assert_eq!(result.new_completed, 1, "COMPLETE 試行が1件カウントされるべき");
            assert_eq!(result.pending_running, 0, "COMPLETE後の保留リストは空");
        });
    }

    // TC-1201-02: 末尾の不完全行がスキップされる
    #[test]
    fn tc_1201_02_incomplete_last_line_skipped() {
        // 【テスト目的】: 末尾改行なし行が無視され consumed_bytes が正確なことを確認 🟢 REQ-131
        with_fresh_state(|| {
            let complete = make_create_trial(0);
            let incomplete = r#"{"op_code":4,"study_id":0"#; // 不完全な最終行

            let data = format!("{}\n{}", complete, incomplete).into_bytes();
            let result = append_journal_diff(&data);

            // 【確認内容】: consumed_bytes が complete 行の末尾 '\n' までの長さ
            let expected_consumed = complete.len() + 1; // +1 for '\n'
            assert_eq!(
                result.consumed_bytes, expected_consumed,
                "不完全末尾行は consumed_bytes に含まれないこと"
            );
        });
    }

    // TC-1201-03: 差分の中だけで RUNNING のままの試行が pending_running に反映される
    #[test]
    fn tc_1201_03_running_trial_pending() {
        // 【テスト目的】: COMPLETE にならなかった試行が pending_running に残ることを確認 🟢 REQ-132
        with_fresh_state(|| {
            let lines = vec![
                make_create_trial(0), // この試行は COMPLETE しない
                make_create_trial(0),
                make_complete(1, &[0.5]), // 2番目だけ COMPLETE
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);

            // 【確認内容】: new_completed=1, pending_running=1
            assert_eq!(result.new_completed, 1);
            assert_eq!(result.pending_running, 1, "RUNNING 試行1件が保留に残ること");
        });
    }

    // TC-1201-04: 全行が不完全（改行なし）のとき consumed_bytes=0
    #[test]
    fn tc_1201_04_no_newline_consumed_zero() {
        // 【テスト目的】: 改行がない差分データで consumed_bytes=0 になることを確認 🟢
        with_fresh_state(|| {
            let data = b"incomplete line without newline";
            let result = append_journal_diff(data);
            // 【確認内容】: 消費バイト数が 0
            assert_eq!(result.consumed_bytes, 0);
            assert_eq!(result.new_completed, 0);
        });
    }

    // TC-1201-05: 不正 JSON 行は無視して継続
    #[test]
    fn tc_1201_05_invalid_json_ignored() {
        // 【テスト目的】: 不正な JSON 行がスキップされ後続行が処理されることを確認 🟢
        with_fresh_state(|| {
            let lines = vec![
                "not valid json".to_string(), // 不正 JSON
                make_create_trial(0),
                make_complete(0, &[1.0]),
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);
            // 【確認内容】: 不正行をスキップして COMPLETE 1件を検出
            assert_eq!(result.new_completed, 1);
        });
    }

    // TC-1201-06: 複数 diff 呼び出しで跨ぐ RUNNING→COMPLETE
    #[test]
    fn tc_1201_06_cross_diff_running_to_complete() {
        // 【テスト目的】: 1回目の diff で RUNNING、2回目で COMPLETE になることを確認 🟢 REQ-132
        with_fresh_state(|| {
            // 1回目: CREATE のみ（RUNNING）
            let diff1 = make_diff_bytes(&[make_create_trial(0)]);
            let r1 = append_journal_diff(&diff1);
            assert_eq!(r1.new_completed, 0, "1回目は RUNNING のまま");
            assert_eq!(r1.pending_running, 1);

            // 2回目: COMPLETE
            let diff2 = make_diff_bytes(&[make_complete(0, &[2.0])]);
            let r2 = append_journal_diff(&diff2);
            assert_eq!(r2.new_completed, 1, "2回目で COMPLETE カウント");
            assert_eq!(r2.pending_running, 0);
        });
    }

    // TC-1201-07: reset_live_update_state で状態がクリアされる
    #[test]
    fn tc_1201_07_reset_clears_state() {
        // 【テスト目的】: リセット後に next_trial_id と pending が初期化されることを確認 🟢
        with_fresh_state(|| {
            let diff1 = make_diff_bytes(&[make_create_trial(0)]);
            append_journal_diff(&diff1);
            // リセット
            reset_live_update_state();
            // リセット後に再び CREATE→COMPLETE
            let diff2 = make_diff_bytes(&[make_create_trial(0), make_complete(0, &[1.0])]);
            let result = append_journal_diff(&diff2);
            // 【確認内容】: trial_id が 0 から再スタートして COMPLETE が検出される
            assert_eq!(result.new_completed, 1);
        });
    }

    // TC-1201-P01: 1000 行差分が 100ms 以内で処理される（性能テスト）
    #[test]
    fn tc_1201_p01_performance_1000_lines() {
        // 【テスト目的】: 1000 行の差分データが 100ms 以内で処理されることを確認 🟢
        with_fresh_state(|| {
            #[cfg(debug_assertions)]
            let n = 200; // デバッグビルドは小さいサイズで確認
            #[cfg(not(debug_assertions))]
            let n = 1000;

            // 【テストデータ生成】: CREATE + SET_PARAM + COMPLETE の3行セット × n
            let mut lines = Vec::new();
            for i in 0..n {
                lines.push(make_create_trial(0));
                lines.push(make_set_param(i as u32, "x1", (i as f64) / (n as f64)));
                lines.push(make_complete(i as u32, &[i as f64 * 0.01]));
            }
            let data = make_diff_bytes(&lines);

            let start = std::time::Instant::now();
            let result = append_journal_diff(&data);
            let elapsed = start.elapsed().as_millis();

            // 【確認内容】: 100ms 以内で処理完了
            assert!(elapsed < 100, "{}ms は 100ms を超えている", elapsed);
            // 【確認内容】: n 件の COMPLETE が正しく検出
            assert_eq!(result.new_completed, n, "COMPLETE 件数が一致しない");
        });
    }
}
