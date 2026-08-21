//! The Table stage (`docs/spec/layout.md` §1): header and cells as a
//! two-dimensional array, verbatim. The substrate for editing and for
//! writing back — unknown columns, comment rows and the header spelling all
//! survive a read → edit → write round trip at cell granularity (quoting
//! and record separators are normalised to `docs/spec/format.md` §1).
//! Rows and Layout are derived views: interpret again after editing.

use crate::channel::{BitFieldDef, ChannelDef, Value};
use crate::columns::{BfColumn, ChColumn, ColumnMap};
use crate::csv::{decode_utf8, interpret_bf, interpret_ch, is_blank, is_comment};
use crate::error::{ChdefError, Result};
use crate::issue::{Issue, IssueCode, Parsed};

/// What [`ChTable::insert_channel_renumbering`] moved: `(old, new)` per
/// renumbered channel, ascending by old number. The consumer repairs or
/// announces its own references with it — chdef does not know them.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renumbered {
    pub moved: Vec<(u32, u32)>,
}

/// The cell grid shared by [`ChTable`] and [`BfTable`].
#[derive(Debug, Clone)]
struct Cells<C> {
    /// The header row as read (or as created); `None` for a positional file.
    header: Option<Vec<String>>,
    map: ColumnMap<C>,
    rows: Vec<Vec<String>>,
}

impl<C: Copy + PartialEq> Cells<C> {
    fn parse(
        content: &str,
        from_header: fn(&[&str]) -> Option<ColumnMap<C>>,
        positional: fn() -> ColumnMap<C>,
    ) -> Result<Self> {
        let content = content.trim_start_matches('\u{FEFF}');
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content.as_bytes());

        let mut rows: Vec<Vec<String>> = Vec::new();
        for (index, record) in rdr.records().enumerate() {
            let record = record.map_err(|e| ChdefError::CsvParse {
                row: index + 1,
                message: e.to_string(),
            })?;
            rows.push(record.iter().map(str::to_string).collect());
        }

        let map = rows
            .first()
            .and_then(|first| from_header(&first.iter().map(String::as_str).collect::<Vec<_>>()));
        Ok(match map {
            Some(map) => Cells {
                header: Some(rows.remove(0)),
                map,
                rows,
            },
            None => Cells {
                header: None,
                map: positional(),
                rows,
            },
        })
    }

    fn to_csv(&self) -> String {
        let mut out = String::from("\u{FEFF}");
        if let Some(header) = &self.header {
            push_row(&mut out, header);
        }
        for row in &self.rows {
            push_row(&mut out, row);
        }
        out
    }

    fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.rows.get(row)?.get(col).map(String::as_str)
    }

    fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if let Some(cells) = self.rows.get_mut(row) {
            if cells.len() <= col {
                cells.resize(col + 1, String::new());
            }
            cells[col] = value;
        }
    }

    /// Width for a generated row: the header width, or the widest position
    /// the column map knows.
    fn new_row_width(&self) -> usize {
        self.header.as_ref().map(Vec::len).unwrap_or_default()
    }
}

fn push_row(out: &mut String, row: &[String]) {
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
    out.push_str("\r\n");
}

fn shortest(v: f64) -> String {
    format!("{v}")
}

fn value_cell(value: Value) -> String {
    match value {
        Value::Physical(v) => shortest(v),
        Value::Raw(r) => format!("0x{r:X}"),
    }
}

macro_rules! grid_api {
    () => {
        /// Serialise the table back to CSV text: UTF-8 BOM first, `\r\n`
        /// separators, and a cell quoted only when it holds `,` `"` or a
        /// newline (`docs/spec/format.md` §1). Cell contents and row
        /// structure round-trip; the original quoting and separators are
        /// normalised to those rules.
        pub fn to_csv(&self) -> String {
            self.cells.to_csv()
        }

        /// The cell at `(row, col)` of the data grid (header excluded),
        /// exactly as it will be written. `None` outside the grid.
        pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
            self.cells.cell(row, col)
        }

        /// Overwrite one cell; a row shorter than `col` is padded with
        /// empty cells. Out-of-grid rows are ignored.
        pub fn set_cell(&mut self, row: usize, col: usize, value: impl Into<String>) {
            self.cells.set_cell(row, col, value.into());
        }

        /// Number of data rows (comment and blank rows included).
        pub fn row_count(&self) -> usize {
            self.cells.rows.len()
        }

        /// Insert a raw row of cells at `index` (clamped to the end).
        pub fn insert_row(&mut self, index: usize, cells: Vec<String>) {
            let index = index.min(self.cells.rows.len());
            self.cells.rows.insert(index, cells);
        }

        /// Append a raw row of cells.
        pub fn append_row(&mut self, cells: Vec<String>) {
            self.cells.rows.push(cells);
        }

        /// Remove and return the row at `index`.
        pub fn remove_row(&mut self, index: usize) -> Vec<String> {
            self.cells.rows.remove(index)
        }
    };
}

/// A CH CSV as its cell grid. See the module docs.
#[derive(Debug, Clone)]
pub struct ChTable {
    cells: Cells<ChColumn>,
}

impl ChTable {
    /// Parse CSV text into the grid. Only a structurally broken file fails;
    /// everything interpretable-or-not is kept as cells.
    pub fn parse(content: &str) -> Result<ChTable> {
        Ok(ChTable {
            cells: Cells::parse(content, ColumnMap::ch_from_header, ColumnMap::ch_positional)?,
        })
    }

    /// [`parse`](ChTable::parse) after stripping BOMs and decoding UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<ChTable> {
        ChTable::parse(decode_utf8(bytes)?)
    }

    /// An empty table with the English canonical 16-column header.
    pub fn new() -> ChTable {
        let header: Vec<String> = ChColumn::CANONICAL.iter().map(|c| c.en().into()).collect();
        let cells: Vec<&str> = header.iter().map(String::as_str).collect();
        ChTable {
            cells: Cells {
                map: ColumnMap::ch_from_header(&cells).expect("canonical header names `number`"),
                header: Some(header),
                rows: Vec::new(),
            },
        }
    }

    /// Interpret the current cells as Rows (`docs/spec/format.md` §3) —
    /// the derived view behind [`crate::parse_ch_csv`]. Interpret again
    /// after editing; nothing is cached.
    pub fn channels(&self) -> Parsed<Vec<ChannelDef>> {
        interpret_ch(&self.cells.map, &self.cells.rows)
    }

    /// Insert `def` as a new data row at `row_index`, rendering the columns
    /// this file has (a field without a column is dropped): `number` /
    /// `bytes` in decimal, `type` as its category (`UI` / `SI` / `BF` — the
    /// width stays in `bytes`), `min` / `max` in their own notation, and
    /// `default` in decimal.
    pub fn insert_channel(&mut self, row_index: usize, def: &ChannelDef) {
        let row = self.channel_cells(def);
        self.insert_row(row_index, row);
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
        if let Some(col) = self.cells.map.position(ChColumn::Number) {
            for row in &mut self.cells.rows {
                if is_blank(row) || is_comment(row) {
                    continue;
                }
                let number = row.get(col).and_then(|c| c.trim().parse::<u32>().ok());
                if let Some(n) = number.filter(|n| *n >= def.number) {
                    row[col] = (n + 1).to_string();
                    moved.push((n, n + 1));
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
        let mut row = vec![String::new(); self.cells.new_row_width()];
        let mut put = |column: ChColumn, value: String| {
            if let Some(i) = self.cells.map.position(column) {
                if row.len() <= i {
                    row.resize(i + 1, String::new());
                }
                row[i] = value;
            }
        };
        put(ChColumn::Number, def.number.to_string());
        put(ChColumn::Bytes, def.byte_count.to_string());
        put(ChColumn::Name, def.name.clone());
        put(ChColumn::Type, def.data_type.category().into());
        put(ChColumn::Lsb, shortest(def.lsb));
        put(ChColumn::Offset, shortest(def.offset));
        put(ChColumn::Unit, def.unit.clone());
        if let Some(v) = def.default_value {
            put(ChColumn::Default, v.to_string());
        }
        if let Some(b) = def.min {
            put(ChColumn::Min, value_cell(b));
        }
        if let Some(b) = def.max {
            put(ChColumn::Max, value_cell(b));
        }
        row
    }

    grid_api!();
}

impl Default for ChTable {
    fn default() -> Self {
        ChTable::new()
    }
}

/// A BF CSV as its cell grid. See the module docs.
#[derive(Debug, Clone)]
pub struct BfTable {
    cells: Cells<BfColumn>,
}

impl BfTable {
    /// Parse CSV text into the grid. Only a structurally broken file fails.
    pub fn parse(content: &str) -> Result<BfTable> {
        Ok(BfTable {
            cells: Cells::parse(content, ColumnMap::bf_from_header, ColumnMap::bf_positional)?,
        })
    }

    /// [`parse`](BfTable::parse) after stripping BOMs and decoding UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<BfTable> {
        BfTable::parse(decode_utf8(bytes)?)
    }

    /// An empty table with the English canonical 5-column header.
    pub fn new() -> BfTable {
        let header: Vec<String> = BfColumn::CANONICAL.iter().map(|c| c.en().into()).collect();
        let cells: Vec<&str> = header.iter().map(String::as_str).collect();
        BfTable {
            cells: Cells {
                map: ColumnMap::bf_from_header(&cells).expect("canonical header names `number`"),
                header: Some(header),
                rows: Vec::new(),
            },
        }
    }

    /// Interpret the current cells as Rows — the derived view behind
    /// [`crate::parse_bf_csv`]. Interpret again after editing.
    pub fn bitfields(&self) -> Parsed<Vec<BitFieldDef>> {
        interpret_bf(&self.cells.map, &self.cells.rows)
    }

    /// The cross-file checks of `build_layout`, run where the grid rows
    /// still exist: every readable BF row is checked against `channels`,
    /// and each finding carries the grid row and the `number` / `bit`
    /// column, so an editor can point at the cell. Rows whose `number` or
    /// `bit` does not parse are left to `bitfields()`, which already
    /// reports them.
    pub fn cross_issues(&self, channels: &[ChannelDef]) -> Vec<Issue> {
        let number_col = self.cells.map.position(BfColumn::Number);
        let bit_col = self.cells.map.position(BfColumn::Bit);
        let read = |row: &[String], col: Option<usize>| {
            col.and_then(|c| row.get(c)).map(|s| s.trim().to_string())
        };

        let mut issues = Vec::new();
        for (row_index, row) in self.cells.rows.iter().enumerate() {
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
                        issues.push(Issue {
                            code: IssueCode::BfBitOutOfRange,
                            row: Some(row_index),
                            col: bit_col,
                            message: format!(
                                "bit {bit} of channel {parent} is beyond its {}-bit width; the layout skips this row.",
                                ch.bits()
                            ),
                        });
                    }
                }
                _ => issues.push(Issue {
                    code: IssueCode::BfParentNotBitfield,
                    row: Some(row_index),
                    col: number_col,
                    message: format!(
                        "bit {bit} of channel {parent} has no `BF` parent channel; the layout skips this row."
                    ),
                }),
            }
        }
        issues
    }

    /// Rewrite every parent `number` ≥ `from` up by one, following a
    /// channel renumbering.
    fn shift_parents(&mut self, from: u32) {
        if let Some(col) = self.cells.map.position(BfColumn::Number) {
            for row in &mut self.cells.rows {
                if is_blank(row) || is_comment(row) {
                    continue;
                }
                let number = row.get(col).and_then(|c| c.trim().parse::<u32>().ok());
                if let Some(n) = number.filter(|n| *n >= from) {
                    row[col] = (n + 1).to_string();
                }
            }
        }
    }

    grid_api!();
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
        let written = table.to_csv();

        assert!(written.starts_with('\u{FEFF}'));
        assert_eq!(
            written.trim_start_matches('\u{FEFF}'),
            "番号,バイト数,memo,謎の列\r\n1,2,note,keep\r\n# comment\r\n,,,\r\n2,4,,also\r\n"
        );
    }

    #[test]
    fn table_quotes_only_what_needs_quoting() {
        let table = ChTable::parse("number,name\n1,\"plain\"\n2,\"with,comma\"\n").unwrap();

        let written = table.to_csv();

        assert!(written.contains("\r\n1,plain\r\n"));
        assert!(written.contains("2,\"with,comma\"\r\n"));
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
        assert_eq!(removed, vec!["1".to_string(), "a".to_string()]);
        assert_eq!(table.channels().value.len(), 3);
    }

    #[test]
    fn insert_channel_writes_only_the_columns_the_file_has() {
        let mut table = ChTable::parse("number,bytes,name\n1,2,First\n").unwrap();
        let mut def = ChannelDef::new(7, 4, DataType::UI32);
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
        let mut def = ChannelDef::new(1, 2, DataType::SI16);
        def.lsb = 0.1;
        def.min = Some(Value::Raw(0x10));
        def.max = Some(Value::Physical(50.0));
        def.default_value = Some(126);

        table.insert_channel(0, &def);

        let parsed = table.channels();
        assert!(parsed.issues.is_empty());
        let ch = &parsed.value[0];
        assert_eq!(ch.data_type, DataType::SI16);
        assert_eq!(ch.lsb, 0.1);
        assert_eq!(ch.min, Some(Value::Raw(0x10)));
        assert_eq!(ch.max, Some(Value::Physical(50.0)));
        assert_eq!(ch.default_value, Some(126));
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
            ChannelDef::new(1, 2, DataType::UI16),
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
        let mut def = ChannelDef::new(3, 2, DataType::UI16);
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
            table.insert_channel_renumbering(2, &ChannelDef::new(9, 2, DataType::UI16), None);

        assert!(report.moved.is_empty());
        let numbers: Vec<u32> = table.channels().value.iter().map(|c| c.number).collect();
        assert_eq!(numbers, vec![1, 2, 9]);
    }

    #[test]
    fn positional_tables_write_without_a_header() {
        let table = ChTable::parse("1,2,,Sec,Name,UI16,1,,\n").unwrap();

        let written = table.to_csv();

        assert_eq!(
            written.trim_start_matches('\u{FEFF}'),
            "1,2,,Sec,Name,UI16,1,,\r\n"
        );
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
