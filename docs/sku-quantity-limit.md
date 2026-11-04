# sku-quantity-limit(SKU ごとの購入数量上限バリデーション)

バリアントに設定した**購入上限数**(メタフィールド `custom.max_quantity`)を超えて同じ SKU がカートに入っていると、チェックアウトへの進行をブロックし、日本語のエラーメッセージを表示する Shopify Function です。カート全体の合計を見る [`cart-quantity-limit`](./cart-quantity-limit.md) とは**独立した Function** で、2 つを同時に有効化できます。

- 種別: Cart and Checkout Validation Function
- ターゲット: `cart.validations.generate.run`
- API バージョン: `2026-10`
- 言語 / ランタイム: Rust → WebAssembly(`wasm32-unknown-unknown`)
- 使用クレート: [`shopify_function`](https://crates.io/crates/shopify_function) 2.3.0
- 場所: [`extensions/sku-quantity-limit/`](../extensions/sku-quantity-limit)

---

## 1. 仕様

| 項目 | 内容 |
| --- | --- |
| 上限値の保存先 | バリアントのメタフィールド `custom.max_quantity`(マーチャント所有、`number_integer`、最小値 1) |
| 集計キー | バリアントの `sku`(前後の空白は除去)。同じ SKU が複数の行・複数のバリアントにまたがる場合は**合算**する |
| SKU が空のバリアント | バリアント ID で集計する(上限が設定されていれば効く) |
| 同じ SKU で上限が異なる | **小さいほう**の上限を使う |
| 上限が未設定 / 数値でない / 1 未満 | 制限なし |
| 合計が上限以下 | **許可**(上限ちょうども許可) |
| 合計が上限を超える | **ブロック**(違反した SKU ごとにエラーを 1 件) |
| エラーの順序 | カートの行順(SKU が最初に出てきた順) |
| エラー位置(`target`) | `$.cart`。Validation API ではカート行を `target` に指定できないため |
| エラーメッセージ | `「{商品名} {バリアント名}」は{上限}点までご購入いただけます(現在{数量}点)。数量を減らしてください。` |

バリアント名が `Default Title`(バリアントが 1 つだけの商品に Shopify が付ける名前)や空の場合、表示名は商品名だけになります。

### 判定のイメージ

```
SKU-A(上限 2)× 2                      → OK(上限ちょうど)
SKU-A(上限 2)× 3                      → NG
SKU-A(上限 2)× 1 + SKU-A × 2(別の行) → 合計 3 → NG(行が分かれても合算)
SKU-A(上限 2)× 2 + SKU-B(上限 1)× 2  → SKU-B だけ NG(エラー 1 件)
SKU-C(上限なし)× 50                    → OK
```

---

## 2. 上限値の保存先を選んだ理由

| 方式 | 採否 | 理由 |
| --- | --- | --- |
| **バリアントのメタフィールド(マーチャント所有)** | ✅ 採用 | 各カート行に値が付いて Function に届くため、SKU がいくつ増えても上限がない。管理画面・一括エディタ・ほかのアプリからも編集できる |
| Validation 本体に JSON で `{SKU: 上限}` を持たせる | ❌ | Function の入力クエリは **10,000 バイトを超えるメタフィールドを返さない**(`null` になる)。数百 SKU で上限に達し、超えた瞬間に**全 SKU の制限が黙って外れる** |
| バリアントのメタフィールド(アプリ所有 `$app`) | ❌ | 値は管理画面で編集できるが、定義を変更できるのはこのアプリだけで、**ほかのアプリ(CSV 一括入稿アプリや PIM/ERP 連携など)からは読み書きできない** |
| コレクションごとの段階制限 | ❌ | SKU ごとに細かい値を設定できない |

> **マーチャント所有を選んだことによる注意**: 定義はマーチャントやほかのアプリからも変更・削除できます。定義が削除されたり、キーや型が変わったりすると、Function は値を読めず**すべて「制限なし」**になります(Function はクラッシュせずにチェックアウトを通します)。`custom.max_quantity` の定義は消さず、名前空間・キー・型を変えないでください。

---

## 3. セットアップ

### 3.1 メタフィールドの定義を作る(ストアごとに 1 回)

マーチャント所有にするため、`shopify.app.toml` には書きません(TOML で書いた定義はアプリ所有になります)。次のどちらかで作成します。

**管理画面から**: 「設定 > カスタムデータ > バリアント > 定義を追加」

| 項目 | 値 |
| --- | --- |
| 名前 | 購入上限数 |
| 名前空間とキー | `custom.max_quantity` |
| 型 | 整数(1 つの値) |
| 最小値 | 1 |

**Admin GraphQL API から**:

```graphql
mutation CreateMaxQuantityDefinition {
  metafieldDefinitionCreate(
    definition: {
      name: "購入上限数"
      namespace: "custom"
      key: "max_quantity"
      description: "この SKU を 1 回の注文で購入できる上限数。未設定なら制限なし。"
      type: "number_integer"
      ownerType: PRODUCTVARIANT
      validations: [{ name: "min", value: "1" }]
    }
  ) {
    createdDefinition { id namespace key }
    userErrors { field message code }
  }
}
```

### 3.2 上限値を設定する

管理画面のバリアント編集画面(または商品の一括エディタ)で「購入上限数」を入力します。API から設定する場合:

```graphql
mutation SetVariantMaxQuantity {
  metafieldsSet(
    metafields: [
      {
        ownerId: "gid://shopify/ProductVariant/1234567890"
        namespace: "custom"
        key: "max_quantity"
        type: "number_integer"
        value: "2"
      }
    ]
  ) {
    metafields { id value }
    userErrors { field message code }
  }
}
```

上限を外すときは、値を空にします(メタフィールドを削除します)。

### 3.3 Validation を有効化する

デプロイしただけでは動きません。Validation を作成して有効化します(`write_validations` スコープが必要です。`shopify.app.toml` に設定済みです)。

```graphql
mutation ActivateSkuQuantityLimit {
  validationCreate(
    validation: {
      functionHandle: "sku-quantity-limit"
      enable: true
      blockOnFailure: true
    }
  ) {
    validation { id }
    userErrors { field message }
  }
}
```

`blockOnFailure` の考え方は [cart-quantity-limit.md §5.1](./cart-quantity-limit.md#51-デプロイ後の有効化) と同じです。

---

## 4. ファイル構成

```
extensions/sku-quantity-limit/
├── shopify.extension.toml                    # Function の定義(api_version 2026-10)
├── Cargo.toml                                # Rust の依存関係・リリースビルド最適化
├── schema.graphql                            # Function API のスキーマ(CLI 生成物、2026-10)
├── locales/en.default.json                   # 拡張機能の表示名・説明
├── src/
│   ├── main.rs                               # エントリポイント + 型生成(typegen)の宣言
│   ├── cart_validations_generate_run.graphql # 入力クエリ
│   └── cart_validations_generate_run.rs      # バリデーションのロジック本体
└── tests/
    ├── default.test.js                       # fixtures を読み込んで実行する統合テスト
    └── fixtures/                             # 入力と期待出力のペア(11 ケース)
```

`main.rs` / `Cargo.toml` / `tests/default.test.js` の役割は [cart-quantity-limit.md §3](./cart-quantity-limit.md#3-各ファイルの解説) と同じです。

---

## 5. 実装の解説

### 5.1 入力クエリ

```graphql
query Input {
  cart {
    lines {
      quantity
      merchandise {
        __typename
        ... on ProductVariant {
          id
          sku
          title
          product { title }
          metafield(namespace: "custom", key: "max_quantity") { value }
        }
      }
    }
  }
}
```

- `merchandise` はユニオン型なので、`... on ProductVariant` でバリアントのときだけフィールドを取ります。Rust 側では `Merchandise::ProductVariant(variant)` として受け取ります。
- `metafield` の値はバリアントごとに届くため、SKU がいくら多くても入力サイズの上限にかかりにくい構成です。
- クエリコストは 8 です(`metafield` が 3、`quantity` / `id` / `sku` / `title` / `product.title` が各 1。上限は 30)。

### 5.2 ロジック(`cart_validations_generate_run.rs`)

1. カートの各行について、集計キー(SKU、空ならバリアント ID)を決める。
2. メタフィールドの値を `parse_max_quantity` で読む。数値でない値や 1 未満の値は `None`(制限なし)になる。
3. 同じキーの集計があれば数量を足し、上限は `min_limit` で小さいほうを残す。なければ、表示名(最初の行の商品名とバリアント名)と一緒に新しく追加する。
4. 上限がある SKU のうち `合計 > 上限` のものについて `ValidationError` を作る。
5. `cart-quantity-limit` と同じく、`ValidationAdd` オペレーションを常に 1 つ返す(問題がなければ `errors` は空配列)。

集計は `HashMap` ではなく `Vec` で持っています。エラーをカートの行順で返すためです。カートの行数は多くても数百なので、線形探索でも性能上の問題はありません。

---

## 6. テスト

```bash
cd extensions/sku-quantity-limit
npx vitest run
```

| ファイル | 内容 | 期待結果 |
| --- | --- | --- |
| `empty-cart.json` | 空のカート | エラーなし |
| `no-limit-set.json` | 上限なしの SKU × 100 | エラーなし |
| `invalid-limit-values-are-ignored.json` | 上限値が `"abc"` と `"0"` | エラーなし(制限なし扱い) |
| `single-sku-at-limit.json` | 上限 2 × 2 | エラーなし(境界値) |
| `single-sku-exceeds-limit.json` | 上限 2 × 3 | エラー 1 件(境界値) |
| `default-variant-title-is-omitted.json` | バリアント名が `Default Title` | 表示名は商品名だけ |
| `same-sku-across-lines-is-summed.json` | 同じ SKU が 2 行(1 + 2、上限 2) | エラー 1 件(合算) |
| `mixed-skus-only-one-exceeds.json` | OK の SKU、NG の SKU、上限なしの SKU が混在 | NG の SKU だけエラー |
| `multiple-skus-exceed-in-cart-order.json` | 2 つの SKU が両方超過 | エラー 2 件(行順) |
| `empty-sku-falls-back-to-variant-id.json` | SKU が `null` と `""` の別バリアント | バリアント ID で別々に判定 |
| `shared-sku-uses-lowest-limit.json` | 同じ SKU で上限 5 と 3 | 上限 3 で判定 |

> 動作確認: 11 件すべてパスすることを確認済みです(`Tests 11 passed (11)`)。判定を `>` から `>=` に変えると 3 件が失敗することも確認しており、境界値のテストが効いています。

---

## 7. 補足・制約

- 見ているのは**今のカートの内容だけ**です。過去の注文を含めた「1 人あたり累計◯個まで」の制限は、この Function だけではできません。
- 上限値は**バリアント単位**で設定します。商品単位の一括設定(商品にデフォルト値を持たせ、バリアントで上書きする方式)は今後の拡張候補です。
- エラーはカート全体(`$.cart`)に表示されます。どの商品が対象かは、メッセージ内の商品名で示しています。
- エラーメッセージは**日本語固定**です。
- 「どの SKU に上限が設定されているか」を一覧する専用画面はありません。商品の一括エディタに「購入上限数」列を追加して確認するか、Admin API で取得します。

---

## 8. 参考リンク

- [Cart and Checkout Validation Function API](https://shopify.dev/docs/api/functions/latest/cart-and-checkout-validation)
- [Metafields for input queries](https://shopify.dev/docs/apps/build/functions/input-queries/metafields-for-input-queries)
- [Metafield limits(Function 入力の 10,000 バイト制限)](https://shopify.dev/docs/apps/build/metafields/metafield-limits)
- [Manage metafield definitions(アプリ所有とマーチャント所有)](https://shopify.dev/docs/apps/build/metafields/definitions)
