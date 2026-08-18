# 診断

🌐 [English](./diagnostics.md) | **日本語**

実装済み（0.0.1）: なし（`番号` 非整数行の読み飛ばしは行うが Issue は返さない。CSV 構造エラーは行番号つきエラー）。

## 1. 原則

- 行単位の問題で読み込みを止めない。読める行は全部読み、問題は **Issue** として返す。
- 読み込みを止めるのは、ファイルが開けない（I/O）と、CSV として構造が壊れている（閉じない引用符）ときだけ。これらはエラー。
- Issue には行と列を付ける。行は**データ行の 0 始まり**（見出しを除く。グリッドの行にそのまま対応）、列は 0 始まりの列位置。行に紐づかない Issue は行なし。
- 同じ Issue が全行で出る場合も間引かない（利用側が集約する）。

## 2. Issue

```
Issue { code, row: Option<usize>, col: Option<usize>, message: String }
```

`code` は安定な識別子（ASCII）。利用側は `code` を鍵にして翻訳・絞り込みを行う。`message` は英語 1 文で、見つけたことと chdef がどう扱ったかを述べる。

| code | 行 | 意味 / 挙動 |
|---|---|---|
| `header_assumed` | — | 見出しが無いか `番号` 列が無く、先頭 9 列を正順とみなした |
| `channel_number_invalid` | あり | `番号` が整数でない / 0 以下。行を読み飛ばした |
| `channel_duplicate` | あり | 同じ `番号` が既にある。Layout では最初の行だけを使う |
| `bytes_assumed` | あり | `バイト数` が空欄 / 非整数。`型` の幅か 2 とみなした |
| `bytes_out_of_range` | あり | `バイト数` が 1〜8 の外。1〜8 に丸めた |
| `type_assumed` | あり | `型` が空欄 / 不明。`UI` とみなした |
| `type_width_mismatch` | あり | `型` の幅サフィックスと `バイト数` が違う。`バイト数` を使った |
| `lsb_invalid` | あり | `LSB` が非数 / 無限。1 とみなした |
| `offset_invalid` | あり | `オフセット` が非数。0 とみなした |
| `default_invalid` | あり | `値(デフォルト)` が整数でも `0x` 表記でもない。未指定とみなした |
| `hex_with_lsb` | あり | `表示形式` が `HEX` なのに `LSB` が 1 でない |
| `raw_out_of_range` | あり | `0x` 指定の生値が幅を超える。下位ビットだけ使った |
| `bf_parent_invalid` | あり | BF の `番号` が整数でない。行を読み飛ばした |
| `bf_bit_invalid` | あり | `BIT番号` が整数でない。行を読み飛ばした |
| `bf_bit_out_of_range` | あり | `BIT番号` が親の幅以上。行を読み飛ばした |
| `bf_parent_not_bitfield` | あり | 親チャンネルが無い、または `型` が `BF` でない。行を読み飛ばした |
| `bf_default_invalid` | あり | BF の `値(デフォルト)` が `0` / `1` でない。未指定とみなした |
| `bf_duplicate` | あり | 同じ `(番号, BIT番号)` が既にある。最初の行だけを使う |
| `layout_exceeds_capacity` | — | 合計バイト数が `capacity` を超える |

## 3. エラー

```
Error::Io { path, source }
Error::Csv { row, message }     // 構造エラー（閉じない引用符など）
```

行単位の Issue と違い、エラー時は結果を返さない。利用側は「読み切れたら差し替える」（失敗したら直前の定義を残す）を実装しやすい。
