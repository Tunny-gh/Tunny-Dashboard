//! Generic tree parser for GH_Archive XML (.ghx).
//!
//! .ghx is Grasshopper's GH_Archive serialized as XML; it has a recursive
//! structure of `<chunk>` (named nodes) and `<item>` (name/type/value
//! leaves). This module doesn't concern itself with the format's semantics —
//! it simply copies the tree into `GhxChunk` / `GhxItem`. Semantics
//! (interpreting sliders, wires, etc.) are the `problem` module's job.
//!
//! Note: the GH_Archive XML structure has no official documentation and
//! relies on de facto stability (see ROADMAP item 15). Parsing is based only
//! on tag and attribute names, and unknown elements are ignored, to ensure
//! forward compatibility.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// An `<item>` element. For simple types (gh_double / gh_string / gh_guid,
/// etc.) the value's text content goes straight into `text`. Child elements
/// of compound types (coordinates, colors, etc.) are discarded since they
/// aren't needed for the current use case.
#[derive(Debug, Clone)]
pub(crate) struct GhxItem {
    pub name: String,
    /// Sequence number when multiple items share a name (e.g. wire source
    /// `Source`). Currently unused on the interpreting side since document
    /// order is sufficient (kept for debugging / future use).
    #[allow(dead_code)]
    pub index: Option<i64>,
    /// GH_IO type name (gh_double, etc.). Unused since interpretation is done via the text value.
    #[allow(dead_code)]
    pub type_name: String,
    pub text: String,
}

/// A `<chunk>` element (the root `<Archive>` is also treated as a chunk for convenience).
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

    /// Finds a direct child chunk by name (case-insensitive).
    pub fn find_chunk(&self, name: &str) -> Option<&GhxChunk> {
        self.chunks
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Returns all direct child chunks with the given name.
    pub fn chunks_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GhxChunk> {
        self.chunks
            .iter()
            .filter(move |c| c.name.eq_ignore_ascii_case(name))
    }

    /// Depth-first search for a descendant chunk by name (does not include self).
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

    /// Finds a direct child item by name (case-insensitive).
    pub fn item(&self, name: &str) -> Option<&GhxItem> {
        self.items
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(name))
    }

    /// The text value of a direct child item.
    pub fn item_text(&self, name: &str) -> Option<&str> {
        self.item(name).map(|i| i.text.as_str())
    }

    /// Reads a direct child item as f64.
    pub fn item_f64(&self, name: &str) -> Option<f64> {
        self.item_text(name).and_then(|t| t.trim().parse().ok())
    }

    /// Reads a direct child item as i64.
    pub fn item_i64(&self, name: &str) -> Option<i64> {
        self.item_text(name).and_then(|t| t.trim().parse().ok())
    }

    /// Returns all direct child items with the same name (index order follows document order).
    pub fn items_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GhxItem> {
        self.items
            .iter()
            .filter(move |i| i.name.eq_ignore_ascii_case(name))
    }
}

/// The kind of element on the element stack. Used both to determine where
/// text belongs (only text directly under an item is captured) and to track
/// chunk open/close.
enum ElemKind {
    /// `<Archive>` or `<chunk>` (an element whose GhxChunk was pushed onto the stack)
    Chunk,
    /// `<item>` (treated as an Item only while current_item is alive)
    Item,
    /// Anything else (`<items>`, `<chunks>`, child elements of compound-type items, etc.)
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

/// Parses the entire .ghx and returns the root chunk (`<Archive>`).
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
                // Self-closing element. An empty-string item can take this form.
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
                // Only text directly under an item element is adopted as
                // the value (text inside a compound type's child elements
                // falls under ElemKind::Other and is not captured).
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
                // Entity references (&amp; etc.) arrive as a separate event
                // from text, so resolve them and append to the item's text.
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
                        // For compound-type items, formatting whitespace between child
                        // elements ends up mixed into text, so trim it (simple-type values
                        // are written without leading/trailing whitespace).
                        item.text = item.text.trim().to_string();
                        chunk.items.push(item);
                    }
                }
                Some(ElemKind::Other) => {}
                None => return Err("XML 要素の開閉が対応していません".to_string()),
            },
            Ok(Event::Eof) => break,
            Ok(_) => {} // ignore declarations, comments, CDATA, etc.
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
        // A compound-type item's child element text doesn't leak into the value
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
        // XML escapes are unescaped
        assert_eq!(objs[1].item_text("Name"), Some("A & B"));
        assert_eq!(objs[1].index, Some(1));
    }

    #[test]
    fn rejects_broken_xml() {
        assert!(parse_archive("<Archive><chunk></Archive>").is_err());
        assert!(parse_archive("no xml at all").is_err());
    }
}
