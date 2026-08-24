// The column vocabulary: which header spelling denotes which column.
//
// A vocabulary is data, not a language chdef knows (ADR-0024). The one it
// ships is a value like any other, so a header in any language is read by
// teaching its spellings.

using System.Text;

namespace Chdef;

/// <summary>A column of a CH CSV. The enum name lower-cased is its canonical name.</summary>
public enum ChColumn
{
    /// <summary>The <c>number</c> column.</summary>
    Number,
    /// <summary>The <c>bytes</c> column.</summary>
    Bytes,
    /// <summary>The <c>bits</c> column.</summary>
    Bits,
    /// <summary>The <c>section</c> column.</summary>
    Section,
    /// <summary>The <c>name</c> column.</summary>
    Name,
    /// <summary>The <c>type</c> column.</summary>
    Type,
    /// <summary>The <c>lsb</c> column.</summary>
    Lsb,
    /// <summary>The <c>offset</c> column.</summary>
    Offset,
    /// <summary>The <c>unit</c> column.</summary>
    Unit,
    /// <summary>The <c>min</c> column.</summary>
    Min,
    /// <summary>The <c>max</c> column.</summary>
    Max,
    /// <summary>The <c>default</c> column.</summary>
    Default,
    /// <summary>The <c>memo</c> column.</summary>
    Memo,
    /// <summary>The <c>var</c> column.</summary>
    Var,
    /// <summary>The <c>format</c> column.</summary>
    Format,
    /// <summary>The <c>favorite</c> column.</summary>
    Favorite,
}

/// <summary>A column of a BF CSV. The enum name lower-cased is its canonical name.</summary>
public enum BfColumn
{
    /// <summary>The <c>number</c> column.</summary>
    Number,
    /// <summary>The <c>bit</c> column.</summary>
    Bit,
    /// <summary>The <c>name</c> column.</summary>
    Name,
    /// <summary>The <c>default</c> column.</summary>
    Default,
    /// <summary>The <c>memo</c> column.</summary>
    Memo,
}

/// <summary>
/// The spellings one caller accepts for the columns of a CH / BF CSV, and
/// the spelling it writes for each. Dispose it once.
/// </summary>
public sealed unsafe class ColumnVocabulary : IDisposable
{
    private nint _vocabulary;

    private ColumnVocabulary(nint vocabulary) => _vocabulary = vocabulary;

    /// <summary>
    /// The empty vocabulary: the canonical column names and their variants
    /// alone.
    /// </summary>
    public static ColumnVocabulary Create()
    {
        nint handle = 0;
        Check(Native.chdef_vocabulary_new(&handle));
        return new ColumnVocabulary(handle);
    }

    /// <summary>
    /// The spellings of the definition files this format was extracted
    /// from. One value among any number a caller can build, with no
    /// standing they lack.
    /// </summary>
    public static ColumnVocabulary Japanese()
    {
        nint handle = 0;
        Check(Native.chdef_vocabulary_japanese(&handle));
        return new ColumnVocabulary(handle);
    }

    /// <summary>
    /// Teach a spelling for a CH column. The <b>first</b> spelling taught
    /// for a column is the one written for a file created with this
    /// vocabulary; every one taught is accepted when reading, trimmed and
    /// case-insensitively.
    /// </summary>
    public ColumnVocabulary Ch(string spelling, ChColumn column) =>
        Teach(Native.CHDEF_COLUMNS_CH, spelling, CanonicalName(column.ToString()));

    /// <summary>Teach a spelling for a BF column. See <see cref="Ch"/>.</summary>
    public ColumnVocabulary Bf(string spelling, BfColumn column) =>
        Teach(Native.CHDEF_COLUMNS_BF, spelling, CanonicalName(column.ToString()));

    /// <summary>The canonical names of the CH columns, in canonical order.</summary>
    public static IReadOnlyList<string> ChColumnNames() => Names(Native.CHDEF_COLUMNS_CH);

    /// <summary>The canonical names of the BF columns, in canonical order.</summary>
    public static IReadOnlyList<string> BfColumnNames() => Names(Native.CHDEF_COLUMNS_BF);

    internal nint Handle =>
        _vocabulary != 0
            ? _vocabulary
            : throw new ObjectDisposedException(nameof(ColumnVocabulary));

    /// <inheritdoc />
    public void Dispose()
    {
        if (_vocabulary != 0)
        {
            Native.chdef_vocabulary_free(_vocabulary);
            _vocabulary = 0;
        }
    }

    private ColumnVocabulary Teach(int kind, string spelling, string column)
    {
        var spellingBytes = Encoding.UTF8.GetBytes(spelling);
        var columnBytes = Encoding.UTF8.GetBytes(column);
        int status;
        fixed (byte* spellingPtr = spellingBytes)
        fixed (byte* columnPtr = columnBytes)
        {
            status = Native.chdef_vocabulary_teach(
                Handle, kind,
                spellingPtr, (nuint)spellingBytes.Length,
                columnPtr, (nuint)columnBytes.Length);
        }
        Check(status);
        return this;
    }

    private static List<string> Names(int kind)
    {
        var count = (int)Native.chdef_column_count(kind);
        var names = new List<string>(count);
        for (var i = 0; i < count; i++)
        {
            var index = (nuint)i;
            names.Add(TextBuffer.Read((buf, cap) => Native.chdef_column_name(kind, index, buf, cap)));
        }
        return names;
    }

    private static string CanonicalName(string enumName) => enumName.ToLowerInvariant();

    private static void Check(int status)
    {
        if (status != Native.CHDEF_OK)
        {
            throw new ChdefException(status, $"chdef returned status {status}");
        }
    }
}
