# 交換形式

🌐 [English](./interchange.md) | **日本語**

実装済み（0.0.2）: §1・§2 の JSON の形（`serde` feature。`interchange::Definitions` / `Readings` / `ChTable::to_json`。chdef は値を組み立て、直列化は利用側が行う）と、§3 のゴールデンベクタおよびそれを走らせるハーネス。TypeScript 型の生成は未実装。

## 1. JSON

Rust 以外の利用側は chdef の JSON を表示・編集するだけで、CSV を自分では解釈しない。形は次のとおり（キーは固定、追加は後方互換）。

```json
{
  "total_bytes": 13,
  "capacity": 246,
  "endian": "little",
  "channels": [
    {"n": 1, "at": 0, "bytes": 4, "type": "UI", "section": "General", "name": "Frame counter",
     "lsb": 1.0, "offset": 0.0, "unit": "", "default": null,
     "format": "DEC", "min": "", "max": "", "memo": "", "var": "", "favorite": false}
  ],
  "bitfields": [
    {"n": 2, "bit": 0, "name": "Reserved", "default": null, "memo": ""}
  ],
  "issues": [
    {"code": "type_assumed", "row": 3, "col": 5, "message": "type must be UI, SI or BF; assuming UI"}
  ]
}
```

- `lsb` は CSV が 0 / 空欄でも `1.0` を出す（読む側に規則を持たせない）。
- `default` は未指定なら `null`。BF の `default` も同様。
- `capacity` は渡されたときだけ出す。
- 値の JSON（decode 結果）: `{"n": 4, "raw": 65413, "value": -12.3}` の配列。非数は `null`。

TypeScript 型は Rust の型から生成して配布する。

## 2. Table（セル）の JSON

```json
{"header": ["番号", "バイト数", "..."], "rows": [["1", "4", "..."], ["2", "2", "..."]]}
```

編集 UI 用。知らない列も含めて原文のまま。

## 3. ゴールデンベクタ

言語横断の契約。`crates/chdef/vectors/<name>/` に `ch.csv` / `bf.csv` / `vectors.txt` を置き（パッケージの内側なので公開クレートにも同梱される）、各言語のテストが同じファイルを読む。実機の定義は置かない（全て合成）。ベクタ集自身の定義は Issue なしで読めなければならない。

`vectors.txt` の書式（`#` はコメント、空行は無視）:

```
# E <n=値;...> <wire hex>       : 値を入れて encode したフレーム。無指定は既定値、無ければ 0。'-' は全て既定値
# D <wire hex>  <n=raw/値;...>  : フレームを decode した生値と物理値。短いフレームははみ出すチャネルを落とす
# L <total_bytes> <n:at:bytes;...> : レイアウト
E 1=1;2=5;3=2;4=-12.3;5=1.5 0100000005000285ffdc050000
E - 00000000010000000000000000
D 0100000005000285ffdc050000 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3;5=1500/1.5
D 0100000005000285ff 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3
L 13 1:0:4;2:4:2;3:6:1;4:7:2;5:9:4
```

- `E` の値は物理値。`0x` 接頭辞なら生値。
- 物理値の比較は許容誤差 1e-9。
- 各ベクタ集合は最低 1 本ずつ `E` / `D` / `L` を持つ。
