//! Columns of the CH / BF CSVs, independent of the language their header is
//! spelled in. Every column has an English and a Japanese canonical spelling
//! plus aliases; header cells are matched case-insensitively after trimming.

/// Which of a column's two canonical spellings to write. A file chdef
/// creates uses one of these throughout; a file it reads keeps whatever
/// spellings it had, mixed languages included
/// (`docs/spec/format.md` §2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum HeaderLanguage {
    /// Lower-case ASCII: `number`, `bytes`, … — the default for a new file.
    #[default]
    En,
    /// The original Japanese form: `番号`, `バイト数`, ….
    Ja,
}

/// Spellings one reader accepts on top of the ones
/// `docs/spec/format.md` §2 defines, for files whose headers word a column
/// their own way.
///
/// An alias only ever **adds** a spelling. A canonical spelling and a
/// documented alias always denote the column the specification says they
/// do — teaching `number` to mean something else does nothing — and no
/// alias reaches the writer: a file keeps the header it was read with, and
/// a file chdef creates uses the canonical spellings. The golden vectors
/// of `docs/spec/interchange.md` §3 never use one either, so what an
/// implementation must do to conform is unchanged by anything a consumer
/// teaches its own reader.
#[derive(Debug, Clone, Default)]
pub struct ColumnAliases {
    ch: Vec<(String, ChColumn)>,
    bf: Vec<(String, BfColumn)>,
}

impl ColumnAliases {
    /// No aliases: the canonical vocabulary alone.
    pub fn new() -> ColumnAliases {
        ColumnAliases::default()
    }

    /// Accept `spelling` as a name for a CH column, matched trimmed and
    /// case-insensitively like every other spelling. Teaching the same
    /// spelling twice keeps the last.
    pub fn ch(mut self, spelling: &str, column: ChColumn) -> ColumnAliases {
        let key = spelling.trim().to_lowercase();
        self.ch.retain(|(k, _)| *k != key);
        self.ch.push((key, column));
        self
    }

    /// Accept `spelling` as a name for a BF column. See [`ch`](Self::ch).
    pub fn bf(mut self, spelling: &str, column: BfColumn) -> ColumnAliases {
        let key = spelling.trim().to_lowercase();
        self.bf.retain(|(k, _)| *k != key);
        self.bf.push((key, column));
        self
    }

    /// The CH column a header cell denotes: what the specification says,
    /// and only if it says nothing, what this reader was taught.
    pub(crate) fn ch_column(&self, cell: &str) -> Option<ChColumn> {
        ChColumn::from_header(cell).or_else(|| Self::taught(&self.ch, cell))
    }

    /// The BF column a header cell denotes. See [`ch_column`](Self::ch_column).
    pub(crate) fn bf_column(&self, cell: &str) -> Option<BfColumn> {
        BfColumn::from_header(cell).or_else(|| Self::taught(&self.bf, cell))
    }

    fn taught<C: Copy>(table: &[(String, C)], cell: &str) -> Option<C> {
        let cell = cell.trim().to_lowercase();
        table
            .iter()
            .rev()
            .find(|(spelling, _)| *spelling == cell)
            .map(|(_, column)| *column)
    }
}

/// A column of a CH CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChColumn {
    Number,
    Bytes,
    Bits,
    Section,
    Name,
    Type,
    Lsb,
    Offset,
    Unit,
    Min,
    Max,
    Default,
    Memo,
    Var,
    Format,
    Favorite,
}

impl ChColumn {
    /// The columns in canonical order — the header a new file gets unless
    /// the caller names another set.
    pub fn canonical() -> &'static [ChColumn] {
        &Self::CANONICAL
    }

    /// The columns assumed, in order, for a file with no recognisable
    /// header (`docs/spec/format.md` §2).
    pub fn positional() -> &'static [ChColumn] {
        &Self::POSITIONAL
    }

    /// The canonical spelling of this column in the given language.
    pub fn name(self, lang: HeaderLanguage) -> &'static str {
        match lang {
            HeaderLanguage::En => self.spellings()[0],
            HeaderLanguage::Ja => self.spellings()[1],
        }
    }

    pub(crate) const CANONICAL: [ChColumn; 16] = [
        ChColumn::Number,
        ChColumn::Bytes,
        ChColumn::Bits,
        ChColumn::Section,
        ChColumn::Name,
        ChColumn::Type,
        ChColumn::Lsb,
        ChColumn::Offset,
        ChColumn::Unit,
        ChColumn::Min,
        ChColumn::Max,
        ChColumn::Default,
        ChColumn::Memo,
        ChColumn::Var,
        ChColumn::Format,
        ChColumn::Favorite,
    ];

    pub(crate) const POSITIONAL: [ChColumn; 9] = [
        ChColumn::Number,
        ChColumn::Bytes,
        ChColumn::Bits,
        ChColumn::Section,
        ChColumn::Name,
        ChColumn::Type,
        ChColumn::Lsb,
        ChColumn::Offset,
        ChColumn::Unit,
    ];

    /// English canonical, Japanese canonical, then aliases.
    fn spellings(self) -> &'static [&'static str] {
        match self {
            ChColumn::Number => &["number", "番号", "no", "ch", "chnumber"],
            ChColumn::Bytes => &["bytes", "バイト数"],
            ChColumn::Bits => &["bits", "ビット数"],
            ChColumn::Section => &["section", "セクション名"],
            ChColumn::Name => &["name", "メッセージ名称", "signalname", "信号名称"],
            ChColumn::Type => &["type", "型", "datatype", "データ型"],
            ChColumn::Lsb => &["lsb", "LSB", "scale", "スケール"],
            ChColumn::Offset => &["offset", "オフセット", "基準値"],
            ChColumn::Unit => &["unit", "単位"],
            ChColumn::Min => &["min", "値(最小)", "最小値"],
            ChColumn::Max => &["max", "値(最大)", "最大値"],
            ChColumn::Default => &["default", "値(デフォルト)", "デフォルト値", "defaultvalue"],
            ChColumn::Memo => &["memo", "備考", "description"],
            ChColumn::Var => &["var", "変数名", "variable"],
            ChColumn::Format => &["format", "表示形式", "displayformat"],
            ChColumn::Favorite => &["favorite", "お気に入り", "isfavorite"],
        }
    }

    /// The column a header cell denotes, if any. Cells are trimmed and
    /// matched case-insensitively against both canonical spellings and the
    /// aliases (`docs/spec/format.md` §2).
    pub fn from_header(cell: &str) -> Option<ChColumn> {
        let cell = cell.trim().to_lowercase();
        ChColumn::CANONICAL
            .into_iter()
            .find(|c| c.spellings().iter().any(|s| s.to_lowercase() == cell))
    }
}

/// A column of a BF CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BfColumn {
    Number,
    Bit,
    Name,
    Default,
    Memo,
}

impl BfColumn {
    /// The columns in canonical order — the header a new file gets unless
    /// the caller names another set.
    pub fn canonical() -> &'static [BfColumn] {
        &Self::CANONICAL
    }

    /// The canonical spelling of this column in the given language.
    pub fn name(self, lang: HeaderLanguage) -> &'static str {
        match lang {
            HeaderLanguage::En => self.spellings()[0],
            HeaderLanguage::Ja => self.spellings()[1],
        }
    }

    pub(crate) const CANONICAL: [BfColumn; 5] = [
        BfColumn::Number,
        BfColumn::Bit,
        BfColumn::Name,
        BfColumn::Default,
        BfColumn::Memo,
    ];

    fn spellings(self) -> &'static [&'static str] {
        match self {
            BfColumn::Number => &["number", "番号", "no", "ch"],
            BfColumn::Bit => &["bit", "BIT番号", "bitnumber"],
            BfColumn::Name => &["name", "メッセージ名称", "signalname", "信号名称"],
            BfColumn::Default => &["default", "値(デフォルト)", "デフォルト値"],
            BfColumn::Memo => &["memo", "備考"],
        }
    }

    /// The column a header cell denotes, if any. Cells are trimmed and
    /// matched case-insensitively against both canonical spellings and the
    /// aliases (`docs/spec/format.md` §2).
    pub fn from_header(cell: &str) -> Option<BfColumn> {
        let cell = cell.trim().to_lowercase();
        BfColumn::CANONICAL
            .into_iter()
            .find(|c| c.spellings().iter().any(|s| s.to_lowercase() == cell))
    }
}

/// Where each recognised column sits in a particular file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMap<C> {
    positions: Vec<(C, usize)>,
    /// True when the file had no recognisable header and the positional
    /// order was assumed.
    pub assumed: bool,
}

impl<C: Copy + PartialEq> ColumnMap<C> {
    /// The map of a header a caller chose: each column at its own position,
    /// in the order given.
    pub(crate) fn in_order(columns: &[C]) -> Self {
        Self::new(columns.iter().copied().zip(0..).collect(), false)
    }

    fn new(positions: Vec<(C, usize)>, assumed: bool) -> Self {
        ColumnMap { positions, assumed }
    }

    /// Column position of `column`, if the file has it.
    pub fn position(&self, column: C) -> Option<usize> {
        self.positions
            .iter()
            .find(|(c, _)| *c == column)
            .map(|(_, i)| *i)
    }
}

impl ColumnMap<ChColumn> {
    /// Identify CH columns from a header row, by the spellings the
    /// specification defines and any the reader was taught. `None` when the
    /// row holds no `number` column, i.e. it is not a header.
    pub fn ch_from_header(cells: &[&str], aliases: &ColumnAliases) -> Option<Self> {
        let map = Self::from_cells(cells, |cell| aliases.ch_column(cell));
        map.position(ChColumn::Number).map(|_| map)
    }

    /// The positional CH map used when a file has no recognisable header.
    pub fn ch_positional() -> Self {
        Self::new(
            ChColumn::POSITIONAL
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c, i))
                .collect(),
            true,
        )
    }
}

impl ColumnMap<BfColumn> {
    /// Identify BF columns from a header row. See
    /// [`ch_from_header`](ColumnMap::ch_from_header).
    pub fn bf_from_header(cells: &[&str], aliases: &ColumnAliases) -> Option<Self> {
        let map = Self::from_cells(cells, |cell| aliases.bf_column(cell));
        map.position(BfColumn::Number).map(|_| map)
    }

    /// The positional BF map used when a file has no recognisable header.
    pub fn bf_positional() -> Self {
        Self::new(
            BfColumn::CANONICAL
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c, i))
                .collect(),
            true,
        )
    }
}

impl<C: Copy + PartialEq> ColumnMap<C> {
    fn from_cells(cells: &[&str], recognise: impl Fn(&str) -> Option<C>) -> Self {
        let mut positions: Vec<(C, usize)> = Vec::new();
        for (i, cell) in cells.iter().enumerate() {
            if let Some(c) = recognise(cell) {
                if !positions.iter().any(|(seen, _)| *seen == c) {
                    positions.push((c, i));
                }
            }
        }
        Self::new(positions, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_japanese_spellings_denote_the_same_column() {
        assert_eq!(ChColumn::from_header("number"), Some(ChColumn::Number));
        assert_eq!(ChColumn::from_header("番号"), Some(ChColumn::Number));
        assert_eq!(ChColumn::from_header(" Default "), Some(ChColumn::Default));
        assert_eq!(
            ChColumn::from_header("値(デフォルト)"),
            Some(ChColumn::Default)
        );
        assert_eq!(
            ChColumn::from_header("DisplayFormat"),
            Some(ChColumn::Format)
        );
        assert_eq!(ChColumn::from_header("unknown"), None);
        assert_eq!(BfColumn::from_header("BIT番号"), Some(BfColumn::Bit));
        assert_eq!(BfColumn::from_header("bit"), Some(BfColumn::Bit));
    }

    #[test]
    fn canonical_names_round_trip_in_both_languages() {
        for lang in [HeaderLanguage::En, HeaderLanguage::Ja] {
            for c in ChColumn::CANONICAL {
                assert_eq!(ChColumn::from_header(c.name(lang)), Some(c));
            }
            for c in BfColumn::CANONICAL {
                assert_eq!(BfColumn::from_header(c.name(lang)), Some(c));
            }
        }
    }

    #[test]
    fn header_map_locates_columns_wherever_they_are() {
        let map = ColumnMap::ch_from_header(
            &["name", "number", "default", "extra"],
            &ColumnAliases::new(),
        )
        .unwrap();
        assert_eq!(map.position(ChColumn::Number), Some(1));
        assert_eq!(map.position(ChColumn::Default), Some(2));
        assert_eq!(map.position(ChColumn::Bytes), None);
        assert!(!map.assumed);
    }

    #[test]
    fn a_row_without_number_column_is_not_a_header() {
        assert!(
            ColumnMap::ch_from_header(&["1", "4", "", "General"], &ColumnAliases::new()).is_none()
        );
        assert!(
            ColumnMap::bf_from_header(&["2", "0", "Reserved"], &ColumnAliases::new()).is_none()
        );
        let positional = ColumnMap::ch_positional();
        assert!(positional.assumed);
        assert_eq!(positional.position(ChColumn::Unit), Some(8));
        assert_eq!(positional.position(ChColumn::Default), None);
    }

    #[test]
    fn duplicate_header_names_keep_the_first_position() {
        let map =
            ColumnMap::ch_from_header(&["number", "name", "name"], &ColumnAliases::new()).unwrap();
        assert_eq!(map.position(ChColumn::Name), Some(1));
    }
}
