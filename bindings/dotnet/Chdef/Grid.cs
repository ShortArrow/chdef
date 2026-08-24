// A definition file as its cells, over the grid calls of the ABI. The
// column vocabulary is deliberately absent: cells are what the round-trip
// guarantee of docs/spec/editing.md is about, and an editor drawing a file
// needs no vocabulary to draw it (ADR-0023).

using System.Text;

namespace Chdef;

/// <summary>
/// A definition file as a header row and data rows of verbatim cells —
/// comment rows, blank rows and unknown columns included. Dispose it once.
/// </summary>
public sealed unsafe class Grid : IDisposable
{
    private nint _grid;

    private Grid(nint grid) => _grid = grid;

    /// <summary>
    /// Read definition bytes as cells. Throws <see cref="ChdefException"/>
    /// when the bytes are not decodable or the CSV is structurally broken.
    /// </summary>
    public static Grid Parse(ReadOnlySpan<byte> bytes)
    {
        var error = new byte[512];
        nint grid = 0;
        int status;

        fixed (byte* bytesPtr = bytes)
        fixed (byte* errPtr = error)
        {
            status = Native.chdef_grid_parse(
                bytesPtr, (nuint)bytes.Length, &grid, errPtr, (nuint)error.Length);
        }

        if (status != Native.CHDEF_OK)
        {
            throw new ChdefException(status, Message(error, status));
        }

        return new Grid(grid);
    }

    /// <summary><see cref="Parse(ReadOnlySpan{byte})"/> of the UTF-8 of a string.</summary>
    public static Grid Parse(string text) => Parse(Encoding.UTF8.GetBytes(text));

    /// <summary>
    /// The header cells, or an empty list for a file read without a header.
    /// </summary>
    public IReadOnlyList<string> Header
    {
        get
        {
            var count = (int)Native.chdef_grid_header_count(Handle);
            var cells = new List<string>(count);
            for (var col = 0; col < count; col++)
            {
                var index = (nuint)col;
                cells.Add(TextBuffer.Read((buf, cap) =>
                    Native.chdef_grid_header_at(Handle, index, buf, cap)));
            }
            return cells;
        }
    }

    /// <summary>
    /// How many data rows the file has, comment and blank rows included.
    /// </summary>
    public int RowCount => (int)Native.chdef_grid_row_count(Handle);

    /// <summary>How many cells the data row at <paramref name="row"/> has.</summary>
    public int ColumnCount(int row) => (int)Native.chdef_grid_col_count(Handle, (nuint)row);

    /// <summary>
    /// One data cell, 0-based with the header excluded — the row numbering
    /// <see cref="Issue.Row"/> uses. Empty outside the grid.
    /// </summary>
    public string Cell(int row, int col) =>
        TextBuffer.Read((buf, cap) =>
            Native.chdef_grid_cell(Handle, (nuint)row, (nuint)col, buf, cap));

    /// <summary>
    /// One whole data row, in order.
    /// </summary>
    public IReadOnlyList<string> Row(int row)
    {
        var count = ColumnCount(row);
        var cells = new List<string>(count);
        for (var col = 0; col < count; col++)
        {
            cells.Add(Cell(row, col));
        }
        return cells;
    }

    /// <summary>
    /// Overwrite one data cell; a row shorter than <paramref name="col"/>
    /// is padded with empty cells. A row outside the grid throws.
    /// </summary>
    public void SetCell(int row, int col, string value)
    {
        var utf8 = Encoding.UTF8.GetBytes(value);
        int status;
        fixed (byte* ptr = utf8)
        {
            status = Native.chdef_grid_set_cell(
                Handle, (nuint)row, (nuint)col, ptr, (nuint)utf8.Length);
        }
        Check(status);
    }

    /// <summary>
    /// Insert an empty data row at <paramref name="at"/>, clamped to the
    /// end. Cells are written with <see cref="SetCell"/>, which pads.
    /// </summary>
    public void InsertRow(int at) => Check(Native.chdef_grid_insert_row(Handle, (nuint)at));

    /// <summary>Append an empty data row after the last.</summary>
    public void AppendRow() => Check(Native.chdef_grid_append_row(Handle));

    /// <summary>
    /// Remove the data row at <paramref name="at"/>. A row outside the
    /// grid removes nothing and throws.
    /// </summary>
    public void RemoveRow(int at) => Check(Native.chdef_grid_remove_row(Handle, (nuint)at));

    /// <summary>
    /// The file as text, in the shape it was read in — its byte-order mark
    /// and record separator. A file already following the write rules
    /// round-trips unchanged.
    /// </summary>
    public string ToCsv() =>
        TextBuffer.Read((buf, cap) => Native.chdef_grid_to_csv(Handle, buf, cap));

    /// <summary>The bytes <see cref="ToCsv"/> writes.</summary>
    public byte[] ToCsvBytes() => Encoding.UTF8.GetBytes(ToCsv());

    /// <inheritdoc />
    public void Dispose()
    {
        if (_grid != 0)
        {
            Native.chdef_grid_free(_grid);
            _grid = 0;
        }
    }

    private nint Handle =>
        _grid != 0 ? _grid : throw new ObjectDisposedException(nameof(Grid));

    private static void Check(int status)
    {
        if (status != Native.CHDEF_OK)
        {
            throw new ChdefException(status, $"chdef returned status {status}");
        }
    }

    private static string Message(byte[] error, int status)
    {
        var end = Array.IndexOf(error, (byte)0);
        var text = Encoding.UTF8.GetString(error, 0, end < 0 ? error.Length : end);
        return text.Length > 0 ? text : $"status {status}";
    }
}

/// <summary>One text call of the ABI, waiting for a buffer.</summary>
internal unsafe delegate nuint TextCall(byte* buf, nuint cap);

/// <summary>
/// The two-call dance the ABI asks of every string: query the length, then
/// fill a buffer of it.
/// </summary>
internal static unsafe class TextBuffer
{
    internal static string Read(TextCall call)
    {
        var needed = (int)call(null, 0);
        if (needed == 0)
        {
            return string.Empty;
        }
        var buf = new byte[needed + 1];
        fixed (byte* ptr = buf)
        {
            call(ptr, (nuint)buf.Length);
        }
        return Encoding.UTF8.GetString(buf, 0, needed);
    }
}
