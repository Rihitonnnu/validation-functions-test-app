# cart-quantity-limit(SKU 単位の購入数量上限バリデーション)

カート内で**同一 SKU の合計数量が 30 点以上になったとき**、チェックアウトへの進行をブロックして日本語のエラーメッセージを表示する Shopify Function です。同じ SKU が複数のカート行に分かれていても合算して判定します。

- 種別: Cart and Checkout Validation Function
- ターゲット: `cart.validations.generate.run`
- API バージョン: `2026-10`
- 言語 / ランタイム: Rust → WebAssembly(`wasm32-unknown-unknown`)
- 使用クレート: [`shopify_function`](https://crates.io/crates/shopify_function) 2.3.0
- 場所: [`extensions/cart-quantity-limit/`](../extensions/cart-quantity-limit)

---

## 1. 仕様

| 項目 | 内容 |
| --- | --- |
| 判定対象 | `cart.lines[].quantity` を **SKU ごとに合算**した値(`merchandise` の `ProductVariant.sku` をキーにする) |
| ブロックする値 | SKU あたり `30`(`BLOCK_QUANTITY_PER_SKU` 定数) |
| 29 点まで | **許可**(エラーなし) |
| 30 点以上 | **ブロック**(該当する SKU ごとにエラーを 1 件返す) |
| 異なる SKU どうし | 互いに合算しない(SKU-A が 29、SKU-B が 29 なら許可) |
| 同じ SKU が複数行にある場合 | 行をまたいで合算する |
| SKU が未設定のバリアント | **判定対象外**(上限なし) |
| カスタム商品(`CustomProduct`) | **判定対象外**(バリアントを持たないため SKU がない) |
| 空のカート | 許可(エラーなし) |
| エラー位置(`target`) | `$.cart`(カート全体に紐づくエラー) |
| エラーメッセージ | `SKU「{sku}」は合計30点以上ご購入いただけません。カート内の数量を減らしてください。` |
| エラーの並び順 | SKU の辞書順 |

### 判定のイメージ

```
カート: [SKU-A × 20, SKU-A × 9]                → SKU-A 合計 29        → OK
カート: [SKU-A × 20, SKU-A × 10]               → SKU-A 合計 30        → NG(SKU-A のエラー 1 件)
カート: [SKU-A × 31]                           → SKU-A 合計 31        → NG(1 行でも SKU 合計で判定)
カート: [SKU-A × 29, SKU-B × 29]               → A 29 / B 29          → OK(SKU が違えば合算しない)
カート: [SKU-A × 31, SKU-B × 31, SKU-C × 5]    → A 31 / B 31 / C 5    → NG(SKU-A と SKU-B のエラー 2 件)
カート: [SKU 未設定 × 40]                      → 対象外               → OK
カート: []                                     → 合計 0               → OK
```

---

## 2. ファイル構成

```
extensions/cart-quantity-limit/
├── shopify.extension.toml                    # Function の定義(ターゲット・ビルド設定)
├── Cargo.toml                                # Rust の依存関係・リリースビルド最適化
├── schema.graphql                            # Function API のスキーマ(5,000 行超。CLI 生成物)
├── locales/en.default.json                   # 拡張機能の表示名・説明
├── src/
│   ├── main.rs                               # エントリポイント + 型生成(typegen)の宣言
│   ├── cart_validations_generate_run.graphql # Function への入力クエリ
│   └── cart_validations_generate_run.rs      # バリデーションのロジック本体
└── tests/
    ├── default.test.js                       # fixtures を読み込んで実行する統合テスト
    └── fixtures/                             # 入力と期待出力のペア(9 ケース)
```

---

## 3. 各ファイルの解説

### 3.1 `shopify.extension.toml` — Function の定義

```toml
api_version = "2026-10"

[[extensions]]
handle = "cart-quantity-limit"
type = "function"

  [[extensions.targeting]]
  target = "cart.validations.generate.run"
  input_query = "src/cart_validations_generate_run.graphql"
  export = "cart_validations_generate_run"

  [extensions.build]
  command = "cargo build --target=wasm32-unknown-unknown --release"
  path = "target/wasm32-unknown-unknown/release/cart-quantity-limit.wasm"
  watch = ["src/**/*.rs"]
```

- `target` … この Function が呼ばれるタイミング。カート/チェックアウトの検証時に実行されます。
- `input_query` … Function に渡すデータを GraphQL で宣言するファイル。
- `export` … wasm の中でエクスポートされる関数名。Rust 側の関数名(`cart_validations_generate_run`)と一致している必要があります。
- `[extensions.build]` … ビルドコマンドと成果物(`.wasm`)のパス。`watch` により `shopify app dev` 中は Rust ソースの変更で自動再ビルドされます。
- `handle = "cart-quantity-limit"` … 後述の有効化(`validationCreate`)で Function を指定するときの識別子です。

### 3.2 `src/cart_validations_generate_run.graphql` — 入力クエリ

```graphql
query Input {
  cart {
    lines {
      quantity
      merchandise {
        __typename
        ... on ProductVariant {
          sku
        }
      }
    }
  }
}
```

判定に必要な `quantity` と、SKU(`merchandise` が `ProductVariant` のときの `sku`)**だけ**を取得しています。`merchandise` は `CustomProduct | ProductVariant` のユニオン型なので、`... on ProductVariant` で SKU を取り出します。Function の入力は小さいほど実行が速く、Function の入力サイズ上限・命令数上限にも余裕が出るため、必要最小限のフィールドに絞るのが基本方針です。

### 3.3 `src/main.rs` — 型生成とエントリポイント

```rust
#[typegen("schema.graphql")]
pub mod schema {
    #[query("src/cart_validations_generate_run.graphql")]
    pub mod cart_validations_generate_run {}
}
```

`typegen` マクロが `schema.graphql` と入力クエリから、Rust の型(`schema::cart_validations_generate_run::Input`、`schema::CartValidationsGenerateRunResult` など)を**コンパイル時に自動生成**します。クエリで取得していないフィールドには型の上でもアクセスできないため、入力と実装のズレをコンパイルエラーで検出できます。

`main()` は「名前付きエクスポートを呼んでください」とログを出して abort するだけです。Shopify は `main` ではなく `export` で指定した関数を直接呼び出します。

### 3.4 `src/cart_validations_generate_run.rs` — ロジック本体

```rust
const BLOCK_QUANTITY_PER_SKU: i32 = 30;

#[shopify_function]
fn cart_validations_generate_run(
    input: schema::cart_validations_generate_run::Input,
) -> Result<schema::CartValidationsGenerateRunResult> {
    let mut quantity_by_sku: BTreeMap<String, i32> = BTreeMap::new();
    for line in input.cart().lines() {
        if let Merchandise::ProductVariant(variant) = line.merchandise() {
            if let Some(sku) = variant.sku() {
                *quantity_by_sku.entry(sku.clone()).or_insert(0) += *line.quantity();
            }
        }
    }

    let errors = quantity_by_sku
        .into_iter()
        .filter(|(_, quantity)| *quantity >= BLOCK_QUANTITY_PER_SKU)
        .map(|(sku, _)| schema::ValidationError {
            message: format!("SKU「{}」は合計{}点以上…", sku, BLOCK_QUANTITY_PER_SKU),
            target: "$.cart".to_owned(),
        })
        .collect();

    let operation = schema::ValidationAddOperation { errors };

    Ok(schema::CartValidationsGenerateRunResult {
        operations: vec![schema::Operation::ValidationAdd(operation)],
    })
}
```

(実際のコードでは `Merchandise` は `schema::cart_validations_generate_run::input::cart::lines::Merchandise` と完全修飾されています。)

処理の流れ:

1. 各カート行について、`merchandise` が `ProductVariant` で、かつ `sku` が設定されている場合のみ、SKU をキーに `quantity` を `BTreeMap` へ加算する(`quantity_by_sku`)。
2. 合計が `BLOCK_QUANTITY_PER_SKU`(30)**以上**の SKU についてのみ、`ValidationError` を 1 件ずつ作る。
3. エラーの配列(空の場合もある)を `ValidationAdd` オペレーションに包んで返す。

ポイント:

- **常に `ValidationAdd` オペレーションを 1 つ返す**設計です。問題がなければ `errors` は空配列になり、チェックアウトはそのまま進みます(テストの期待値もこの形)。
- ブロックする値は `BLOCK_QUANTITY_PER_SKU` 定数に切り出してあり、メッセージにも埋め込まれるため、**値を変更するときは定数 1 か所の修正で済みます**(ただしテストの fixture のメッセージ・数量も合わせて更新が必要)。
- 集計に `BTreeMap` を使っているため、**エラーの並び順は SKU の辞書順で一定**になります(出力が決定的なのでテストしやすい)。
- `target` に `$.cart` を指定しているため、エラーは特定のカート行ではなく**カート全体のエラー**として表示されます。どの SKU が超過したかはメッセージ内の SKU 名で伝えます。
- 比較は `>=`(以上)なので、ちょうど 30 点もブロックされ、許可されるのは 29 点までです。

### 3.5 `Cargo.toml` — ビルド最適化

```toml
[profile.release]
lto = true
opt-level = 'z'
strip = true
```

Function の wasm には**バイナリサイズの上限**があるため、サイズ最優先(`opt-level = 'z'`)、LTO 有効、シンボル除去の設定にしています。

### 3.6 `locales/en.default.json`

```json
{ "name": "cart-quantity-limit", "description": "cart-quantity-limit" }
```

`shopify.extension.toml` の `t:name` / `t:description` が参照する表示名です。現状は Function のハンドル名と同じ文字列(スキャフォールド時の初期値)です。管理画面での見え方を整えたい場合はここを編集します。

---

## 4. テスト

### 4.1 構成

[`tests/default.test.js`](../extensions/cart-quantity-limit/tests/default.test.js) は `@shopify/shopify-function-test-helpers` と [Vitest](https://vitest.dev/) を使った統合テストです。`tests/fixtures/` 配下の `*.json` を**すべて自動で読み込み、1 ファイル = 1 テスト**として実行します。

各テストは次の順に動きます。

1. `beforeAll` で `cargo build` を実行して wasm をビルドし、Function の情報(スキーマ・ランナー・wasm パス)を取得。
2. fixture の入力クエリ・入力データ・期待出力が**スキーマに適合しているか**を検証(`validateTestAssets`)。
3. 実際に wasm を Function Runner で実行し、出力が fixture の期待値と一致することを検証。

### 4.2 fixture 一覧

| ファイル | 入力(SKU × 数量) | 期待結果 |
| --- | --- | --- |
| `empty-cart.json` | なし | エラーなし |
| `same-sku-total-is-29.json` | SKU-A × 20, SKU-A × 9 | エラーなし(同一 SKU を行をまたいで合算、境界値・許可) |
| `same-sku-total-is-30.json` | SKU-A × 20, SKU-A × 10 | SKU-A のエラー 1 件(境界値・ブロック) |
| `single-line-exceeds-30.json` | SKU-A × 31 | SKU-A のエラー 1 件(単一行で 30 以上) |
| `different-skus-each-at-29.json` | SKU-A × 29, SKU-B × 29 | エラーなし(SKU が違えば合計 58 でも許可) |
| `multiple-skus-exceed.json` | SKU-B × 31, SKU-C × 5, SKU-A × 31 | SKU-A・SKU-B のエラー 2 件(辞書順) |
| `only-exceeding-sku-is-reported.json` | SKU-A × 20, SKU-A × 11, SKU-B × 29 | SKU-A のエラー 1 件(30 以上の SKU のみ報告) |
| `variant-without-sku-is-ignored.json` | SKU 未設定のバリアント × 40 | エラーなし(判定対象外) |
| `custom-product-is-ignored.json` | カスタム商品 × 40 | エラーなし(判定対象外) |

境界値(29 / 30)、空カート、同一 SKU の行またぎ合算、SKU 間で合算しないこと、複数 SKU の同時超過、SKU なし・カスタム商品の除外を押さえています。

### 4.3 実行方法

```bash
cd extensions/cart-quantity-limit
npx vitest run
```

前提: Rust ツールチェーンと `wasm32-unknown-unknown` ターゲットが必要です。

```bash
rustup target add wasm32-unknown-unknown
```

> 動作確認: ドキュメント作成時点で 9 件すべてパスすることを確認済みです(`Tests 9 passed (9)`)。

### 4.4 ケースを追加するには

`tests/fixtures/` に JSON を置くだけでテストが増えます。形式は既存ファイルと同じで、`payload.input`(入力)と `payload.output`(期待出力)を書きます。

---

## 5. ビルド・開発・デプロイ

プロジェクトルートで実行します。

| 目的 | コマンド |
| --- | --- |
| 開発サーバー起動(Function を自動ビルド) | `npm run dev` |
| ビルドのみ | `npm run build` |
| デプロイ | `npm run deploy` |

### 5.1 デプロイ後の有効化

Function はデプロイしただけでは動きません。ストアに **Validation として登録(有効化)** する必要があります。Admin GraphQL API の `validationCreate` ミューテーションで、Function のハンドルを指定して作成します。

```graphql
mutation {
  validationCreate(
    validation: {
      functionHandle: "cart-quantity-limit"
      enable: true
      blockOnFailure: true
    }
  ) {
    validation { id }
    userErrors { field message }
  }
}
```

> **注意(要確認)**: 現在の [`shopify.app.toml`](../shopify.app.toml) の `scopes` は `write_products` のみです。アプリから `validationCreate` を呼ぶ場合は、通常 `write_validations` スコープが必要になるため、有効化の方法(アプリの UI から行う / CLI・GraphiQL で行う等)に合わせてスコープの追加を検討してください。

`blockOnFailure` は「Function 自体が失敗(クラッシュ・タイムアウト等)した場合にチェックアウトをブロックするか」の設定です。このデモでは数量超過を確実に止めたいので `true` の例を載せていますが、可用性を優先する場合は `false` を検討してください。

---

## 6. 補足・制約

- **判定の単位は SKU** です。カート全体の合計や、商品(Product)・バリアント(variant ID)単位の上限ではありません。
- **SKU は Shopify 上で一意性が強制されません**。別々のバリアントに同じ SKU が付いていると、それらは合算されます。バリアントごとに制限したい場合は、入力クエリで `ProductVariant.id` を取得してキーを `id` に変えてください。
- **SKU が未設定のバリアントとカスタム商品は上限の対象外**です。それらも制限したい場合は、バリアント `id` を代替キーにするなど、扱いを決めて拡張が必要です。
- ブロックする値(30)は**全 SKU 共通でコードに固定**されています。SKU ごとに上限を変えたり、ストア管理者が画面から変えられるようにするには、バリアントのメタフィールドや Function 設定用メタフィールドを入力クエリで読み込む構成に拡張する必要があります。
- 見ているのは**今のカートの内容だけ**です。過去の注文を含めた「累計◯個まで」の制限はできません。
- エラーメッセージは**日本語固定**です。多言語ストア向けには `localization` の取得などが必要です。
- 数量は `i32` で合算しています。実運用上のカート数量では桁あふれの心配はありません。

---

## 7. 参考リンク

- [Cart and Checkout Validation Function API](https://shopify.dev/docs/api/functions/latest/cart-and-checkout-validation)
- [Shopify Functions の概要](https://shopify.dev/docs/apps/build/functions)
- [Rust での Functions 開発](https://shopify.dev/docs/apps/build/functions/programming-languages/rust-for-functions)
