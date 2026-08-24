# 診断

🌐 [English](./diagnostics.md) | **日本語**

実装済み（0.0.4）: §2 のフィールドを持つ `Issue` 型、全ローダー（`build_layout` 含む）の戻り値 `Parsed { value, issues }`、および下表の全コード。横断コードは `build_layout` が、`layout_exceeds_capacity` は `ChannelLayout::check_capacity` が報告する。横断コードの行なしは ADR-0008 の決定。

## 1. 原則

- 行単位の問題で読み込みを止めない。読める行は全部読み、問題は **Issue** として返す。
- 読み込みを止めるのは、ファイルが開けない（I/O）と、CSV として構造が壊れている（閉じない引用符）ときだけ。これらはエラー。
- Issue には行と列を付ける。行は**データ行の 0 始まり**（見出しを除く。グリッドの行にそのまま対応）、列は 0 始まりの列位置。行に紐づかない Issue は行なし。
- 同じ Issue が全行で出る場合も間引かない（利用側が集約する）。

## 2. Issue

```
Issue {
    code,
    row: Option<usize>, col: Option<usize>,     // ファイル上の位置
    channel: Option<u32>, bit: Option<u8>,      // 何についての指摘か
    found: Option<String>, used: Option<String>,// 値
    message: String,                            // 同じ内容の英文
}
```

利用側が自前の文を組み立てるのに必要なものは、全てフィールドにある:

- `code` は安定な識別子（ASCII）。利用側は `code` を鍵にして翻訳・絞り込みを行う。
- `found` は chdef が使えなかった値を、ファイルの綴りのまま持つ。生値はセルの記法を保つので、`0x1FF` は `0x1FF`、`511` は `511` として返る。解釈が捨ててしまう唯一の事実。
- `used` は chdef が代わりに使った値（置き換えた場合）。
- `channel` と `bit` は、どのチャンネル、どのチャンネルのどのビットについての指摘かを言う。行を持たない指摘を名指す唯一の手段。
- 入れるものが無いフィールドは空にする。でっち上げない。

`message` は同じ事実の英語表現で、ログや英文を読みたい人のためのもの。**その文面は契約の一部ではなく、どのリリースでも変わりうる**。自前の文を組み立てる利用側はフィールドを読む。

| code | 行 | 意味 / 挙動 |
|---|---|---|
| `header_assumed` | — | 見出しが無いか `番号` 列が無く、ある列を正順とみなした（CH CSV は先頭 9 列、BF CSV は 5 列）|
| `channel_number_invalid` | あり | `番号` が整数でない / 0 以下。行を読み飛ばした |
| `channel_duplicate` | あり | 同じ `番号` が既にある。Layout では最初の行だけを使う |
| `bytes_assumed` | あり | `バイト数` が空欄 / 非整数。`型` の幅か 2 とみなした |
| `bytes_out_of_range` | あり | `バイト数` が 1〜8 の外。1〜8 に丸めた |
| `type_assumed` | あり | `型` が空欄 / 不明。`UI` とみなした |
| `type_width_mismatch` | あり | `型` の幅サフィックスと `バイト数` が違う。`バイト数` を使った |
| `lsb_invalid` | あり | `LSB` が非数 / 無限。1 とみなした |
| `offset_invalid` | あり | `オフセット` が非数。0 とみなした |
| `default_invalid` | あり | `値(デフォルト)` が整数でも `0x` 表記でもない。未指定とみなした |
| `raw_display_with_lsb` | あり | チャンネルが生値を表示する設定（`表示形式` が `HEX`）なのに `LSB` が 1 でなく、表示される数が物理量でない |
| `raw_out_of_range` | あり² | 生値（`値(デフォルト)`、`値(最小)` / `値(最大)`、`encode` に渡された値）がチャンネルの幅を超える。下位ビットだけ使った |
| `min_invalid` | あり | `値(最小)` が数値でも `0x` でもない。未指定として扱う |
| `max_invalid` | あり | `値(最大)` が数値でも `0x` でもない。未指定として扱う |
| `min_max_swapped` | あり | 解決後の `min` が `max` を上回る。両方保持し、範囲は何にも一致しない |
| `bf_parent_invalid` | あり | BF の `番号` が整数でない。行を読み飛ばした |
| `bf_bit_invalid` | あり | `BIT番号` が整数でない。行を読み飛ばした |
| `bf_bit_out_of_range` | あり / —¹ | `BIT番号` が親の幅以上。行を読み飛ばした |
| `bf_parent_not_bitfield` | —¹ | 親チャンネルが無い、または `型` が `BF` でない。行を読み飛ばした |
| `bf_default_invalid` | あり | BF の `値(デフォルト)` が `0` / `1` でない。未指定とみなした |
| `bf_duplicate` | あり | 同じ `(番号, BIT番号)` が既にある。最初の行だけを使う |
| `layout_exceeds_capacity` | — | 合計バイト数が `capacity` を超える |
| `encode_unknown_channel` | — | encode の値がレイアウトに無いチャンネルを指す。無視した |
| `encode_value_invalid` | — | encode の値が NaN / 無限大。チャンネルの既定値を使った |

¹ `build_layout` からは行なし（型付き行はファイル座標を持たない）。`channel` と `bit` のフィールドが何についての指摘かを名指す。同じ指摘を `BfTable::cross_issues` がグリッドの行・列つきで報告し、エディタはセルに落とせる。

² `encode` から出る場合は行なし。encode が受け取るのは行ではなく値だから。

## 3. エラー

```
ChdefError::Io { path, source }         // ファイルが読めない
ChdefError::CsvParse { line, message }  // 構造が壊れている（閉じない引用符）
ChdefError::Encoding { valid_up_to }    // UTF-8 として解釈できないバイト列
```

`line` は**ファイルの 1 始まりの行番号**であり、Issue が持つ 0 始まりのデータ行ではない。構造が壊れたファイルにはまだ数えるべき行が無い。`Encoding` は `parse_*_csv_bytes` の入口から出る。`load_*_csv` で読んだ UTF-8 でないファイルは、読み取り自体が拒否するので `Io` になる。

行単位の Issue と違い、エラー時は結果を返さない。利用側は「読み切れたら差し替える」（失敗したら直前の定義を残す）を実装しやすい。
