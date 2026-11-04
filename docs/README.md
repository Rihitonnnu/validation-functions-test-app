# ドキュメント

`validation-functions-test-app` の実装解説です。

| ドキュメント | 内容 |
| --- | --- |
| [cart-quantity-limit.md](./cart-quantity-limit.md) | カート全体の購入数量上限(合計 31 点以上でブロック)をチェックする Validation Function の解説 |

## プロジェクト概要

Shopify の [Cart and Checkout Validation Function](https://shopify.dev/docs/api/functions/latest/cart-and-checkout-validation) を試すためのデモアプリです。Shopify の「Extension only」アプリテンプレート(Preact + Vite)から作成されており、**このプロジェクトで実装した中心は `cart-quantity-limit` Function** です。

### ディレクトリ構成

```
.
├── shopify.app.toml            # アプリ設定(スコープ、メタオブジェクト定義など)
├── package.json                # npm workspaces(extensions/*)
├── extensions/
│   ├── cart-quantity-limit/    # ★ 実装: カート全体の数量上限 Validation Function (Rust)
│   ├── app-home/               # テンプレート由来: 管理画面のアプリホーム UI (Preact)
│   └── app-tools/              # テンプレート由来: Sidekick 用の FAQ 読み取りツール
├── shared/                     # テンプレート由来: FAQ メタオブジェクト操作の共通コード
└── docs/                       # このドキュメント
```

### 各拡張機能の位置づけ

| 拡張機能 | 種別 | 区分 | 概要 |
| --- | --- | --- | --- |
| `cart-quantity-limit` | `function` | **実装** | カート全体の合計数量が 31 点以上になるとチェックアウトをブロック |
| `app-home` | `ui_extension` (`admin.app.home.render`) | テンプレート | FAQ(メタオブジェクト)の管理画面サンプル |
| `app-tools` | `ui_extension` (`admin.app.tools.data`) | テンプレート | アプリ内の FAQ を Sidekick から読めるようにするツール |

`app-home` / `app-tools` / `shared` はテンプレートのサンプル実装で、Validation Function とは独立しています(FAQ メタオブジェクトの定義は `shopify.app.toml` の `[metaobjects.app.faq]` にあります)。

## セットアップと開発

前提:

- [Shopify CLI](https://shopify.dev/docs/apps/tools/cli/getting-started)
- Node.js / npm
- Rust(`rustup target add wasm32-unknown-unknown`)

```bash
npm install
npm run dev      # 開発サーバー起動
```

Function のテスト:

```bash
cd extensions/cart-quantity-limit
npx vitest run
```

その他のコマンド(`npm run build` / `npm run deploy` など)は [cart-quantity-limit.md](./cart-quantity-limit.md#5-ビルド開発デプロイ) を参照してください。
