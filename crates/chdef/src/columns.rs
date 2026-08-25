//! Columns of the CH / BF CSVs. A column has one canonical name, and
//! every other spelling a header may use belongs to a
//! [`ColumnVocabulary`] the caller supplies (`docs/spec/format.md` §2).

/// The spellings one caller accepts for the columns of a CH / BF CSV, and
/// the spelling it writes for each.
///
/// A vocabulary is **data**, not a language chdef knows: chdef ships
/// [`japanese`](ColumnVocabulary::japanese) because the format was
/// extracted from files that use those spellings, and it has no standing
/// one built here lacks (ADR-0024).
///
/// Two rules keep the format a format whatever a caller teaches. A
/// vocabulary only **adds**: cells are matched against the canonical names
/// and their variants first, so teaching `number` to mean something else
/// does nothing. And no vocabulary appears in the golden vectors of
/// `docs/spec/interchange.md` §3, so conformance is defined without one.
#[derive(Debug, Clone, Default)]
pub struct ColumnVocabulary {
    ch: Vec<(String, ChColumn)>,
    bf: Vec<(String, BfColumn)>,
}

impl ColumnVocabulary {
    /// The empty vocabulary: canonical names and their variants alone.
    pub fn new() -> ColumnVocabulary {
        ColumnVocabulary::default()
    }

    /// The spellings of the definition files this format was extracted
    /// from, listed in `docs/spec/format.md` §2.
    pub fn japanese() -> ColumnVocabulary {
        ColumnVocabulary::new()
            .ch("番号", ChColumn::Number)
            .ch("バイト数", ChColumn::Bytes)
            .ch("ビット数", ChColumn::Bits)
            .ch("セクション名", ChColumn::Section)
            .ch("メッセージ名称", ChColumn::Name)
            .ch("信号名称", ChColumn::Name)
            .ch("型", ChColumn::Type)
            .ch("データ型", ChColumn::Type)
            .ch("LSB", ChColumn::Lsb)
            .ch("スケール", ChColumn::Lsb)
            .ch("オフセット", ChColumn::Offset)
            .ch("基準値", ChColumn::Offset)
            .ch("単位", ChColumn::Unit)
            .ch("値(最小)", ChColumn::Min)
            .ch("最小値", ChColumn::Min)
            .ch("値(最大)", ChColumn::Max)
            .ch("最大値", ChColumn::Max)
            .ch("値(デフォルト)", ChColumn::Default)
            .ch("デフォルト値", ChColumn::Default)
            .ch("備考", ChColumn::Memo)
            .ch("変数名", ChColumn::Var)
            .ch("表示形式", ChColumn::Format)
            .ch("お気に入り", ChColumn::Favorite)
            .ch("種別", ChColumn::Kind)
            .ch("算出", ChColumn::Derived)
            .bf("番号", BfColumn::Number)
            .bf("BIT番号", BfColumn::Bit)
            .bf("メッセージ名称", BfColumn::Name)
            .bf("信号名称", BfColumn::Name)
            .bf("値(デフォルト)", BfColumn::Default)
            .bf("デフォルト値", BfColumn::Default)
            .bf("備考", BfColumn::Memo)
    }

    /// Teach a spelling for a CH column. The **first** spelling taught for
    /// a column is the one a file created with this vocabulary is written
    /// with; every one taught is accepted when reading, trimmed and
    /// case-insensitively.
    pub fn ch(mut self, spelling: &str, column: ChColumn) -> ColumnVocabulary {
        self.ch.push((spelling.to_string(), column));
        self
    }

    /// Teach a spelling for a BF column. See [`ch`](Self::ch).
    pub fn bf(mut self, spelling: &str, column: BfColumn) -> ColumnVocabulary {
        self.bf.push((spelling.to_string(), column));
        self
    }

    /// This vocabulary, then everything `other` teaches. Where both name
    /// the same spelling or the same column, this one wins.
    pub fn with(mut self, other: &ColumnVocabulary) -> ColumnVocabulary {
        self.ch.extend(other.ch.iter().cloned());
        self.bf.extend(other.bf.iter().cloned());
        self
    }

    /// The spelling this vocabulary writes for a CH column: the first it
    /// was taught, or the canonical name when it was taught none.
    pub fn ch_spelling(&self, column: ChColumn) -> &str {
        Self::written(&self.ch, column).unwrap_or_else(|| column.name())
    }

    /// The spelling this vocabulary writes for a BF column. See
    /// [`ch_spelling`](Self::ch_spelling).
    pub fn bf_spelling(&self, column: BfColumn) -> &str {
        Self::written(&self.bf, column).unwrap_or_else(|| column.name())
    }

    /// The CH column a header cell denotes: what the specification says,
    /// and only if it says nothing, what this vocabulary teaches.
    pub(crate) fn ch_column(&self, cell: &str) -> Option<ChColumn> {
        ChColumn::from_header(cell).or_else(|| Self::taught(&self.ch, cell))
    }

    /// The BF column a header cell denotes. See [`ch_column`](Self::ch_column).
    pub(crate) fn bf_column(&self, cell: &str) -> Option<BfColumn> {
        BfColumn::from_header(cell).or_else(|| Self::taught(&self.bf, cell))
    }

    fn written<C: Copy + PartialEq>(table: &[(String, C)], column: C) -> Option<&str> {
        table
            .iter()
            .find(|(_, c)| *c == column)
            .map(|(spelling, _)| spelling.as_str())
    }

    fn taught<C: Copy>(table: &[(String, C)], cell: &str) -> Option<C> {
        let cell = cell.trim().to_lowercase();
        table
            .iter()
            .find(|(spelling, _)| spelling.trim().to_lowercase() == cell)
            .map(|(_, column)| *column)
    }
}

/// Whether a header cell denotes `column`, by the canonical name or one of
/// its variants. Trimmed and matched case-insensitively.
fn denotes(cell: &str, name: &str, variants: &[&str]) -> bool {
    name.to_lowercase() == cell || variants.iter().any(|v| v.to_lowercase() == cell)
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
    Kind,
    Derived,
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

    /// The canonical name of this column — its identity throughout the
    /// specification and this crate.
    pub fn name(self) -> &'static str {
        match self {
            ChColumn::Number => "number",
            ChColumn::Bytes => "bytes",
            ChColumn::Bits => "bits",
            ChColumn::Section => "section",
            ChColumn::Name => "name",
            ChColumn::Type => "type",
            ChColumn::Lsb => "lsb",
            ChColumn::Offset => "offset",
            ChColumn::Unit => "unit",
            ChColumn::Min => "min",
            ChColumn::Max => "max",
            ChColumn::Default => "default",
            ChColumn::Memo => "memo",
            ChColumn::Var => "var",
            ChColumn::Format => "format",
            ChColumn::Favorite => "favorite",
            ChColumn::Kind => "kind",
            ChColumn::Derived => "derived",
        }
    }

    /// The other spellings of the canonical name itself, recognised with
    /// no vocabulary at all (`docs/spec/format.md` §3).
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            ChColumn::Number => &["no", "ch", "chnumber"],
            ChColumn::Name => &["signalname"],
            ChColumn::Type => &["datatype"],
            ChColumn::Lsb => &["scale"],
            ChColumn::Default => &["defaultvalue"],
            ChColumn::Memo => &["description"],
            ChColumn::Var => &["variable"],
            ChColumn::Format => &["displayformat"],
            ChColumn::Favorite => &["isfavorite"],
            _ => &[],
        }
    }

    pub(crate) const CANONICAL: [ChColumn; 18] = [
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
        ChColumn::Kind,
        ChColumn::Derived,
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

    /// The column a header cell denotes, if any: the canonical name or
    /// one of its variants (`docs/spec/format.md` §2). A spelling beyond
    /// these belongs to a [`ColumnVocabulary`].
    pub fn from_header(cell: &str) -> Option<ChColumn> {
        let cell = cell.trim().to_lowercase();
        ChColumn::CANONICAL
            .into_iter()
            .find(|c| denotes(&cell, c.name(), c.variants()))
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

    /// The canonical name of this column — its identity throughout the
    /// specification and this crate.
    pub fn name(self) -> &'static str {
        match self {
            BfColumn::Number => "number",
            BfColumn::Bit => "bit",
            BfColumn::Name => "name",
            BfColumn::Default => "default",
            BfColumn::Memo => "memo",
        }
    }

    /// The other spellings of the canonical name itself, recognised with
    /// no vocabulary at all (`docs/spec/format.md` §4).
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            BfColumn::Number => &["no", "ch"],
            BfColumn::Bit => &["bitnumber"],
            BfColumn::Name => &["signalname"],
            _ => &[],
        }
    }

    pub(crate) const CANONICAL: [BfColumn; 5] = [
        BfColumn::Number,
        BfColumn::Bit,
        BfColumn::Name,
        BfColumn::Default,
        BfColumn::Memo,
    ];

    /// The column a header cell denotes, if any. See
    /// [`ChColumn::from_header`].
    pub fn from_header(cell: &str) -> Option<BfColumn> {
        let cell = cell.trim().to_lowercase();
        BfColumn::CANONICAL
            .into_iter()
            .find(|c| denotes(&cell, c.name(), c.variants()))
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
    /// Identify CH columns from a header row, by the canonical
    /// names and variants and any spelling the vocabulary teaches. `None` when the
    /// row holds no `number` column, i.e. it is not a header.
    pub fn ch_from_header(cells: &[&str], vocabulary: &ColumnVocabulary) -> Option<Self> {
        let map = Self::from_cells(cells, |cell| vocabulary.ch_column(cell));
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
    pub fn bf_from_header(cells: &[&str], vocabulary: &ColumnVocabulary) -> Option<Self> {
        let map = Self::from_cells(cells, |cell| vocabulary.bf_column(cell));
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
    fn a_canonical_name_and_its_variants_denote_the_same_column() {
        assert_eq!(ChColumn::from_header("number"), Some(ChColumn::Number));
        assert_eq!(ChColumn::from_header("ChNumber"), Some(ChColumn::Number));
        assert_eq!(ChColumn::from_header(" Default "), Some(ChColumn::Default));
        assert_eq!(
            ChColumn::from_header("DisplayFormat"),
            Some(ChColumn::Format)
        );
        assert_eq!(ChColumn::from_header("unknown"), None);
        assert_eq!(BfColumn::from_header("bit"), Some(BfColumn::Bit));
        assert_eq!(BfColumn::from_header("BitNumber"), Some(BfColumn::Bit));
    }

    #[test]
    fn a_spelling_of_no_vocabulary_denotes_nothing_on_its_own() {
        // ADR-0024: Japanese is a vocabulary, so it is not recognised until
        // one is asked for.
        assert_eq!(ChColumn::from_header("番号"), None);
        assert_eq!(BfColumn::from_header("BIT番号"), None);
        assert_eq!(
            ColumnVocabulary::japanese().ch_column("番号"),
            Some(ChColumn::Number)
        );
        assert_eq!(
            ColumnVocabulary::japanese().bf_column("BIT番号"),
            Some(BfColumn::Bit)
        );
    }

    #[test]
    fn every_canonical_name_and_variant_round_trips() {
        for c in ChColumn::CANONICAL {
            assert_eq!(ChColumn::from_header(c.name()), Some(c), "{c:?}");
            for variant in c.variants() {
                assert_eq!(ChColumn::from_header(variant), Some(c), "{variant}");
            }
        }
        for c in BfColumn::CANONICAL {
            assert_eq!(BfColumn::from_header(c.name()), Some(c), "{c:?}");
            for variant in c.variants() {
                assert_eq!(BfColumn::from_header(variant), Some(c), "{variant}");
            }
        }
    }

    #[test]
    fn header_map_locates_columns_wherever_they_are() {
        let map = ColumnMap::ch_from_header(
            &["name", "number", "default", "extra"],
            &ColumnVocabulary::new(),
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
            ColumnMap::ch_from_header(&["1", "4", "", "General"], &ColumnVocabulary::new())
                .is_none()
        );
        assert!(
            ColumnMap::bf_from_header(&["2", "0", "Reserved"], &ColumnVocabulary::new()).is_none()
        );
        let positional = ColumnMap::ch_positional();
        assert!(positional.assumed);
        assert_eq!(positional.position(ChColumn::Unit), Some(8));
        assert_eq!(positional.position(ChColumn::Default), None);
    }

    #[test]
    fn duplicate_header_names_keep_the_first_position() {
        let map = ColumnMap::ch_from_header(&["number", "name", "name"], &ColumnVocabulary::new())
            .unwrap();
        assert_eq!(map.position(ChColumn::Name), Some(1));
    }
}
