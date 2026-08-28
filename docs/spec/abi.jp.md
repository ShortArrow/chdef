# ABI

🌐 [English](./abi.md) | **日本語**

実装済み（0.0.16）: 下記の全部 — レイアウト、変換、名前付きビット、Grid、値の記法、Issue を、`chdef-capi` の C ABI と、その上に立つ .NET バインディング経由で。TypeScript はこの ABI 経由では届かず、クレートの上に立つ WebAssembly バインディングが同じ規則を運ぶ。その名前は §3 に他と並べて挙げる。

## 1. ABI が何を運ぶか

**ABI 経由で chdef を使う利用側は、この仕様が定める規則を自分で書き直さなくてよい。** これが唯一の基準で、両方向に線を引き、ABI が次に何を増やすかを決める。

運ぶもの。利用側が書けば、住所が 1 つしかないはずの規則の 2 つ目の実装を抱えることになるから:

| 規則 | 文書 | 担当する呼び出し |
|---|---|---|
| 位置・幅・合計・容量 | [layout.jp.md](./layout.jp.md) | レイアウト |
| 生値 ↔ 物理値、符号、クランプ、バイト順 | [conversion.jp.md](./conversion.jp.md) §1〜§3 | encode / decode |
| BF 既定値の合成、読みの名前付きビット | [conversion.jp.md](./conversion.jp.md) §4・§6 | ビット |
| 読みがどちらを表示するか、その文字列 | [conversion.jp.md](./conversion.jp.md) §7 | 読み |
| `0x` なら生値、それ以外は物理値 | [format.jp.md](./format.jp.md) §3 | 値の記法 |
| どの見出し綴りがどの列を指すか | [format.jp.md](./format.jp.md) §2 | 語彙 |
| ファイルのセルと、読んだ形のまま書き戻すこと | [editing.jp.md](./editing.jp.md) | Grid |
| コード・メッセージ・指す行と列 | [diagnostics.jp.md](./diagnostics.jp.md) | Issue |

運ばないもの。chdef が何をしようと利用側が書くものだから: 編集 UI、undo 履歴、保存のオーケストレーション、および仕様索引の対象外リストにある表示の選択。これらを運ぶ ABI は、アプリケーションの代わりに決めている。

Grid が公開されるのは**セルとして**であって、型付きの編集 API としてではない。[editing.jp.md §2](./editing.jp.md) の往復の保証が語っているのはセルであり、定義ファイルを表示する利用側は列の語彙を持たずにそれができる。

## 2. 呼び出し規約

- **ステータスは `int32_t`。** `CHDEF_OK` が `0`、失敗は全て負、panic は `extern "C"` を越えさせずに `CHDEF_PANIC` を返す。長さを返す呼び出しは、失敗する場面で `0` を返す。
- **列挙は書き出さない。** 種別 — データ型、Issue コード、表示形式 — は ASCII 文字列として渡る。増やしても ABI の破壊にならず、知らない値を受けた利用側にも表示するものが残る。
- **ハンドルは不透明でタグ付き。** 生成時にタグを打つので、null や、別種のハンドルを渡された場合は、誤った型として参照されずに `CHDEF_ERR_HANDLE` になる。null は対応する `_free` が無視する。
- **解放済みハンドルの使用は未定義**。C のポインタ一般と同じで、メモリはアロケータのものに戻り、タグがそこに残る保証は無い。解放はメモリを返す前にタグを消すので古いハンドルは**しばしば**捕まるが、それはバグに向かう途中の親切であって保証ではなく、契約もしていない。二重解放も同じ。
- **文字列は利用側のバッファに書き、利用側のために確保しない。** 全ての文字列呼び出しは UTF-8 を書き、必ず終端し、値に必要な長さを返す。利用側は容量 `0` で長さを聞き、確保して、もう一度聞く。ライブラリが解放義務つきのポインタを渡すことはない。
- **配列も同じ**: 個数を先に書き、足りないバッファは `CHDEF_ERR_BUFFER` で、何も書かない。
- **添字は全て 0 始まり**で、範囲外は `CHDEF_ERR_INDEX`。panic せず、途中まで書くこともない。

宣言の正本は C ヘッダ（`crates/chdef-capi/include/chdef.h`）で、宣言の無いシンボルがあればこのリポジトリのテストが落ちる。

## 3. 面

運ぶものの名前をとって 6 群:

- **レイアウト** — CH / BF の CSV をレイアウトに読み、合計とチャンネルを問い、バイト順と容量を設定し、容量を検査する。
- **変換** — 値をフレームに encode、フレームを読みに decode、値 1 つをどちら向きにも変換、幅が値を保持できるかとチャンネルが宣言する範囲、1 つの読みの表示値と表示文字列。
- **ビット** — チャンネルの名前付きビット（番号・名称・備考・プロトコル仕様の既定値またはその不在）と、デコード済みフレームの各ビットの値。
- **Grid** — 定義のバイト列を、列を名付ける語彙とともにセルに読み、見出しと任意のセルを読み、セルを書き、行を挿入・追加・削除し、ファイルを書き戻し、いまのセルの何が悪いかを問う。
- **値の記法** — 値の文字列形を、それが表す値に読む。
- **語彙** — 正準の列名と、それを使って組む語彙。parse はこれで見出しを読む。列は番号ではなく正準**名**で指されるので、形式に列を足しても ABI の破壊にならない。
- **Issue** — 各指摘の個数・数値・文字列。

フレームのビットは 1 回の走査でデコードされ、ビットごとの呼び出しにはならない。個数はレイアウトが答え、1 回の呼び出しが配列を埋めるので、全ビットを読む費用は全チャンネルを読む費用と同じ。

Grid はレイアウトと同じ規則で読む。先頭レコードが与えられた語彙で `number` を名乗るときだけ見出しで、名乗らないファイルは見出し無しの位置指定として読む。だから指摘の行番号は、どの呼び出しが出したものでも同じセルを指す。

### 同じ面を、名前で

1 行が 1 つの操作で、各バインディングでの名前を並べる。3 つのどこから
chdef に触れても上の規則は全部あり、その言語の慣習に沿った名前で見つかる。
リポジトリのテストがこの表を読み、ここに書かれた名前をバインディングが
持たなければ落ちる。

| 操作 | C | .NET | JavaScript |
|---|---|---|---|
| 定義をレイアウトに読む | `chdef_layout_parse_with` | `Definitions.Parse` | `Definitions.parse` |
| 読んだときに見つかったもの | `chdef_issue_at` | `Definitions.Issues` | `Definitions.issues` |
| フレームのデータ長 | `chdef_layout_total_bytes` | `Definitions.TotalBytes` | `Definitions.totalBytes` |
| チャンネル、フレーム順 | `chdef_layout_channel_at` | `Definitions.Channels` | `Definitions.channels` |
| バイト順 | `chdef_layout_set_endian` | `Definitions.Endian` | `Definitions.endian` |
| バイト容量 | `chdef_layout_set_capacity` | `Definitions.Capacity` | `Definitions.capacity` |
| チャンネル容量 | `chdef_layout_set_channel_capacity` | `Definitions.ChannelCapacity` | `Definitions.channelCapacity` |
| フレームが容量に収まるか | `chdef_layout_limits_exceeded` | `Definitions.LimitsExceeded` | `Definitions.limitsExceeded` |
| 値をフレームへ | `chdef_encode` | `Definitions.Encode` | `Definitions.encode` |
| フレームを読みへ | `chdef_decode` | `Definitions.Decode` | `Definitions.decode` |
| 派生チャンネルを埋める | `chdef_seal` | `Definitions.Seal` | `Definitions.seal` |
| どの派生チャンネルが食い違うか | `chdef_derived_mismatches` | `Definitions.DerivedMismatches` | `Definitions.derivedMismatches` |
| 派生チャンネルが覆うバイト | `chdef_covered_bytes` | `Definitions.CoveredBytes` | `Definitions.coveredBytes` |
| 名前で知っているレシピ | `chdef_recipe_name` | `Definitions.Recipes` | `recipes` |
| 物理値 1 つを生ビット列へ | `chdef_layout_channel_to_raw` | `Definitions.ToRaw` | `Definitions.toRaw` |
| 生ビット列 1 つを物理値へ | `chdef_layout_channel_to_value` | `Definitions.ToValue` | `Definitions.toValue` |
| 幅が値を保持できるか | `chdef_layout_channel_fits_width` | `Definitions.FitsWidth` | `Definitions.fitsWidth` |
| 宣言された範囲、物理値で | `chdef_layout_channel_range` | `Definitions.RangeOf` | `Definitions.rangeOf` |
| どの値が範囲を外れるか | `chdef_values_out_of_range` | `Definitions.ValuesOutOfRange` | `Definitions.valuesOutOfRange` |
| どの読みが範囲を外れるか | `chdef_readings_out_of_range` | `Definitions.ReadingsOutOfRange` | `Definitions.readingsOutOfRange` |
| format 列がどちらの読みを見せるか | `chdef_layout_channel_displayed` | `Definitions.Displayed` | `Definitions.displayed` |
| 読みの既定の文字列 | `chdef_layout_channel_render` | `Definitions.Render` | `Definitions.render` |
| チャンネルの名前付きビット | `chdef_layout_bit_at` | `Channel.Bits` | `Channel.bits` |
| 読みのビット | `chdef_decode_bits` | `Reading.Bits` | `Reading.bits` |
| 値の文字列形 | `chdef_value_parse` | `Value.Parse` | `parseValue` |
| ファイルをセルとして読む | `chdef_grid_parse_with` | `Grid.Parse` | `Table.parse` |
| 見出しのセル | `chdef_grid_header_at` | `Grid.Header` | `Table.header` |
| データ行の数 | `chdef_grid_row_count` | `Grid.RowCount` | `Table.rowCount` |
| 行のセル数 | `chdef_grid_col_count` | `Grid.ColumnCount` | `Table.columnCount` |
| セル 1 つ | `chdef_grid_cell` | `Grid.Cell` | `Table.cell` |
| セル 1 つを上書き | `chdef_grid_set_cell` | `Grid.SetCell` | `Table.setCell` |
| 行を挿入 | `chdef_grid_insert_row` | `Grid.InsertRow` | `Table.insertRow` |
| 行を追加 | `chdef_grid_append_row` | `Grid.AppendRow` | `Table.appendRow` |
| 行を削除 | `chdef_grid_remove_row` | `Grid.RemoveRow` | `Table.removeRow` |
| ファイルを書き戻す | `chdef_grid_to_csv` | `Grid.ToCsv` | `Table.toCsv` |
| セルの何が悪いか | `chdef_grid_issues` | `Grid.Issues` | `Table.issues` |
| どの default が自分の範囲を外れるか | `chdef_grid_defaults_out_of_range` | `Grid.DefaultsOutOfRange` | `Table.defaultsOutOfRange` |
| 空の語彙 | `chdef_vocabulary_new` | `ColumnVocabulary.Create` | `new ColumnVocabulary()` |
| 日本語の語彙 | `chdef_vocabulary_japanese` | `ColumnVocabulary.Japanese` | `ColumnVocabulary.japanese` |
| CH の綴りを教える | `chdef_vocabulary_teach` | `ColumnVocabulary.Ch` | `ColumnVocabulary.ch` |
| BF の綴りを教える | `chdef_vocabulary_teach` | `ColumnVocabulary.Bf` | `ColumnVocabulary.bf` |
| CH の正準列名 | `chdef_column_name` | `ColumnVocabulary.ChColumnNames` | `ColumnVocabulary.chColumns` |
| BF の正準列名 | `chdef_column_name` | `ColumnVocabulary.BfColumnNames` | `ColumnVocabulary.bfColumns` |
| ビルドが報告しうる全 Issue コード | `chdef_issue_code_name` | `IssueCode.All` | `issueCodes` |

## 4. 版

`chdef_abi_version()` は `CHDEF_ABI_VERSION` を返し、この値は**シンボルの追加・変更のたびに上がる**。利用側は、自分の宣言が書かれた時点の値**以上**であることを確認する。シンボルは増えるだけで取り下げられないので、新しいライブラリは古い利用側に使える。この検査が捕まえるのは逆 — 存在しないシンボルを要求している利用側。

.NET パッケージは対応する全ランタイムのネイティブライブラリを同梱するので、その経路をとる利用側が両者を取り違えることはない。
