//! The Table stage (`docs/spec/layout.md` §1): a CSV file as its cells.
//!
//! [`Grid`] is the file with nothing interpreted — the header row and every
//! data row as verbatim strings, unknown columns and comment rows included.
//! [`ChTable`] and [`BfTable`] are a grid plus a column vocabulary, and add
//! the operations that need one. All three write the file back in the shape
//! it was read (`docs/spec/editing.md` §2); Rows and Layout are derived
//! views, interpreted again after each edit.

use crate::channel::{BitFieldDef, ChannelDef};
use crate::columns::{BfColumn, ChColumn, ColumnMap, ColumnVocabulary};
use crate::csv::{decode_utf8, interpret_bf, interpret_ch, is_blank, is_comment};
use crate::error::{ChdefError, Result};
use crate::issue::{Issue, IssueCode, Parsed};

/// How a CSV file separates its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// `\r\n`, what chdef writes into a file it creates
    /// (`docs/spec/format.md` §1).
    #[default]
    Crlf,
    /// `\n`.
    Lf,
}

impl LineEnding {
    fn as_str(self) -> &'static str {
        match self {
            LineEnding::Crlf => "\r\n",
            LineEnding::Lf => "\n",
        }
    }
}

/// The shape of a CSV file, apart from its cells: whether it starts with a
/// byte-order mark and how it separates records. A grid reads this from the
/// file it parsed and writes it back, so editing one cell of a file kept
/// with LF endings does not rewrite every line of it. The default — what a
/// grid created in code uses — is the write column of
/// `docs/spec/format.md` §1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvStyle {
    pub bom: bool,
    pub line_ending: LineEnding,
}

impl Default for CsvStyle {
    /// The write column of `docs/spec/format.md` §1: a byte-order mark, so
    /// spreadsheet software does not guess another encoding, and CRLF.
    fn default() -> CsvStyle {
        CsvStyle {
            bom: true,
            line_ending: LineEnding::Crlf,
        }
    }
}

/// What [`ChTable::insert_channel_renumbering`] moved: `(old, new)` per
/// renumbered channel, ascending by old number. The consumer repairs or
/// announces its own references with it — chdef does not know them.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renumbered {
    pub moved: Vec<(u32, u32)>,
}

/// A CSV file as its cells, with nothing interpreted: a consumer that
/// displays or edits a definition without reading its columns needs no more
/// than this, and needs not know whether the file is a CH or a BF CSV.
///
/// Whether the first record is a header is the one interpretation a grid
/// makes, and it makes it the simple way: [`parse`](Grid::parse) takes the
/// first record as the header. Deciding it from the column vocabulary —
/// which is what `docs/spec/format.md` §2 specifies, and what lets a
/// headerless file be read positionally — is [`ChTable`]'s and
/// [`BfTable`]'s.
#[derive(Debug, Clone, Default)]
pub struct Grid {
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
    style: CsvStyle,
}

impl Grid {
    /// An empty grid with no header, writing the defaults of
    /// `docs/spec/format.md` §1.
    pub fn new() -> Grid {
        Grid {
            header: None,
            rows: Vec::new(),
            style: CsvStyle::default(),
        }
    }

    /// Parse CSV text, taking the first record as the header. Only a
    /// structurally broken file fails (`docs/spec/diagnostics.md` §1);
    /// everything interpretable-or-not is kept as cells.
    pub fn parse(content: &str) -> Result<Grid> {
        Grid::split(content, |records| !records.is_empty())
    }

    /// [`parse`](Grid::parse) after stripping BOMs and decoding UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Grid> {
        Grid::parse(decode_utf8(bytes)?)
    }

    /// Read the records of `content` and let `has_header` decide, from all
    /// of them, whether the first is a header.
    fn split(content: &str, mut has_header: impl FnMut(&[Vec<String>]) -> bool) -> Result<Grid> {
        let bom = content.starts_with('\u{FEFF}');
        let content = content.trim_start_matches('\u{FEFF}');
        let shape = scan_shape(content);
        if let Some(line) = shape.unterminated_on {
            return Err(ChdefError::CsvParse {
                line,
                message:
                    "a quoted cell is never closed, so the rest of the file was read as part of it"
                        .to_string(),
            });
        }

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content.as_bytes());
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (index, record) in rdr.records().enumerate() {
            let record = record.map_err(|e| ChdefError::CsvParse {
                line: e.position().map(|p| p.line() as usize).unwrap_or(index + 1),
                message: e.to_string(),
            })?;
            rows.push(record.iter().map(str::to_string).collect());
        }

        let header = has_header(&rows).then(|| rows.remove(0));
        Ok(Grid {
            header,
            rows,
            style: CsvStyle {
                bom,
                line_ending: shape.line_ending.unwrap_or_default(),
            },
        })
    }

    /// A grid with the given header row and no data rows.
    fn with_header(header: Vec<String>) -> Grid {
        Grid {
            header: Some(header),
            rows: Vec::new(),
            style: CsvStyle::default(),
        }
    }

    /// The header row, or `None` for a file read without one.
    pub fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }

    /// Every data row in order, header excluded — the grid an editor draws.
    /// Comment and blank rows are rows like any other.
    pub fn rows(&self) -> impl Iterator<Item = &[String]> {
        self.rows.iter().map(Vec::as_slice)
    }

    /// One data row, by the same index [`cell`](Grid::cell) uses.
    pub fn row(&self, index: usize) -> Option<&[String]> {
        self.rows.get(index).map(Vec::as_slice)
    }

    /// The cell at `(row, col)` of the data rows, exactly as it will be
    /// written. `None` outside the grid.
    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.rows.get(row)?.get(col).map(String::as_str)
    }

    /// Number of data rows (comment and blank rows included).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Overwrite one cell; a row shorter than `col` is padded with empty
    /// cells. A row outside the grid is ignored.
    pub fn set_cell(&mut self, row: usize, col: usize, value: impl Into<String>) {
        if let Some(cells) = self.rows.get_mut(row) {
            if cells.len() <= col {
                cells.resize(col + 1, String::new());
            }
            cells[col] = value.into();
        }
    }

    /// Insert a row of cells at `index` (clamped to the end).
    pub fn insert_row(&mut self, index: usize, cells: Vec<String>) {
        let index = index.min(self.rows.len());
        self.rows.insert(index, cells);
    }

    /// Append a row of cells.
    pub fn append_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    /// Remove and return the row at `index`; `None` when the grid has no
    /// such row.
    pub fn remove_row(&mut self, index: usize) -> Option<Vec<String>> {
        (index < self.rows.len()).then(|| self.rows.remove(index))
    }

    /// The shape the file is written in — its byte-order mark and record
    /// separator — read from the file this grid was parsed from, or the
    /// write defaults for one created in code.
    pub fn style(&self) -> CsvStyle {
        self.style
    }

    /// Write the file in another shape from now on.
    pub fn set_style(&mut self, style: CsvStyle) {
        self.style = style;
    }

    /// Serialise back to CSV text in this grid's shape: a cell is quoted
    /// only when it holds `,` `"` or a newline
    /// (`docs/spec/format.md` §1). Cells and rows round-trip; the source's
    /// unnecessary quotes are dropped.
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        if self.style.bom {
            out.push('\u{FEFF}');
        }
        let separator = self.style.line_ending.as_str();
        for row in self.header.iter().chain(self.rows.iter()) {
            push_row(&mut out, row, separator);
        }
        out
    }

    /// The grid in the Table JSON shape of `docs/spec/interchange.md` §2:
    /// the header (absent for a file read without one) and every row,
    /// verbatim, unknown columns included.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> crate::interchange::TableJson<'_> {
        crate::interchange::TableJson {
            header: self.header.as_deref(),
            rows: &self.rows,
        }
    }

    /// The data rows as the interpreters read them.
    pub(crate) fn data(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// The data rows, to renumber a `number` cell in place.
    pub(crate) fn data_mut(&mut self) -> &mut [Vec<String>] {
        &mut self.rows
    }

    /// Width for a generated row: the header's, or none.
    fn new_row_width(&self) -> usize {
        self.header.as_ref().map(Vec::len).unwrap_or_default()
    }
}

/// What one quote-aware pass over the text finds: a quoted cell left open
/// at the end of the file, and how the file separates its records.
struct Shape {
    /// The 1-based line on which a quoted cell opens and is never closed.
    /// A quote only opens a cell at the start of one
    /// (`docs/spec/format.md` §1: a quote outside quotes is a literal
    /// character), and a doubled quote inside one is an escaped quote, not
    /// a closing one. The `csv` crate reads such a cell to the end of the
    /// file without complaint, which silently shortens the definition.
    unterminated_on: Option<usize>,
    /// The first record separator outside a quoted cell; `None` for a file
    /// with no complete record.
    line_ending: Option<LineEnding>,
}

fn scan_shape(content: &str) -> Shape {
    let mut line = 1usize;
    let mut opened_on = None;
    let mut line_ending = None;
    let mut at_field_start = true;
    let mut saw_cr = false;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        match opened_on {
            None => match c {
                '"' if at_field_start => opened_on = Some(line),
                ',' => at_field_start = true,
                '\n' => {
                    line_ending.get_or_insert(if saw_cr {
                        LineEnding::Crlf
                    } else {
                        LineEnding::Lf
                    });
                    line += 1;
                    at_field_start = true;
                    saw_cr = false;
                }
                '\r' => saw_cr = true,
                _ => {
                    at_field_start = false;
                    saw_cr = false;
                }
            },
            Some(_) => match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                }
                '"' => {
                    opened_on = None;
                    at_field_start = false;
                }
                '\n' => line += 1,
                _ => {}
            },
        }
    }
    Shape {
        unterminated_on: opened_on,
        line_ending,
    }
}

fn push_row(out: &mut String, row: &[String], separator: &str) {
    for (i, cell) in row.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if cell.contains(['"', ',', '\r', '\n']) {
            out.push('"');
            out.push_str(&cell.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(cell);
        }
    }
    out.push_str(separator);
}

fn shortest(v: f64) -> String {
    format!("{v}")
}

/// The grid operations a typed table forwards to the [`Grid`] it holds.
/// They are the same on both tables and mean the same thing; the macro
/// exists so the documentation lands on each type rather than behind one
/// more indirection.
macro_rules! grid_delegates {
    () => {
        /// See [`Grid::header`].
        pub fn header(&self) -> Option<&[String]> {
            self.grid.header()
        }

        /// See [`Grid::rows`].
        pub fn rows(&self) -> impl Iterator<Item = &[String]> {
            self.grid.rows()
        }

        /// See [`Grid::row`].
        pub fn row(&self, index: usize) -> Option<&[String]> {
            self.grid.row(index)
        }

        /// See [`Grid::cell`].
        pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
            self.grid.cell(row, col)
        }

        /// See [`Grid::row_count`].
        pub fn row_count(&self) -> usize {
            self.grid.row_count()
        }

        /// See [`Grid::set_cell`].
        pub fn set_cell(&mut self, row: usize, col: usize, value: impl Into<String>) {
            self.grid.set_cell(row, col, value)
        }

        /// See [`Grid::insert_row`].
        pub fn insert_row(&mut self, index: usize, cells: Vec<String>) {
            self.grid.insert_row(index, cells)
        }

        /// See [`Grid::append_row`].
        pub fn append_row(&mut self, cells: Vec<String>) {
            self.grid.append_row(cells)
        }

        /// See [`Grid::remove_row`].
        pub fn remove_row(&mut self, index: usize) -> Option<Vec<String>> {
            self.grid.remove_row(index)
        }

        /// See [`Grid::style`].
        pub fn style(&self) -> CsvStyle {
            self.grid.style()
        }

        /// See [`Grid::set_style`].
        pub fn set_style(&mut self, style: CsvStyle) {
            self.grid.set_style(style)
        }

        /// See [`Grid::to_csv`].
        pub fn to_csv(&self) -> String {
            self.grid.to_csv()
        }

        /// See [`Grid::to_json`].
        #[cfg(feature = "serde")]
        pub fn to_json(&self) -> crate::interchange::TableJson<'_> {
            self.grid.to_json()
        }
    };
}

/// A CH CSV: a [`Grid`] plus the columns its header names.
#[derive(Debug, Clone)]
pub struct ChTable {
    grid: Grid,
    map: ColumnMap<ChColumn>,
}

impl ChTable {
    /// Parse CSV text, identifying the columns by the spellings
    /// `docs/spec/format.md` §2 defines. Only a structurally broken file
    /// fails; everything interpretable-or-not is kept as cells.
    pub fn parse(content: &str) -> Result<ChTable> {
        ChTable::parse_with(content, &ColumnVocabulary::new())
    }

    /// [`parse`](ChTable::parse), also accepting the header spellings this
    /// reader was taught ([`ColumnVocabulary`]).
    pub fn parse_with(content: &str, vocabulary: &ColumnVocabulary) -> Result<ChTable> {
        let mut map = None;
        let grid = Grid::split(content, |records| {
            map = records.first().and_then(|first| {
                ColumnMap::ch_from_header(
                    &first.iter().map(String::as_str).collect::<Vec<_>>(),
                    vocabulary,
                )
            });
            map.is_some()
        })?;
        Ok(ChTable {
            grid,
            map: map.unwrap_or_else(ColumnMap::ch_positional),
        })
    }

    /// [`parse`](ChTable::parse) after stripping BOMs and decoding UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<ChTable> {
        ChTable::parse_bytes_with(bytes, &ColumnVocabulary::new())
    }

    /// [`parse_with`](ChTable::parse_with) after stripping BOMs and
    /// decoding UTF-8.
    pub fn parse_bytes_with(bytes: &[u8], vocabulary: &ColumnVocabulary) -> Result<ChTable> {
        ChTable::parse_with(decode_utf8(bytes)?, vocabulary)
    }

    /// An empty table with the canonical columns under their canonical
    /// names — `with_columns(ChColumn::canonical(), &ColumnVocabulary::new())`.
    pub fn new() -> ChTable {
        ChTable::with_columns(ChColumn::canonical(), &ColumnVocabulary::new())
    }

    /// An empty table whose header names `columns`, in that order, in that
    /// vocabulary (ADR-0024). A column the header omits is one
    /// [`insert_channel`](ChTable::insert_channel) drops and
    /// [`channels`](ChTable::channels) reads as unspecified.
    ///
    /// A header that does not name `number` is not read back as a header at
    /// all (`docs/spec/format.md` §2), so a file written from one is read
    /// positionally.
    pub fn with_columns(columns: &[ChColumn], vocabulary: &ColumnVocabulary) -> ChTable {
        ChTable {
            grid: Grid::with_header(
                columns
                    .iter()
                    .map(|c| vocabulary.ch_spelling(*c).to_string())
                    .collect(),
            ),
            map: ColumnMap::in_order(columns),
        }
    }

    /// The cells, with nothing interpreted — for code that wants the grid
    /// and not the columns.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Interpret the current cells as Rows (`docs/spec/format.md` §3) —
    /// the derived view behind [`crate::parse_ch_csv`]. Interpret again
    /// after editing; nothing is cached.
    pub fn channels(&self) -> Parsed<Vec<ChannelDef>> {
        interpret_ch(&self.map, self.grid.data())
    }

    /// Insert `def` as a new data row at `row_index`, rendering the columns
    /// this file has (a field without a column is dropped);
    /// `docs/spec/editing.md` §3 lists the renderings.
    pub fn insert_channel(&mut self, row_index: usize, def: &ChannelDef) {
        let row = self.channel_cells(def);
        self.grid.insert_row(row_index, row);
    }

    /// [`insert_channel`](ChTable::insert_channel), renumbering every
    /// channel whose `number` is ≥ `def.number` up by one first — the
    /// consecutive-numbering insertion. BF rows in `bf` follow their
    /// parents. The returned [`Renumbered`] lists every `(old, new)` pair;
    /// references outside these two files are the consumer's to repair.
    pub fn insert_channel_renumbering(
        &mut self,
        row_index: usize,
        def: &ChannelDef,
        bf: Option<&mut BfTable>,
    ) -> Renumbered {
        let mut moved = Vec::new();
        if let Some(col) = self.map.position(ChColumn::Number) {
            for row in self.grid.data_mut() {
                if is_blank(row) || is_comment(row) {
                    continue;
                }
                let number = row.get(col).and_then(|c| c.trim().parse::<u32>().ok());
                // u32::MAX has nowhere to move to; `format.md` §3 puts no
                // upper bound on `number`, so it is legal input.
                if let Some(n) = number.filter(|n| *n >= def.number && *n < u32::MAX) {
                    row[col] = (n + 1).to_string();
                    if !moved.contains(&(n, n + 1)) {
                        moved.push((n, n + 1));
                    }
                }
            }
        }
        moved.sort_unstable();
        if !moved.is_empty() {
            if let Some(bf) = bf {
                bf.shift_parents(def.number);
            }
        }
        self.insert_channel(row_index, def);
        Renumbered { moved }
    }

    fn channel_cells(&self, def: &ChannelDef) -> Vec<String> {
        let mut row = vec![String::new(); self.grid.new_row_width()];
        let mut put = |column: ChColumn, value: String| {
            if let Some(i) = self.map.position(column) {
                if row.len() <= i {
                    row.resize(i + 1, String::new());
                }
                row[i] = value;
            }
        };
        put(ChColumn::Number, def.number.to_string());
        put(ChColumn::Bytes, def.byte_count.to_string());
        put(ChColumn::Name, def.name.clone());
        put(ChColumn::Type, def.data_type.as_str().into());
        put(ChColumn::Lsb, shortest(def.lsb));
        put(ChColumn::Offset, shortest(def.offset));
        put(ChColumn::Unit, def.unit.clone());
        put(ChColumn::Section, def.section.clone());
        put(ChColumn::Memo, def.memo.clone());
        put(ChColumn::Var, def.var.clone());
        put(ChColumn::Format, def.format.as_str().into());
        put(
            ChColumn::Favorite,
            if def.favorite { "1".into() } else { "0".into() },
        );
        if let Some(v) = def.default_value {
            put(ChColumn::Default, v.to_string());
        }
        if let Some(b) = def.min {
            put(ChColumn::Min, b.to_string());
        }
        if let Some(b) = def.max {
            put(ChColumn::Max, b.to_string());
        }
        row
    }

    grid_delegates!();
}

impl Default for ChTable {
    fn default() -> Self {
        ChTable::new()
    }
}

/// A BF CSV: a [`Grid`] plus the columns its header names.
#[derive(Debug, Clone)]
pub struct BfTable {
    grid: Grid,
    map: ColumnMap<BfColumn>,
}

impl BfTable {
    /// Parse CSV text. Only a structurally broken file fails.
    pub fn parse(content: &str) -> Result<BfTable> {
        BfTable::parse_with(content, &ColumnVocabulary::new())
    }

    /// [`parse`](BfTable::parse), also accepting the header spellings this
    /// reader was taught ([`ColumnVocabulary`]).
    pub fn parse_with(content: &str, vocabulary: &ColumnVocabulary) -> Result<BfTable> {
        let mut map = None;
        let grid = Grid::split(content, |records| {
            map = records.first().and_then(|first| {
                ColumnMap::bf_from_header(
                    &first.iter().map(String::as_str).collect::<Vec<_>>(),
                    vocabulary,
                )
            });
            map.is_some()
        })?;
        Ok(BfTable {
            grid,
            map: map.unwrap_or_else(ColumnMap::bf_positional),
        })
    }

    /// [`parse`](BfTable::parse) after stripping BOMs and decoding UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<BfTable> {
        BfTable::parse_bytes_with(bytes, &ColumnVocabulary::new())
    }

    /// [`parse_with`](BfTable::parse_with) after stripping BOMs and
    /// decoding UTF-8.
    pub fn parse_bytes_with(bytes: &[u8], vocabulary: &ColumnVocabulary) -> Result<BfTable> {
        BfTable::parse_with(decode_utf8(bytes)?, vocabulary)
    }

    /// An empty table with the canonical columns under their canonical
    /// names — `with_columns(BfColumn::canonical(), &ColumnVocabulary::new())`.
    pub fn new() -> BfTable {
        BfTable::with_columns(BfColumn::canonical(), &ColumnVocabulary::new())
    }

    /// An empty table whose header names `columns`, in that order, in that
    /// vocabulary (ADR-0024). See [`ChTable::with_columns`].
    pub fn with_columns(columns: &[BfColumn], vocabulary: &ColumnVocabulary) -> BfTable {
        BfTable {
            grid: Grid::with_header(
                columns
                    .iter()
                    .map(|c| vocabulary.bf_spelling(*c).to_string())
                    .collect(),
            ),
            map: ColumnMap::in_order(columns),
        }
    }

    /// The cells, with nothing interpreted — for code that wants the grid
    /// and not the columns.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Interpret the current cells as Rows — the derived view behind
    /// [`crate::parse_bf_csv`]. Interpret again after editing.
    pub fn bitfields(&self) -> Parsed<Vec<BitFieldDef>> {
        interpret_bf(&self.map, self.grid.data())
    }

    /// The cross-file checks of `build_layout`, run where the grid rows
    /// still exist: every readable BF row is checked against `channels`,
    /// and each finding carries the grid row and the `number` / `bit`
    /// column, so an editor can point at the cell. Rows whose `number` or
    /// `bit` does not parse are left to `bitfields()`, which already
    /// reports them.
    pub fn cross_issues(&self, channels: &[ChannelDef]) -> Vec<Issue> {
        let number_col = self.map.position(BfColumn::Number);
        let bit_col = self.map.position(BfColumn::Bit);
        let read = |row: &[String], col: Option<usize>| {
            col.and_then(|c| row.get(c)).map(|s| s.trim().to_string())
        };

        let mut issues = Vec::new();
        for (row_index, row) in self.grid.data().iter().enumerate() {
            if is_blank(row) || is_comment(row) {
                continue;
            }
            let parent = read(row, number_col).and_then(|s| s.parse::<u32>().ok());
            let bit = read(row, bit_col).and_then(|s| s.parse::<u8>().ok());
            let (Some(parent), Some(bit)) = (parent, bit) else {
                continue;
            };
            match channels.iter().find(|c| c.number == parent) {
                Some(ch) if ch.data_type.is_bitfield() => {
                    if (bit as u32) >= ch.bits() {
                        issues.push(
                            Issue::new(
                                IssueCode::BfBitOutOfRange,
                                format!(
                                    "bit {bit} of channel {parent} is beyond its {}-bit width; the layout skips this row.",
                                    ch.bits()
                                ),
                            )
                            .at(row_index, bit_col)
                            .about_bit(parent, bit)
                            .found(bit.to_string())
                            .used(ch.bits().to_string()),
                        );
                    }
                }
                _ => issues.push(
                    Issue::new(
                        IssueCode::BfParentNotBitfield,
                        format!(
                            "bit {bit} of channel {parent} has no `BF` parent channel; the layout skips this row."
                        ),
                    )
                    .at(row_index, number_col)
                    .about_bit(parent, bit),
                ),
            }
        }
        issues
    }

    /// Rewrite every parent `number` ≥ `from` up by one, following a
    /// channel renumbering.
    fn shift_parents(&mut self, from: u32) {
        if let Some(col) = self.map.position(BfColumn::Number) {
            for row in self.grid.data_mut() {
                if is_blank(row) || is_comment(row) {
                    continue;
                }
                let number = row.get(col).and_then(|c| c.trim().parse::<u32>().ok());
                if let Some(n) = number.filter(|n| *n >= from && *n < u32::MAX) {
                    row[col] = (n + 1).to_string();
                }
            }
        }
    }

    grid_delegates!();
}

impl Default for BfTable {
    fn default() -> Self {
        BfTable::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{ChannelDef, DataType, Value};
    use crate::issue::IssueCode;

    #[test]
    fn table_round_trips_cells_rows_and_header_spelling() {
        let text = "番号,バイト数,memo,謎の列\n1,2,note,keep\n# comment\n,,,\n2,4,,also\n";

        let table = ChTable::parse(text).unwrap();

        // The source has no byte-order mark and separates records with a
        // lone newline; both come back as they were.
        assert_eq!(table.to_csv(), text);
    }

    #[test]
    fn table_quotes_only_what_needs_quoting() {
        let table = ChTable::parse("number,name\n1,\"plain\"\n2,\"with,comma\"\n").unwrap();

        let written = table.to_csv();

        assert!(written.contains("\n1,plain\n"));
        assert!(written.contains("2,\"with,comma\"\n"));
    }

    #[test]
    fn table_escapes_inner_quotes_when_writing() {
        let mut table = ChTable::parse("number,name\n1,a\n").unwrap();
        table.set_cell(0, 1, "say \"hi\"\nplease");

        let written = table.to_csv();

        assert!(written.contains("\"say \"\"hi\"\"\nplease\""));
        let reparsed = ChTable::parse(written.trim_start_matches('\u{FEFF}')).unwrap();
        assert_eq!(reparsed.cell(0, 1), Some("say \"hi\"\nplease"));
    }

    #[test]
    fn channels_derive_from_the_current_cells() {
        let mut table = ChTable::parse("number,bytes,name\n1,2,Status\n").unwrap();
        assert_eq!(table.channels().value[0].byte_count, 2);

        table.set_cell(0, 1, "4");

        assert_eq!(table.channels().value[0].byte_count, 4);
    }

    #[test]
    fn set_cell_pads_a_short_row() {
        let mut table = ChTable::parse("number,bytes,name\n1\n").unwrap();

        table.set_cell(0, 2, "Named");

        assert_eq!(table.cell(0, 1), Some(""));
        assert_eq!(table.channels().value[0].name, "Named");
    }

    #[test]
    fn rows_can_be_inserted_appended_and_removed() {
        let mut table = ChTable::parse("number,name\n1,a\n3,c\n").unwrap();

        table.insert_row(1, vec!["2".into(), "b".into()]);
        table.append_row(vec!["4".into(), "d".into()]);
        assert_eq!(table.row_count(), 4);

        let removed = table.remove_row(0);
        assert_eq!(removed, Some(vec!["1".to_string(), "a".to_string()]));
        assert_eq!(table.channels().value.len(), 3);
    }

    #[test]
    fn insert_channel_writes_only_the_columns_the_file_has() {
        let mut table = ChTable::parse("number,bytes,name\n1,2,First\n").unwrap();
        let mut def = ChannelDef::new(7, 4, DataType::UI);
        def.name = "Inserted".into();
        def.lsb = 0.5;

        table.insert_channel(1, &def);

        assert_eq!(table.cell(1, 0), Some("7"));
        assert_eq!(table.cell(1, 1), Some("4"));
        assert_eq!(table.cell(1, 2), Some("Inserted"));
        assert_eq!(table.channels().value[1].lsb, 1.0);
    }

    #[test]
    fn insert_channel_renders_type_bounds_and_default() {
        let mut table = ChTable::new();
        let mut def = ChannelDef::new(1, 2, DataType::SI);
        def.lsb = 0.1;
        def.min = Some(Value::Raw(0x10));
        def.max = Some(Value::Physical(50.0));
        def.default_value = Some(126);

        table.insert_channel(0, &def);

        let parsed = table.channels();
        assert!(parsed.issues.is_empty());
        let ch = &parsed.value[0];
        assert_eq!(ch.data_type, DataType::SI);
        assert_eq!(ch.lsb, 0.1);
        assert_eq!(ch.min, Some(Value::Raw(0x10)));
        assert_eq!(ch.max, Some(Value::Physical(50.0)));
        assert_eq!(ch.default_value, Some(126));
    }

    #[test]
    fn insert_channel_writes_the_text_columns_too() {
        let mut table = ChTable::new();
        let mut def = ChannelDef::new(1, 2, DataType::UI);
        def.section = "General".into();
        def.memo = "a, quoted memo".into();
        def.var = "g_status".into();
        def.format = crate::channel::ValueDisplay::Raw;
        def.favorite = true;

        table.insert_channel(0, &def);
        let round_tripped = ChTable::parse(table.to_csv().trim_start_matches('\u{FEFF}'))
            .unwrap()
            .channels()
            .value;

        let ch = &round_tripped[0];
        assert_eq!(ch.section, "General");
        assert_eq!(ch.memo, "a, quoted memo");
        assert_eq!(ch.var, "g_status");
        assert_eq!(ch.format, crate::channel::ValueDisplay::Raw);
        assert!(ch.favorite);
    }

    #[test]
    fn new_tables_use_the_english_canonical_header() {
        let table = ChTable::new();
        assert!(table.to_csv().contains(
            "number,bytes,bits,section,name,type,lsb,offset,unit,min,max,default,memo,var,format,favorite"
        ));
        assert_eq!(table.row_count(), 0);

        let bf = BfTable::new();
        assert!(bf.to_csv().contains("number,bit,name,default,memo"));
    }

    #[test]
    fn cross_issues_point_at_the_grid_cells() {
        let bf = BfTable::parse(
            "number,bit,name
2,3,ok
# note
9,0,orphan
2,16,beyond
1,0,not bf
",
        )
        .unwrap();
        let channels = vec![
            ChannelDef::new(1, 2, DataType::UI),
            ChannelDef::new(2, 2, DataType::BF),
        ];

        let issues = bf.cross_issues(&channels);

        assert_eq!(issues.len(), 3);
        assert_eq!(
            (issues[0].code, issues[0].row, issues[0].col),
            (IssueCode::BfParentNotBitfield, Some(2), Some(0))
        );
        assert_eq!(
            (issues[1].code, issues[1].row, issues[1].col),
            (IssueCode::BfBitOutOfRange, Some(3), Some(1))
        );
        assert_eq!(
            (issues[2].code, issues[2].row),
            (IssueCode::BfParentNotBitfield, Some(4))
        );
    }

    #[test]
    fn cross_issues_skip_rows_the_parser_already_rejected() {
        let bf = BfTable::parse(
            "number,bit,name
x,0,bad number
2,y,bad bit
",
        )
        .unwrap();
        let channels = vec![ChannelDef::new(2, 2, DataType::BF)];

        assert!(bf.cross_issues(&channels).is_empty());
    }

    #[test]
    fn insert_channel_renumbering_shifts_later_numbers_and_bf_parents() {
        let mut table =
            ChTable::parse("number,bytes,type,name\n1,1,UI8,a\n3,2,BF,flags\n# note\n4,2,UI16,c\n")
                .unwrap();
        let mut bf = BfTable::parse("number,bit,name\n3,0,b0\n4,1,x\n").unwrap();
        let mut def = ChannelDef::new(3, 2, DataType::UI);
        def.name = "inserted".into();

        let report = table.insert_channel_renumbering(1, &def, Some(&mut bf));

        assert_eq!(report.moved, vec![(3, 4), (4, 5)]);
        let numbers: Vec<u32> = table.channels().value.iter().map(|c| c.number).collect();
        assert_eq!(numbers, vec![1, 3, 4, 5]);
        let bits = bf.bitfields().value;
        assert_eq!(bits[0].parent_channel, 4);
        assert_eq!(bits[1].parent_channel, 5);
        assert_eq!(table.cell(3, 0), Some("# note"));
    }

    #[test]
    fn insert_channel_renumbering_with_a_free_number_moves_nothing() {
        let mut table = ChTable::parse("number,name\n1,a\n2,b\n").unwrap();

        let report =
            table.insert_channel_renumbering(2, &ChannelDef::new(9, 2, DataType::UI), None);

        assert!(report.moved.is_empty());
        let numbers: Vec<u32> = table.channels().value.iter().map(|c| c.number).collect();
        assert_eq!(numbers, vec![1, 2, 9]);
    }

    #[test]
    fn positional_tables_write_without_a_header() {
        let table = ChTable::parse("1,2,,Sec,Name,UI16,1,,\n").unwrap();

        let written = table.to_csv();

        assert_eq!(written, "1,2,,Sec,Name,UI16,1,,\n");
        assert_eq!(table.channels().issues[0].code, IssueCode::HeaderAssumed);
    }

    #[test]
    fn parse_bytes_accepts_a_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"number,name\n1,x\n");

        let table = ChTable::parse_bytes(&bytes).unwrap();

        assert_eq!(table.channels().value.len(), 1);
    }
}
