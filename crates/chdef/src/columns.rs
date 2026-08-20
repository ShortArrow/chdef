//! Columns of the CH / BF CSVs, independent of the language their header is
//! spelled in. Every column has an English and a Japanese canonical spelling
//! plus aliases; header cells are matched case-insensitively after trimming.

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
    /// The 16 columns in canonical order.
    pub const CANONICAL: [ChColumn; 16] = [
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

    /// The columns assumed, in order, when a file has no recognisable header.
    pub const POSITIONAL: [ChColumn; 9] = [
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

    /// The column a header cell denotes, if any.
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
    /// The 5 columns in canonical order.
    pub const CANONICAL: [BfColumn; 5] = [
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

    /// The column a header cell denotes, if any.
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
    /// Identify CH columns from a header row. `None` when the row holds no
    /// `number` column, i.e. it is not a header.
    pub fn ch_from_header(cells: &[&str]) -> Option<Self> {
        let map = Self::from_cells(cells, ChColumn::from_header);
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
    /// Identify BF columns from a header row. `None` when the row holds no
    /// `number` column, i.e. it is not a header.
    pub fn bf_from_header(cells: &[&str]) -> Option<Self> {
        let map = Self::from_cells(cells, BfColumn::from_header);
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
    fn from_cells(cells: &[&str], recognise: fn(&str) -> Option<C>) -> Self {
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
        for lang in [0, 1] {
            for c in ChColumn::CANONICAL {
                assert_eq!(ChColumn::from_header(c.spellings()[lang]), Some(c));
            }
            for c in BfColumn::CANONICAL {
                assert_eq!(BfColumn::from_header(c.spellings()[lang]), Some(c));
            }
        }
    }

    #[test]
    fn header_map_locates_columns_wherever_they_are() {
        let map = ColumnMap::ch_from_header(&["name", "number", "default", "extra"]).unwrap();
        assert_eq!(map.position(ChColumn::Number), Some(1));
        assert_eq!(map.position(ChColumn::Default), Some(2));
        assert_eq!(map.position(ChColumn::Bytes), None);
        assert!(!map.assumed);
    }

    #[test]
    fn a_row_without_number_column_is_not_a_header() {
        assert!(ColumnMap::ch_from_header(&["1", "4", "", "General"]).is_none());
        assert!(ColumnMap::bf_from_header(&["2", "0", "Reserved"]).is_none());
        let positional = ColumnMap::ch_positional();
        assert!(positional.assumed);
        assert_eq!(positional.position(ChColumn::Unit), Some(8));
        assert_eq!(positional.position(ChColumn::Default), None);
    }

    #[test]
    fn duplicate_header_names_keep_the_first_position() {
        let map = ColumnMap::ch_from_header(&["number", "name", "name"]).unwrap();
        assert_eq!(map.position(ChColumn::Name), Some(1));
    }
}
