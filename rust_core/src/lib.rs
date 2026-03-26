#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod journal_parser;
pub mod dataframe;
pub mod filter;
pub mod pareto;
pub mod clustering;
pub mod sensitivity;
pub mod pdp;
pub mod sampling;
pub mod export;
pub mod live_update;

/// WASM初期化時にパニックハンドラをセットアップする
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    // パニック時のデバッグ情報をコンソールに出力
    #[cfg(debug_assertions)]
    console_error_panic_hook();
}

#[cfg(feature = "wasm")]
fn console_error_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("WASM Panic: {}", info).into());
    }));
}

#[cfg(test)]
mod tests {
    #[test]
    fn lib_compiles() {
        // 基本的なコンパイル確認
        assert!(true);
    }
}
