# delivery-cool-split の処理の流れ(シーケンス図)

[delivery-cool-split](./delivery-cool-split.md) の、準備から配送オプションが絞られて表示されるまでの流れです。

```mermaid
sequenceDiagram
    autonumber
    actor Dev as 開発者(兼 買い手)
    participant CLI as Shopify CLI<br/>(app dev)
    participant Store as ストア<br/>(Admin API)
    participant Checkout as チェックアウト<br/>(Shopify)
    participant Fn as Function<br/>(wasm)

    rect rgba(150, 150, 150, 0.12)
    Note over Dev,Fn: 準備(1回だけ)
    Dev->>CLI: npm run dev
    CLI->>Store: wasm を開発版として反映
    Dev->>Store: コレクション・商品を作成
    Dev->>Store: deliveryCustomizationCreate(Function を有効化)
    Store-->>Dev: deliveryCustomization の ID
    Dev->>Store: metafieldsSet(collectionIds を設定)
    end

    rect rgba(30, 160, 120, 0.12)
    Note over Dev,Fn: 実行時(チェックアウトのたびに毎回)
    Dev->>Checkout: カートに入れて決済へ進む
    Checkout->>Store: メタフィールドから $collectionIds を取得
    Store-->>Checkout: collectionIds
    Checkout->>Fn: 入力(カート行の所属コレクション・配送料金)
    Note right of Fn: クール品/通常品の有無を判定し、<br/>title 一致の料金を隠す
    Fn-->>Checkout: deliveryOptionHide(隠す料金の handle)
    Checkout-->>Dev: 残りの配送料金を表示
    end
```

## 各ステップの補足

| # | 内容 |
| --- | --- |
| 1-2 | `shopify app dev` が Function をビルドし、開発ストアに開発版として反映する。`deploy` は使わない |
| 3 | 「クール便対象」コレクションと、クール品・通常品のテスト商品を作る |
| 4-5 | `functionHandle: "delivery-cool-split"` で有効化する。このとき deliveryCustomization が作られる |
| 6 | deliveryCustomization の `$app:delivery-cool-split` / `variables` に `{"collectionIds": [...]}` を設定する。ショップやアプリ設置ではなく、deliveryCustomization 自体に付ける |
| 7-9 | チェックアウトが 6 の値を読み、入力クエリの変数 `$collectionIds` に埋めてから Function を呼ぶ |
| 10 | 判定ルールは [delivery-cool-split.md](./delivery-cool-split.md) の「2. 仕様」を参照 |
| 11-12 | Function は隠す料金を返すだけで、残った料金はそのまま表示される |

## 判定の流れ(Function の中)

```mermaid
flowchart TD
    A[カートの各行を見る] --> B{クール便対象<br/>コレクションに所属?}
    B -- はい --> C[クール品あり]
    B -- いいえ --> D[通常品あり]
    C --> E{クール品と通常品}
    D --> E
    E -- 通常品のみ --> F[クール便・通常便＋クール便を隠す]
    E -- クール品のみ --> G[通常便・通常便＋クール便を隠す]
    E -- 両方 --> H[通常便を隠す]
    E -- どちらもなし --> I[何も隠さない]
```
