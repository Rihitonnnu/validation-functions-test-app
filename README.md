# Shopify App Template - Extension Only

This is a template for building a [Shopify app](https://shopify.dev/docs/apps/getting-started) using [Preact](https://preactjs.com/) and [Vite](https://vite.dev/). It uses Shopify's [Direct API access](https://shopify.dev/docs/api/app-home#direct-api-access) and [App Bridge](https://shopify.dev/docs/api/app-bridge) to make authenticated calls to the Shopify Admin API directly from the browser — no server required.

Rather than cloning this repo, follow the [Quick Start steps](#quick-start) below.

## Quick start

### Prerequisites

Before you begin, you'll need to [download and install the Shopify CLI](https://shopify.dev/docs/apps/tools/cli/getting-started) if you haven't already.

### Setup

```shell
shopify app init --template=https://github.com/Shopify/shopify-app-template-extension-only
```

### Local Development

```shell
shopify app dev
```

Press P to open the URL to your app. Once you click install, you can start development.

Local development is powered by [Shopify CLI](https://shopify.dev/docs/apps/build/cli-for-apps/test-apps-locally). It logs into your account, connects to an app, provides environment variables, updates remote config, creates a tunnel and provides commands to generate extensions.

## How it works

### Authentication

This template uses [Shopify managed installation](https://shopify.dev/docs/apps/build/authentication-authorization/app-installation). Shopify handles the OAuth flow and app installation automatically. Once installed, the app is fully embedded in the Shopify Admin.

### Querying data

This template uses [Direct API access](https://shopify.dev/docs/api/app-home#direct-api-access) — the Shopify Admin API is called directly from the browser using App Bridge. No server-side code is needed.

This template comes pre-configured with examples of querying data using GraphQL with direct API access, and using [metaobjects](https://shopify.dev/docs/apps/custom-data/metaobjects) to store and retrieve structured app data — see [/shared/models/faq.ts](./shared/models/faq.ts).

### App Bridge

[App Bridge](https://shopify.dev/docs/api/app-bridge) is loaded automatically in embedded apps.

### Polaris Web Components

This template uses [Polaris Web Components](https://shopify.dev/docs/api/app-home/web-components) — the native custom element version of Polaris that works in any framework (including Preact). No additional package installation is required as they are provided automatically in the Shopify Admin iframe.

## GraphQL Codegen

This template is pre-configured with [GraphQL Codegen](https://the-guild.dev/graphql/codegen) to generate TypeScript types from your GraphQL queries.

To regenerate types after updating queries:

```shell
npm run codegen
```

To watch for changes:

```shell
npm run codegen:watch
```

## Build

Build the app by running:

Using npm:

```shell
npm run build
```

Using yarn:

```shell
yarn build
```

Using pnpm:

```shell
pnpm run build
```

## 動作確認

`cart-quantity-limit`（**同一 SKU の合計数量が 30 点を超えるとチェックアウトをブロックする** Validation Function）の動作確認手順です。上から順に、手軽なものから実際のストアでの確認になっています。

| 判定ルール | 内容 |
| --- | --- |
| 判定単位 | カート内の複数行にまたがる**同一 SKU の数量合計** |
| 上限 | 1 SKU あたり 30 点（ちょうど 30 点は許可、31 点以上でブロック） |
| 対象外 | SKU が未設定のバリアント、カスタム商品 |
| エラー | SKU ごとに 1 件。`SKU「SKU-A」は合計30点までご購入いただけます。カート内の数量を減らしてください。` |

### 前提

- [Shopify CLI](https://shopify.dev/docs/apps/tools/cli/getting-started)、Node.js / npm
- Rust と wasm ターゲット: `rustup target add wasm32-unknown-unknown`
- 手順 3 のみ: 開発ストア（Partner Dashboard で作成）

```shell
npm install
```

### 1. 自動テスト（fixtures）

`tests/fixtures/*.json` を 1 ファイル = 1 テストとして実行します。wasm のビルドも自動で行われます。

```shell
cd extensions/cart-quantity-limit
npx vitest run
```

`Tests 9 passed (9)` と表示されれば OK です。確認しているケースは次のとおりです。

| fixture | 内容 | 期待結果 |
| --- | --- | --- |
| `empty-cart` | 空のカート | エラーなし |
| `same-sku-total-is-30` | SKU-A を 20 + 10 | エラーなし（境界値） |
| `same-sku-total-is-31` | SKU-A を 20 + 11 | SKU-A のエラー 1 件（境界値） |
| `single-line-exceeds-30` | SKU-A を 31 | SKU-A のエラー 1 件 |
| `different-skus-each-at-30` | SKU-A を 30、SKU-B を 30 | エラーなし（合計 60 でも SKU ごとの判定） |
| `only-exceeding-sku-is-reported` | SKU-A を 20 + 11、SKU-B を 30 | SKU-A のエラーのみ |
| `multiple-skus-exceed` | SKU-A 31、SKU-B 31、SKU-C 5 | SKU-A・SKU-B のエラー 2 件 |
| `variant-without-sku-is-ignored` | SKU なしを 40 | エラーなし |
| `custom-product-is-ignored` | カスタム商品を 40 | エラーなし |

### 2. 任意の入力でローカル実行

Shopify CLI で wasm を直接実行し、任意のカート入力に対する出力を確認できます（プロジェクトルートで実行）。

```shell
cat > /tmp/input.json <<'EOF'
{
  "cart": {
    "lines": [
      { "quantity": 20, "merchandise": { "__typename": "ProductVariant", "sku": "SKU-A" } },
      { "quantity": 11, "merchandise": { "__typename": "ProductVariant", "sku": "SKU-A" } }
    ]
  }
}
EOF
```

```shell
shopify app function run \
  --path extensions/cart-quantity-limit \
  --export cart_validations_generate_run \
  --input /tmp/input.json
```

`Output` に次のようにエラーが 1 件含まれていれば OK です。数量を `20` と `10`（合計 30）に変えるとエラーが空配列になります。

```json
{
  "operations": [
    {
      "validationAdd": {
        "errors": [
          {
            "message": "SKU「SKU-A」は合計30点までご購入いただけます。カート内の数量を減らしてください。",
            "target": "$.cart"
          }
        ]
      }
    }
  ]
}
```

### 3. 開発ストアでの確認

実際のカート・チェックアウトでブロックされることを確認します。

#### 3.1 スコープを追加する

Validation をストアに登録する `validationCreate` ミューテーションには `write_validations` スコープが必要です。`shopify.app.toml` の `scopes` に追加してください。

```toml
[access_scopes]
scopes = "write_products,write_validations"
```

#### 3.2 開発サーバーを起動する

```shell
npm run dev
```

開発ストアを選択して起動します。アクセス権の更新（再インストール）を求められた場合は承認してください。起動後、`cart-quantity-limit` が開発プレビューとしてストアに反映されます。

#### 3.3 Validation を有効化する

Function は反映しただけでは動作せず、ストアに Validation として登録する必要があります。`npm run dev` を実行しているターミナルで `g` を押して GraphiQL を開き、次のミューテーションを実行します。

```graphql
mutation {
  validationCreate(
    validation: {
      functionHandle: "cart-quantity-limit"
      enable: true
      blockOnFailure: true
    }
  ) {
    validation { id enabled blockOnFailure }
    userErrors { field message }
  }
}
```

`userErrors` が空で `enabled: true` が返れば登録完了です。登録済みの Validation は次のクエリで確認できます。

```graphql
query {
  validations(first: 10) {
    nodes { id title enabled blockOnFailure }
  }
}
```

#### 3.4 ストアの準備

開発ストアに次の商品を用意します（在庫が 31 点以上、または「在庫切れでも販売を続ける」設定にしておきます）。

| 商品 | SKU |
| --- | --- |
| 商品 A | `SKU-A` |
| 商品 B | `SKU-B` |
| 商品 C | （SKU なし） |

#### 3.5 カート・チェックアウトで確認する

ストアフロントで商品をカートに入れ、カートページまたはチェックアウトで結果を確認します。

| # | カートの中身 | 期待結果 |
| --- | --- | --- |
| 1 | 商品 A × 30 | ブロックされない |
| 2 | 商品 A × 31 | 「SKU「SKU-A」は合計30点までご購入いただけます。…」が表示されチェックアウトに進めない |
| 3 | 商品 A × 30 + 商品 B × 30 | ブロックされない（SKU ごとに判定するため） |
| 4 | 商品 A × 31 + 商品 B × 31 | SKU-A・SKU-B それぞれのエラーが表示される |
| 5 | 商品 C × 40 | ブロックされない（SKU なしは対象外） |

数量を 30 以下に減らすとエラーが消え、チェックアウトに進めることも確認してください。

> 同一 SKU が複数行に分かれるケース（例: 同じ SKU の別バリアントや、line item properties 付きで別行になる場合）は、ストアで再現しにくいため手順 1 の fixtures で確認します。

#### 3.6 後片付け

確認が終わったら、不要になった Validation は削除または無効化します。`id` は 3.3 のクエリで取得できます。

```graphql
mutation DeleteValidation($id: ID!) {
  validationDelete(id: $id) {
    deletedId
    userErrors { field message }
  }
}
```

### 動作確認で失敗したとき

| 症状 | 確認すること |
| --- | --- |
| `vitest` がビルドで失敗する | `rustup target add wasm32-unknown-unknown` を実行済みか |
| `validationCreate` が権限エラーになる | `write_validations` を追加し、`npm run dev` でアクセス権の更新を承認したか |
| `validationCreate` が `userErrors` を返す | `functionHandle` が `cart-quantity-limit` と一致しているか、`npm run dev` が起動中か |
| カートでブロックされない | Validation が `enabled: true` か、商品の SKU が設定されているか（SKU なしは対象外） |

実装の詳細は [docs/cart-quantity-limit.md](./docs/cart-quantity-limit.md) を参照してください。

## Shopify Dev MCP

This template is configured with the Shopify Dev MCP. This instructs [Cursor](https://cursor.com/), [GitHub Copilot](https://github.com/features/copilot), [Claude Code](https://claude.com/product/claude-code), and [Google Gemini CLI](https://github.com/google-gemini/gemini-cli) to use the Shopify Dev MCP.

For more information on the Shopify Dev MCP please read [the documentation](https://shopify.dev/docs/apps/build/devmcp).

## Metafields and Metaobjects

This template uses [metaobjects](https://shopify.dev/docs/apps/custom-data/metaobjects) and [metafields](https://shopify.dev/docs/apps/custom-data/metafields) to store structured app data without a custom database.

### Metaobject: FAQ

The template defines a `faq` metaobject type for storing FAQ entries. Each FAQ has a question, answer, a flag to control visibility on the FAQ page, and optional product associations.

Defined in `shopify.app.toml`:

```toml
[metaobjects.app.faq]
name = "FAQ"

[metaobjects.app.faq.fields.question]
name = "Question"
type = "single_line_text_field"
required = true

[metaobjects.app.faq.fields.answer]
name = "Answer"
type = "multi_line_text_field"
required = true

[metaobjects.app.faq.fields.show_on_faq_page]
name = "Show on FAQ page"
type = "boolean"

[metaobjects.app.faq.fields.products]
name = "Products"
type = "list.product_reference"
```

### Metafield: Product FAQ

A metafield definition links individual products to a FAQ metaobject entry, allowing merchants to associate a FAQ with specific products.

```toml
[product.metafields.app.faq]
name = "FAQ"
description = "FAQ for this product"
type = "metaobject_reference<$app:faq>"
access.admin = "merchant_read_write"
```

These definitions are automatically synced to Shopify when you run `shopify app dev` or `shopify app deploy`. See [/shared/models/faq.ts](./shared/models/faq.ts) for the client-side model that reads and writes these metaobjects via the Admin GraphQL API.

## Resources

Preact & Vite:

- [Preact docs](https://preactjs.com/guide/v10/getting-started)
- [Vite docs](https://vite.dev/)

Shopify:

- [Intro to Shopify apps](https://shopify.dev/docs/apps/getting-started)
- [Direct API access](https://shopify.dev/docs/api/app-home#direct-api-access)
- [Shopify CLI](https://shopify.dev/docs/apps/tools/cli)
- [App Bridge](https://shopify.dev/docs/api/app-bridge)
- [Polaris Web Components](https://shopify.dev/docs/api/app-home/web-components)
- [Metaobjects](https://shopify.dev/docs/apps/custom-data/metaobjects)
- [App extensions](https://shopify.dev/docs/apps/app-extensions/list)
- [Shopify Functions](https://shopify.dev/docs/api/functions)
