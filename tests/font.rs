extern crate font_parse;
use font_parse::Font;

#[test]
fn test_font_collection_02() {
    let data = include_bytes!("font_files/02font-collection-broken.otc");
    let _ = Font::from_bytes(data, 0);
}
