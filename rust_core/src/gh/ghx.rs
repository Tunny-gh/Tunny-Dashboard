//! GH_Archive XML（.ghx）の汎用ツリーパーサ。
//!
//! .ghx は Grasshopper の GH_Archive を XML でシリアライズしたもので、
//! `<chunk>`（名前付きノード）と `<item>`（名前・型・値の葉）の再帰構造を持つ。
//! ここではフォーマットの意味論には立ち入らず、ツリーをそのまま
//! `GhxChunk` / `GhxItem` に写し取る。意味論（スライダー・ワイヤ等の解釈）は
//! `problem` モジュールが担う。
//!
//! 注意: GH_Archive の XML 構造は公式ドキュメントがなく、事実上の安定性に
//! 依存している（ROADMAP 項目 15 参照）。パースはタグ名・属性名のみに基づき、
//! 未知の要素は無視する方針で前方互換性を確保する。

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// `<item>` 要素。単純型（gh_double / gh_string / gh_guid 等）の値は
/// `text` にテキスト内容がそのまま入る。複合型（座標・色等）の子要素は
/// 現状の用途では不要なため捨てる。
#[derive(Debug, Clone)]
pub(crate) struct GhxItem {
    pub name: String,
    /// 同名 item が並ぶ場合の連番（例: ワイヤ接続元の `Source`）。
    /// 現状は文書順で十分なため解釈側では未使用（デバッグ・将来用に保持）。
    #[allow(dead_code)]
    pub index: Option<i64>,
    /// GH_IO の型名（gh_double 等）。解釈はテキスト値で行うため未使用。
    #[allow(dead_code)]
    pub type_name: String,
    pub text: String,
}

/// `<chunk>` 要素（ルートの `<Archive>` も便宜上 chunk として扱う）。
#[derive(Debug, Clone)]
pub(crate) struct GhxChunk {
    pub name: String,
    #[allow(dead_code)]
    pub index: Option<i64>,
    pub items: Vec<GhxItem>,
    pub chunks: Vec<GhxChunk>,
}

impl GhxChunk {
    fn new(name: String, index: Option<i64>) -> Self {
        Self {
            name,
            index,
            items: Vec::new(),
            chunks: Vec::new(),
        }
    }

    /// 直下の子 chunk を名前（大文字小文字無視）で探す。
    pub fn find_chunk(&self, name: &str) -> Option<&GhxChunk> {
        self.chunks
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// 直下の子 chunk のうち指定名のものをすべて返す。
    pub fn chunks_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GhxChunk> {
        self.chunks
            .iter()
            .filter(move |c| c.name.eq_ignore_ascii_case(name))
    }

    /// 深さ優先で子孫 chunk を名前で探す（自身は含まない）。
    pub fn find_chunk_recursive(&self, name: &str) -> Option<&GhxChunk> {
        for child in &self.chunks {
            if child.name.eq_ignore_ascii_case(name) {
                return Some(child);
            }
            if let Some(found) = child.find_chunk_recursive(name) {
                return Some(found);
            }
        }
        None
    }

    /// 直下の item を名前（大文字小文字無視）で探す。
    pub fn item(&self, name: &str) -> Option<&GhxItem> {
        self.items
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(name))
    }

    /// 直下の item のテキスト値。
    pub fn item_text(&self, name: &str) -> Option<&str> {
        self.item(name).map(|i| i.text.as_str())
    }

    /// 直下の item を f64 として読む。
    pub fn item_f64(&self, name: &str) -> Option<f64> {
        self.item_text(name).and_then(|t| t.trim().parse().ok())
    }

    /// 直下の item を i64 として読む。
    pub fn item_i64(&self, name: &str) -> Option<i64> {
        self.item_text(name).and_then(|t| t.trim().parse().ok())
    }

    /// 直下の同名 item をすべて返す（index 順は文書順のまま）。
    pub fn items_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GhxItem> {
        self.items
            .iter()
            .filter(move |i| i.name.eq_ignore_ascii_case(name))
    }
}

/// 要素スタック上の種別。テキストの帰属先判定（item 直下のテキストのみ拾う）
/// と chunk の開閉に使う。
enum ElemKind {
    /// `<Archive>` または `<chunk>`（GhxChunk をスタックに積んだ要素）
    Chunk,
    /// `<item>`（current_item が生きている間のみ Item として扱う）
    Item,
    /// それ以外（`<items>` `<chunks>` や複合型 item の子要素等）
    Other,
}

fn attr_of(e: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in e.attributes() {
        let attr = attr.ok()?;
        if attr.key.as_ref() == key {
            return attr.unescape_value().ok().map(|v| v.into_owned());
        }
    }
    None
}

/// .ghx 全体をパースしてルート chunk（`<Archive>`）を返す。
pub(crate) fn parse_archive(xml: &str) -> Result<GhxChunk, String> {
    let mut reader = Reader::from_str(xml);
    let mut elem_stack: Vec<ElemKind> = Vec::new();
    let mut chunk_stack: Vec<GhxChunk> = Vec::new();
    let mut current_item: Option<GhxItem> = None;
    let mut root: Option<GhxChunk> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = e.local_name();
                let tag = tag.as_ref();
                match tag {
                    b"Archive" | b"chunk" => {
                        let name = attr_of(&e, b"name")
                            .unwrap_or_else(|| String::from_utf8_lossy(tag).into_owned());
                        let index = attr_of(&e, b"index").and_then(|v| v.parse().ok());
                        chunk_stack.push(GhxChunk::new(name, index));
                        elem_stack.push(ElemKind::Chunk);
                    }
                    b"item" if current_item.is_none() => {
                        current_item = Some(GhxItem {
                            name: attr_of(&e, b"name").unwrap_or_default(),
                            index: attr_of(&e, b"index").and_then(|v| v.parse().ok()),
                            type_name: attr_of(&e, b"type_name").unwrap_or_default(),
                            text: String::new(),
                        });
                        elem_stack.push(ElemKind::Item);
                    }
                    _ => elem_stack.push(ElemKind::Other),
                }
            }
            Ok(Event::Empty(e)) => {
                // 自己終了要素。空文字列の item がこの形になることがある。
                if e.local_name().as_ref() == b"item" && current_item.is_none() {
                    if let Some(chunk) = chunk_stack.last_mut() {
                        chunk.items.push(GhxItem {
                            name: attr_of(&e, b"name").unwrap_or_default(),
                            index: attr_of(&e, b"index").and_then(|v| v.parse().ok()),
                            type_name: attr_of(&e, b"type_name").unwrap_or_default(),
                            text: String::new(),
                        });
                    }
                }
            }
            Ok(Event::Text(t)) => {
                // item 要素の直下のテキストのみ値として採用する
                // （複合型の子要素内テキストは ElemKind::Other の下なので拾わない）。
                if matches!(elem_stack.last(), Some(ElemKind::Item)) {
                    if let Some(item) = current_item.as_mut() {
                        let decoded = t
                            .decode()
                            .map_err(|e| format!("XML テキストのデコードに失敗: {e}"))?;
                        item.text.push_str(&decoded);
                    }
                }
            }
            Ok(Event::GeneralRef(r)) => {
                // 実体参照（&amp; 等）はテキストと別イベントで届くため、
                // 解決して item のテキストに連結する。
                if matches!(elem_stack.last(), Some(ElemKind::Item)) {
                    if let Some(item) = current_item.as_mut() {
                        let name = r
                            .decode()
                            .map_err(|e| format!("実体参照のデコードに失敗: {e}"))?;
                        if let Ok(Some(c)) = r.resolve_char_ref() {
                            item.text.push(c);
                        } else if let Some(s) = quick_xml::escape::resolve_predefined_entity(&name)
                        {
                            item.text.push_str(s);
                        } else {
                            return Err(format!("未対応の実体参照です: &{name};"));
                        }
                    }
                }
            }
            Ok(Event::End(_)) => match elem_stack.pop() {
                Some(ElemKind::Chunk) => {
                    let finished = chunk_stack
                        .pop()
                        .ok_or_else(|| "chunk の開閉が対応していません".to_string())?;
                    if let Some(parent) = chunk_stack.last_mut() {
                        parent.chunks.push(finished);
                    } else {
                        root = Some(finished);
                    }
                }
                Some(ElemKind::Item) => {
                    if let (Some(mut item), Some(chunk)) =
                        (current_item.take(), chunk_stack.last_mut())
                    {
                        // 複合型 item では子要素間の整形用空白が text に混入する
                        // ため取り除く（単純型の値は前後空白なしで書かれる）。
                        item.text = item.text.trim().to_string();
                        chunk.items.push(item);
                    }
                }
                Some(ElemKind::Other) => {}
                None => return Err("XML 要素の開閉が対応していません".to_string()),
            },
            Ok(Event::Eof) => break,
            Ok(_) => {} // 宣言・コメント・CDATA 等は無視
            Err(e) => return Err(format!(".ghx の XML パースに失敗: {e}")),
        }
    }

    root.ok_or_else(|| ".ghx に <Archive> ルート要素が見つかりません".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_chunks_and_items() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Archive name="Root">
  <!--comment-->
  <items count="1">
    <item name="ArchiveVersion" type_name="gh_version" type_code="80">
      <Major>0</Major><Minor>2</Minor><Revision>2</Revision>
    </item>
  </items>
  <chunks count="1">
    <chunk name="Definition">
      <chunks count="1">
        <chunk name="DefinitionObjects">
          <items count="1">
            <item name="ObjectCount" type_name="gh_int32" type_code="3">2</item>
          </items>
          <chunks count="2">
            <chunk name="Object" index="0">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">57da07bd-ecab-415d-9d86-af36d7073abc</item>
                <item name="Name" type_name="gh_string" type_code="10">Number Slider</item>
              </items>
            </chunk>
            <chunk name="Object" index="1">
              <items count="1">
                <item name="Name" type_name="gh_string" type_code="10">A &amp; B</item>
              </items>
            </chunk>
          </chunks>
        </chunk>
      </chunks>
    </chunk>
  </chunks>
</Archive>"#;
        let root = parse_archive(xml).unwrap();
        assert_eq!(root.name, "Root");
        // 複合型 item の子要素テキストは値に混入しない
        assert_eq!(root.item_text("ArchiveVersion"), Some(""));
        let objects = root
            .find_chunk_recursive("DefinitionObjects")
            .expect("DefinitionObjects");
        assert_eq!(objects.item_i64("ObjectCount"), Some(2));
        let objs: Vec<_> = objects.chunks_named("Object").collect();
        assert_eq!(objs.len(), 2);
        assert_eq!(objs[0].item_text("Name"), Some("Number Slider"));
        assert_eq!(
            objs[0].item_text("GUID"),
            Some("57da07bd-ecab-415d-9d86-af36d7073abc")
        );
        // XML エスケープが解除される
        assert_eq!(objs[1].item_text("Name"), Some("A & B"));
        assert_eq!(objs[1].index, Some(1));
    }

    #[test]
    fn rejects_broken_xml() {
        assert!(parse_archive("<Archive><chunk></Archive>").is_err());
        assert!(parse_archive("no xml at all").is_err());
    }
}
