bitflags! {
    pub flags HighlightParams: u8 {
        const HighlightStrings = (1 << 0),
        const HighlightNumbers = (1 << 1),
    }
}

#[derive(Copy, Clone)]
pub enum HighlightType {
    Normal,
    NonPrint,
    SingleLineComment,
    MultiLineComment,
    Keyword,
    String,
    Number,
    SearchMatch,
}
