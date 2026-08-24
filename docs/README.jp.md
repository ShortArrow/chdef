# chdef

🌐 [English](../README.md) | **日本語**

[![CI](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml/badge.svg)](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#ライセンス)

> バイナリフレームの **CH 設定**（チャンネル定義）と **BF 設定**（ビットフィールド定義）:
> CSV の読み込み、レイアウト計算、生値 ↔ 物理値の変換。

> **⚠ Pre-alpha (0.0.x)。** API と CSV の規則はまだ動きます。
> 0.1.0 が出るまで、patch リリースがどちらも壊しうる前提でお使いください。

---

## 何をするクレートか

**CH 設定**（チャンネル定義 CSV）はバイナリフレームの各フィールドに意味を与え、
**BF 設定**（ビットフィールド定義 CSV）は `BF` 型チャンネルの各ビットに名前を与えます。
`chdef` はその Rust 実装を 1 つに集め、どの利用側も同じファイルを同じように読めるようにします。

- CH CSV / BF CSV のパース — 列は 1 つの正準名（`number,bytes,…`）を持ち、見出しがそれ以外に何と書いていようと、それは**利用側が渡す語彙**。どの言語の見出しも、その綴りを教えれば読める。先頭 BOM は無視、`number` が整数でない行は読み飛ばし
- フレームレイアウトの計算（各チャンネルの位置は `バイト数` の累積、合計バイト数）
- 生値 ↔ 物理値の相互変換 `value = raw × LSB + オフセット` とその逆（`LSB` が 0 / 空欄なら 1、`SI` は符号付き、endian 指定可、0 から遠い方へ丸めて幅に clamp）

```rust
// 語彙なし、正準名だけで読む。
let channels = chdef::parse_ch_csv(ch_csv_text)?;
let bitfields = chdef::parse_bf_csv(bf_csv_text)?;
let layout = chdef::build_layout(channels, bitfields);
let value = layout.channels[0].raw_to_value(&frame[..4]);
let bytes = layout.channels[0].value_to_bytes(value);
```

別の語彙の見出しも同じように読む。chdef が同梱するものでも、自分で組むものでも:

```rust
let japanese = chdef::ColumnVocabulary::japanese();
let german = chdef::ColumnVocabulary::new()
    .ch("Nummer", chdef::ChColumn::Number)
    .ch("Bytes", chdef::ChColumn::Bytes);

let channels = chdef::parse_ch_csv_with(ch_csv_text, &japanese)?;
```

両 CSV の列・正準名・各セルの読み方は
[docs/spec/format.jp.md](./spec/format.jp.md) に定めている。

仕様は [docs/spec/](./spec/README.jp.md)、設計判断は [docs/decisions/](./decisions/README.md)（英語）。

## C# / C から

同じ実装に C ABI 経由で届く。越境するのは仕様が定める規則の全部で、他言語の
利用側がそれを 2 度書くことはない（[docs/spec/abi.jp.md](./spec/abi.jp.md)）。

```sh
dotnet add package Chdef
```

NuGet パッケージは `linux-x64` / `win-x64` / `osx-arm64` / `osx-x64` の
ネイティブライブラリを同梱するので、他に用意するものはない。

```csharp
using var defs = Definitions.Parse(chCsv, bfCsv);
var frame = defs.Encode([Value.Parse("0x0004", 1)], out var issues);
foreach (var reading in defs.Decode(frame))
{
    foreach (var bit in reading.Bits) { /* 各ビットの名称と値 */ }
}

using var grid = Grid.Parse(File.ReadAllBytes(path));
grid.SetCell(0, 1, "4");
File.WriteAllBytes(path, grid.ToCsvBytes());
```

C から使う場合、ヘッダは
[crates/chdef-capi/include/chdef.h](../crates/chdef-capi/include/chdef.h)、
ライブラリは `chdef-capi` クレートを `cdylib` としてビルドしたもの。

## 由来

社内テレメトリブリッジ chbridge の `chbridge-core` にあった CH / BF の概念を独立させたもの。
定義ファイルそのもの（実機の CH 表）は利用側が持ち、本リポジトリには合成データしか置かない。

## ライセンス

以下のいずれかを選択:

- Apache License, Version 2.0（[LICENSE-APACHE](../LICENSE-APACHE)）
- MIT license（[LICENSE-MIT](../LICENSE-MIT)）
