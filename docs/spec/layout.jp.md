# レイアウト

🌐 [English](./layout.md) | **日本語**

実装済み（0.0.3）: 位置の累積計算と合計バイト数（`ChannelLayout::channel_offset` / `total_bytes()` / `positions()`。いずれも都度計算なので幅を編集しても陳腐化しない）、Rows / Layout の分離（`parse_*` は重複を保持し、`build_layout` が先勝ちで落とす）、§2 のレイアウト全体の `endian`、`build_layout` での BF 横断検査（Issue つきでレイアウトを返す）、§5 の `capacity`（レイアウトが持ち、`check_capacity()` が報告する）。Table 段階は `Grid` / `ChTable` / `BfTable`（[editing.jp.md](./editing.jp.md)）。フレーム全体の encode / decode は [conversion.jp.md](./conversion.jp.md) §5 / §6。

## 1. 三つの段階

chdef は定義を三段階で扱い、どの段階も取り出せる。

| 段階 | 内容 | 用途 |
|---|---|---|
| **Table** | 見出しとセルの二次元配列。原文のまま | 編集 UI、知らない列の保持、書き戻し |
| **Rows** | 各行を解釈した記録。読み飛ばした行を除く全行（重複を含む）と Issue | 連番検査など、行の集合そのものを見たい利用側 |
| **Layout** | 重複を除いたチャンネル列と各位置、BF、合計バイト数 | encode / decode、表示 |

## 2. 位置

- 位置は CSV に書かない。**Rows の並び順**（`番号` の昇順ではない）で先頭から `バイト数` を累積した値が各チャンネルの位置 `at`。`positions()` が各チャンネルを `at` つきで順に返す。
- 合計バイト数 `total_bytes` は `バイト数` の総和。これがフレームのデータ長になる（利用側はヘッダを別に足す）。
- 多バイトの生値はリトルエンディアンで並ぶのを既定とし、レイアウト単位で `endian` を Big に切り替えられる。CSV には書かない。

## 3. 重複

- 同じ `番号` が複数行にあるとき、Layout には**最初の行**だけを入れ、以降は Issue `channel_duplicate` として報告する。Rows には全て残る。
- BF で同じ `(親番号, BIT番号)` が複数行にあるときも同じ（最初が勝ち、Issue `bf_duplicate`）。

## 4. 欠番

- `番号` が飛んでいても Layout はそのまま（詰める）。欠番の補完・連番の強制は利用側の判断で、chdef は Rows を渡すだけ。

## 5. 容量

- レイアウトは `capacity`（データ部の最大バイト数）を持てる。`with_capacity` で設定し、`check_capacity()` が、収まらなければ Issue `layout_exceeds_capacity` を、収まるか容量を持たなければ何も返さない。訊かれない限り検査はしない。

## 6. 型と幅

- 幅は `バイト数`（1〜8）。`型` は幅を持たず、解釈（`UI` / `SI` / `BF`）だけを持つ。コード上で組み立てた定義の `バイト数` が 1〜8 の外にある場合は、列と同じ範囲の最も近い幅として測る（`ChannelDef::width`）。
- `SI` は幅で符号拡張する。`UI` / `BF` は符号なし。
