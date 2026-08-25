# chdef

🌐 [English](../README.md) | **日本語**

> バイナリフレームのチャンネル定義（CH）とビットフィールド定義（BF）。CSV を読み、レイアウトを計算し、生値 ↔ 物理値を相互変換する。

> **⚠ Pre-alpha (0.0.x)。** API も CSV の規則もまだ動いている。0.1.0 までは、パッチリリースがどちらを壊すこともある。

---

**CH 定義**（チャンネル定義 CSV）はバイナリフレームの各フィールドに意味を与え、**BF 定義**（ビットフィールド定義 CSV）は `BF` 型チャンネルの各ビットに名前を与える。このリポジトリは両者の実装 1 つと、そこへ至る全ての経路を持つ。どの利用側も同じファイルを同じように読むために。

## 何が置いてあるか

| | | |
|---|---|---|
| [`chdef`](../crates/chdef/README.md) | ライブラリ | `cargo add chdef` |
| [`chdef-capi`](../crates/chdef-capi/include/chdef.h) | その上の C ABI | `cargo add chdef-capi` |
| [`Chdef`](../bindings/dotnet/Chdef/README.md) | .NET バインディング（ネイティブ同梱） | `dotnet add package Chdef` |

それぞれが、呼び出す言語に向けた README を持つ。C ABI は仕様が定める規則を全て運ぶので、他言語の利用側がそれを 2 度書くことはない。

## ドキュメント

| | |
|---|---|
| [docs/guide.jp.md](./guide.jp.md) | 各作業への最短経路 |
| [docs/spec/](./spec/README.jp.md) | 形式が何であるか、正確に |
| [docs/migration.jp.md](./migration.jp.md) | 0.0.x の各版で何が変わったか |
| [docs/decisions/](./decisions/README.md) | なぜそうなっているか（英語） |

Rust の README にある例は全て `cargo test` がコンパイルして実行する。.NET の README にある例は全て `Chdef.Tests` のテストで、README との一致をワークスペースが検査する。**腐りようのないページ**にすることが狙い。

## 由来

CH / BF の概念は、社内テレメトリブリッジ chbridge の `chbridge-core` から抽出したもの。定義ファイルそのもの（実機のチャンネル表）は各利用側に属し、このリポジトリには合成データしか置かない。

## ライセンス

以下のいずれかを選択:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))
