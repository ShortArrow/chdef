# 移行

🌐 [English](./migration.md) | **日本語**

0.0.x の各版で利用側が直す必要のあるものを 1 枚にまとめたもの。[CHANGELOG](../CHANGELOG.md) は各リリースを個別に記録する。このページは、複数版を一気に飛び越える人向け。

0.0.x は各版を MAJOR 相当として扱うので破壊的変更を許す。全て列挙してある。

## 一覧

| 元 → 先 | 直すもの |
|---|---|
| 0.0.5 → 0.0.6 | 無し（0.0.6 はどのレジストリにも届いていない） |
| 0.0.6 → 0.0.7 | 日本語見出しを読むには**語彙を渡す** |
| 0.0.7 → 0.0.8 | `check_capacity()` がリストを返す。`Channel.BitCount` は `Channel.Bits` |
| 0.0.8 → 0.0.9 | 無し。ただしクランプする通信に新しい Issue が出る |
| 0.0.9 → 0.0.10 | `check_capacity` は `limits_exceeded` |
| 0.0.10 → 0.0.11 | `Channel` に `Derived` が増えた（位置分解） |
| 0.0.11 → 0.0.12 | 無し |
| 0.0.12 → 0.0.13 | 無し |
| 0.0.13 → 0.0.14 | 無し |
| 0.0.14 → 0.0.15 | 無し。npm に JavaScript バインディングが加わった |
| 0.0.15 → 0.0.16 | **`chdef_grid_parse` / `Grid.Parse` は見出しがあるときだけ見出しとして読む**。クレート利用側は無し |

この間に `CHDEF_ABI_VERSION` は 2 → 3 → 4 → 5 → 6 → 7。.NET パッケージは自分のネイティブライブラリを同梱するので、この対応付けが利用側の管理対象になることはない。

## 0.0.7 — 語彙はデータ

**日本語の見出しは既定では読まれなくなった。** 列は正準名を 1 つ持ち、それ以外の綴りは利用側が渡す語彙。

```rust
// 変更前
let channels = chdef::parse_ch_csv(text)?;
let table = chdef::ChTable::parse(text)?;

// 変更後 — 日本語見出しのファイル
let japanese = chdef::ColumnVocabulary::japanese();
let channels = chdef::parse_ch_csv_with(text, &japanese)?;
let table = chdef::ChTable::parse_with(text, &japanese)?;
```

```csharp
// 変更前
using var defs = Definitions.Parse(ch, bf);

// 変更後
using var japanese = ColumnVocabulary.Japanese();
using var defs = Definitions.Parse(ch, bf, japanese);
```

見出しが正準名のファイルは変更不要。認識されない見出しのファイルは列を位置で読み、`header_assumed` を報告する — **つまり、この修正を忘れた症状は `issues` に出る `header_assumed` であって、沈黙ではない**。

併せて削除: `ColumnAliases` と `HeaderLanguage`（`ColumnVocabulary` に統合）。`ChColumn::name(lang)` は `ChColumn::name()`、`with_columns(columns, language)` は `with_columns(columns, &vocabulary)`。

## 0.0.8 — 上限が 2 つに、コードが列挙可能に

`ChannelLayout::check_capacity()` の戻り値が `Option<Issue>` から `Vec<Issue>` へ。レイアウトはバイト数上限とチャンネル数上限の両方を超えうるので、1 つずつ知ると 1 つずつ直す羽目になる。ABI と .NET バインディングは無変更（元からリスト）。

```rust
// 変更前
if let Some(issue) = layout.check_capacity() { … }

// 変更後
for issue in layout.check_capacity() { … }
```

.NET バインディングでは `Channel.BitCount` が消え、ビットそのものが `Channel.Bits`、個数は `Bits.Count`。`Reading` に `Bits` が増えたので、そのレコードの位置分解は 4 要素になる。

正準の CH 見出しは 17 列（`kind` を末尾に追加）。9 列・10 列の位置形式は今までどおり読める — **以降に足した列を全て末尾に置いているのはこのため**。

## 0.0.9 — クランプが黙らなくなる

API は変わらない。**範囲外の物理値を送り続けてきた通信に、新しい Issue が出る**: `encode_value_clamped`。チャンネル幅に収まらない物理値が、別の数として線に乗るとき。

バイト列は変わらない。変わるのは `issues` なので、個数や空であることを検査するテストが気づく。それが指しているのは、**頼んだ数と送られた数が黙って違っていた場所**。

`IssueCode.All`（0.0.8）が、コードを鍵にした表を追従させる手段。

## 0.0.10 — 観測は状態の名前で呼ぶ

```
check_capacity()            →  limits_exceeded()
chdef_layout_check_capacity →  chdef_layout_limits_exceeded
CheckCapacity()             →  LimitsExceeded()
```

1 呼び出し箇所につき 1 行。他は何も変わらない。0.0.8 以降このメソッドは 2 つの上限に答えていたのに、名前が 1 つしか言っていなかった。

## 0.0.11 — 算出チャンネル

1 点を除いて追加のみ。.NET バインディングの `Channel` に `Derived` が増えたので位置分解が 1 要素増える（名前指定なら影響なし）。

新しく使えるもの: `算出` 列、`種別 = derived`、`seal` / `derived_mismatches` / `covered_bytes`。算出チャンネルの無い定義は今までと完全に同じ挙動になる — `encode` は無変更。

## 0.0.12 — ドキュメント

コードの変更なし。README の位置が変わった: `crates/chdef/README.md` が crates.io と docs.rs の表紙、`bindings/dotnet/Chdef/README.md` が NuGet の表紙、リポジトリの README は両方を指す。

.NET バインディングの `BitReading` に `Name` が増え、デコードされたビットが名前を持つようになった（Rust 側は元からそうだった）。このレコードの位置分解は 4 要素になる。

## 0.0.16 — グリッドは見出しがあるときだけ見出しを読む

`chdef_grid_parse`（C）と `Grid.Parse`（.NET）は先頭レコードを常に見出しとして扱っていた。レイアウト側の解析が元から使っている規則（[format.jp.md](spec/format.jp.md) §2）で読むようになった: 先頭レコードが見出しになるのは、渡した語彙で `number` を名指しているときだけ。そうでなければ見出し無しとして位置で読む。

**先頭レコードが見出しでないファイルはデータ行が 1 つ増え**、findings の持つ行番号もそのぶん動く。見出しのあるファイルは今までどおり。行を添字で触る、あるいは finding の行番号を検査しているなら、そこが見る箇所。

`CHDEF_ABI_VERSION` は 6 から 7。シンボルは追加のみで撤去は無いので、利用側の検査（ライブラリが宣言の必要とする版以上であること）はそのまま通る。

JavaScript は影響なし — `Table.parse` は元からこの挙動。

クレート利用側に直すものは無い。隣に増えたもの: `chdef-core`（デバイスが持てる raw のみの規則。`no_std`、依存無し、アロケータ無し、浮動小数点無し）と `chdef-gen`（定義を定数表に展開し、Rust ソースか C ヘッダとして書き出す）。
