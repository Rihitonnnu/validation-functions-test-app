# cart-quantity-limit(カート全体の購入数量上限バリデーション)

カート内の**全商品の合計数量が 30 点を超えたとき**(31 点以上)、チェックアウトへの進行をブロックして日本語のエラーメッセージを表示する Shopify Function です。サイズや色が異なるバリアント(SKU)は別の行として扱い、すべての行の数量を合計して判定します。

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
| 判定対象 | `cart.lines[].quantity` を**全行ぶん合計**した値(サイズ・色違いのバリアントも別の行として数え、すべて合算する) |
| 上限値 | カート全体で `30`(`MAX_QUANTITY_PER_CART` 定数) |
| 合計 30 点まで | **許可**(エラーなし) |
| 合計 31 点以上 | **ブロック**(エラーを 1 件返す) |
| 空のカート | 許可(エラーなし) |
| エラー位置(`target`) | `$.cart`(カート全体に紐づくエラー) |
| エラーメッセージ | `カート内の商品は合計30点までご購入いただけます。数量を減らしてください。` |

### 判定のイメージ

```
カート: [30]            → 合計 30 → OK(ちょうど上限)
カート: [31]            → 合計 31 → NG(1 行でも合計で判定)
カート: [12, 18]        → 合計 30 → OK
カート: [12, 19]        → 合計 31 → NG
カート: [12, 20]        → 合計 32 → NG(別商品・別バリアントでも合算する)
カート: []              → 合計 0  → OK
```

> **カート画面での挙動**: この Function は数量の変更自体を止めるのではなく、**チェックアウトに進もうとしたとき**にエラーを返して進行をブロックします。カート画面では 31 点以上に増やせますが、チェックアウトへは進めません(エラーの表示位置はテーマ・チェックアウトの実装に依存します)。

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
    └── fixtures/                             # 入力と期待出力のペア(6 ケース)
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
    }
  }
}
```

判定に必要な各行の `quantity` **だけ**を取得しています。Function の入力は小さいほど実行が速く、Function の入力サイズ上限・命令数上限にも余裕が出るため、必要最小限のフィールドに絞るのが基本方針です。

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
const MAX_QUANTITY_PER_CART: i32 = 30;

#[shopify_function]
fn cart_validations_generate_run(
    input: schema::cart_validations_generate_run::Input,
) -> Result<schema::CartValidationsGenerateRunResult> {
    let total_quantity: i32 = input.cart().lines().iter().map(|line| *line.quantity()).sum();

    let mut errors = vec![];
    if total_quantity > MAX_QUANTITY_PER_CART {
        errors.push(schema::ValidationError {
            message: format!("カート内の商品は合計{}点まで…", MAX_QUANTITY_PER_CART),
            target: "$.cart".to_owned(),
        });
    }

    let operation = schema::ValidationAddOperation { errors };

    Ok(schema::CartValidationsGenerateRunResult {
        operations: vec![schema::Operation::ValidationAdd(operation)],
    })
}
```

処理の流れ:

1. すべてのカート行の `quantity` を合計する。
2. 合計が `MAX_QUANTITY_PER_CART`(30)を**超えた**場合のみ、`ValidationError` を 1 件作る。
3. エラーの配列(空の場合もある)を `ValidationAdd` オペレーションに包んで返す。

ポイント:

- **常に `ValidationAdd` オペレーションを 1 つ返す**設計です。問題がなければ `errors` は空配列になり、チェックアウトはそのまま進みます(テストの期待値もこの形)。
- 上限値は `MAX_QUANTITY_PER_CART` 定数に切り出してあり、メッセージにも埋め込まれるため、**上限を変更するときは定数 1 か所の修正で済みます**(ただしテストの fixture のメッセージ・数量も合わせて更新が必要)。
- カートの 1 行は 1 バリアント(SKU)に対応するため、サイズや色が違う商品は別の行として数えられ、**すべて合算**されます。
- `target` に `$.cart` を指定しているため、エラーは特定のカート行ではなく**カート全体のエラー**として表示されます。
- 比較は `>`(超過)なので、ちょうど 30 点は許可されます。

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

| ファイル | 入力(各行の数量) | 期待結果 |
| --- | --- | --- |
| `empty-cart.json` | なし | エラーなし |
| `single-line-total-is-30.json` | 30 | エラーなし(境界値・許可) |
| `single-line-exceeds-30.json` | 31 | エラー 1 件(境界値・ブロック) |
| `multiple-lines-total-is-30.json` | 12, 18 | エラーなし(複数行の合計が上限ちょうど) |
| `multiple-lines-total-is-31.json` | 12, 19 | エラー 1 件(複数行の合計が 31) |
| `multiple-lines-total-is-32.json` | 12, 20 | エラー 1 件(別商品どうしでも合算してブロック) |

境界値(30 / 31)、空カート、複数行にまたがる合算を押さえています。

### 4.3 実行方法

```bash
cd extensions/cart-quantity-limit
npx vitest run
```

前提: Rust ツールチェーンと `wasm32-unknown-unknown` ターゲットが必要です。

```bash
rustup target add wasm32-unknown-unknown
```

> 動作確認: ドキュメント作成時点で 6 件すべてパスすることを確認済みです(`Tests 6 passed (6)`)。

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

- **判定の単位はカート全体の合計数量**です。商品(Product)・バリアント・SKU ごとの上限ではありません。サイズや色が違うバリアントも別の行としてすべて合算します。
- 上限値(30)は**コードに固定**されています。ストア管理者が画面から変えられるようにするには、Function 設定用メタフィールドを入力クエリで読み込む構成に拡張する必要があります。
- 見ているのは**今のカートの内容だけ**です。過去の注文を含めた「累計◯個まで」の制限はできません。
- エラーメッセージは**日本語固定**です。多言語ストア向けには `localization` の取得などが必要です。
- 数量は `i32` で合算しています。実運用上のカート数量では桁あふれの心配はありません。

---

## 7. 参考リンク

- [Cart and Checkout Validation Function API](https://shopify.dev/docs/api/functions/latest/cart-and-checkout-validation)
- [Shopify Functions の概要](https://shopify.dev/docs/apps/build/functions)
- [Rust での Functions 開発](https://shopify.dev/docs/apps/build/functions/programming-languages/rust-for-functions)
