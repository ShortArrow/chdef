# ガイド

🌐 [English](./guide.md) | **日本語**

各作業への最短経路。形式が**何であるか**は[仕様書](./spec/README.jp.md)にある。このページは、目的を果たすための道順。

ここに出る呼び出しは全て Rust・C ABI・.NET・JavaScript バインディングの 4 経路に存在する（綴りは各言語の流儀）。例は Rust で書くが、各バインディングの綴りはそれぞれの README にある。

## 全体の形

```
CSV バイト列 ──▶ Table（セル）──▶ Rows（チャンネル）──▶ Layout（位置）──▶ フレーム
                 ここで編集        ここで解釈           ここで encode / decode
```

何もキャッシュしない。レイアウトは値であり、セルを編集して解釈し直せば位置は全て再計算される。`encode` / `decode` は `&self` を取り、何も保持しないので、レイアウトはスレッド間でも線の間でも共有できる。

## 定義を読む

```rust
let channels = chdef::parse_ch_csv(ch_csv)?;
let bitfields = chdef::parse_bf_csv(bf_csv)?;
let layout = chdef::build_layout(channels.value, bitfields.value);
```

いずれも `Parsed { value, issues }` を返す。**1 行の問題が読み込みを止めることはない** — 指摘は `issues` に、指す行と列を持って返り、値はその隣にある。`Err` になるのは**ファイルが読めない**ときだけ（文字コード、閉じない引用符）。

`issues` を無視する利用側には「できる限り読めた定義」が渡る。表示する利用側は、その場でセルを指せる。

見出しが別の綴りなら、一度だけ教える:

```rust
let vocabulary = chdef::ColumnVocabulary::japanese();
let channels = chdef::parse_ch_csv_with(ch_csv, &vocabulary)?;
```

## フレームを送る

```rust
let mut frame = layout.encode(&[(2, chdef::Value::Physical(120.0))]).value;
layout.seal(&mut frame);
```

- 名指さなかったチャンネルは `値(デフォルト)` を取る。`BF` チャンネルの既定値には各ビットの既定値が合成済み。
- **カウンタの値は、巡回させてから自分で渡す。** chdef は進めない — カウンタはフレームを送る線に属し、1 つの定義を複数の線が共有しうるから。`Value::Raw(n & mask)` で渡すこと。**物理値で幅を超えると巡回せず飽和する**。
- **`seal` が算出チャンネル（CRC）を埋める。** `encode` は決して埋めないので、渡したものの純粋な関数のままでいられる。封じるのは、他が全て入った後に 1 度だけ。
- 幅に収まらない値はクランプされ、**かつ報告される**（`encode_value_clamped`）。線に乗る数が頼んだ数と違うので、chdef はそれを言う。

## フレームを受ける

```rust
for reading in layout.decode(&frame) {
    println!("{} = {} {}", reading.channel.name, reading.value, reading.channel.unit);
    for (bit, set) in reading.bits() {
        println!("  {} = {}", bit.name, set);
    }
}
```

- 短いフレームからはみ出すチャンネルは落ち、それ以降も落ちる。ゼロ埋めはしない。
- `derived_mismatches(&frame)` が受信側の検め。格納値が手順と食い違う算出チャンネルを名指す。**chdef が「このフレームは間違っている」と言う唯一の場所**。
- `readings_out_of_range(&readings)` は、宣言された `値(最小)` / `値(最大)` の外にある読みを名指す。範囲を勝手に適用することはない。

## 問い合わせる

以下はどれも何も変えず、レイアウトに記憶されない。答えが欲しいかどうかはその時々の問い。

| 問い | 答えるもの |
|---|---|
| `limits_exceeded()` | 指定したバイト数上限・チャンネル数上限に対するレイアウト |
| `values_out_of_range(&values)` | これから送る値と、その範囲 |
| `readings_out_of_range(&readings)` | 届いたフレームについて同じこと |
| `ChTable::defaults_out_of_range()` | 自分の行の範囲を破る `値(デフォルト)` セル。**グリッドの行と列付き**なのでエディタがセルを塗れる |
| `derived_mismatches(&frame)` | 算出チャンネルと手順 |
| `fits_width(value)` | 送る前に、1 つの値が幅に収まるか |

## 定義ファイルを編集する

```rust
let mut table = chdef::ChTable::parse(text)?;
table.set_cell(0, 1, "4");
let written = table.to_csv();
```

- `Grid` はファイルをセルとして持つ（コメント行・空行・知らない列も含む）。読んだ形のまま書き戻し、既に書き出し規則に従うファイルならバイト単位で往復する。
- `insert_channel_renumbering` は連番を保って挿入し、動いた `(旧, 新)` の組を `Renumbered { moved }` で返す。2 ファイルの外にある参照を直すのは利用側の仕事で、`moved` はそれに必要十分な情報。
- **チャンネルに属する事実はセルに書く。別の場所の定数にしない。** `種別` が誰が埋めるかを、`算出` が CRC をどう出すかを言う。セルは行と一緒に動くが、定数は動かない。

## chdef がやらないこと

いずれも理由があって、知っておく価値がある:

| | 理由 |
|---|---|
| カウンタを進める | カウント値は線に属し、1 つの定義を複数の線が使いうる |
| `値(最小)` / `値(最大)` を適用する | 範囲外の値も渡されたとおりに書かれる。隠れるものが無いので、告白すべきことがない |
| CRC の対象範囲を推測する | 対象はプロトコルの性質であり、誤った推測は「ハードウェアが黙ってフレームを捨てる」形で現れる |
| 知らないチェックサムを計算する | ただし `covered_bytes` が対象バイト列をそのまま渡すので、正しい範囲に対して自分の計算を当てられる |
| バイト順を選ぶ | CSV に書かれていない。利用側が設定する |
| 教えていない見出しの綴りを読む | 語彙は利用側が渡すデータ |

## 診断

`Issue` は、安定した `code`、指す `row` / `col`、あれば `channel` / `bit`、使えなかった値（`found`）、代わりに使った値または超えた境界（`used`）を持つ。`message` は英語の散文で、**文言は契約ではない** — フィールドから自分の言葉で文を書く。

`IssueCode::all()` が全コードを列挙するので、コードを鍵にした表の網羅をビルド時に証明できる（実行時に足りないと気づくのではなく）。

各コードの意味は [docs/spec/diagnostics.jp.md](./spec/diagnostics.jp.md)。
