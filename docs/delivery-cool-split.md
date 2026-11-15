# delivery-cool-split(クール品／通常品による配送オプションの出し分け)

カートの中身(クール品・通常品)に応じて、チェックアウトに表示する配送オプションを絞る Shopify Function です。

| カートの中身 | 表示する配送 | 隠す配送 |
| --- | --- | --- |
| 通常品のみ | 通常便 | クール便、通常便＋クール便 |
| クール品のみ | クール便 | 通常便、通常便＋クール便 |
| 通常品＋クール品 | クール便 / 通常便＋クール便 | 通常便 |

- 種別: Delivery Customization Function
- ターゲット: `cart.delivery-options.transform.run`
- API バージョン: `2026-10`
- 言語 / ランタイム: Rust → WebAssembly
- 場所: [`extensions/delivery-cool-split/`](../extensions/delivery-cool-split)

## 1. できること・できないこと

Delivery Customization Function は、**ストアの配送設定に既にある配送オプションを隠す・並べ替える・名前を変える**ことだけができます。**配送オプションの追加はできません。**

そのため「クール便」「通常便」「通常便＋クール便」の3つの配送料金は、配送プロファイルで先に作っておく必要があります(料金額もそちらで決まります)。この Function は、カートに応じてその3つのうち不要なものを隠すだけです。

## 2. 仕様

| 項目 | 内容 |
| --- | --- |
| クール品の判定 | 商品が「クール便対象」コレクションに入っていればクール品、入っていなければ通常品 |
| カートの判定 | `ProductVariant` の行だけを見る。数量は無視し、クール品と通常品がそれぞれ1行でもあるかだけで決める |
| 判定できる商品がない(空カートなど) | 何も隠さない |
| 配送オプションの特定 | `title` の完全一致(前後の空白は無視、半角の `+` は全角の `＋` にそろえて比較) |
| 該当する `title` がない | 何も隠さない(料金名がずれても配送自体は止めない) |
| 配送グループが複数ある | すべてのグループの配送オプションを同じルールで処理する |

## 3. コレクション ID をコードに書かない仕組み(入力クエリ変数)

`inAnyCollection(ids: [...])` の ID は入力クエリの引数なので、そのままだとコードに ID を書くことになり、ストア専用になります。そこで ID を変数にしています。

```graphql
query Input($collectionIds: [ID!] = []) {
  cart { lines { merchandise { __typename ... on ProductVariant {
    product { inCoolCollection: inAnyCollection(ids: $collectionIds) }
  }}}}
  ...
}
```

`$collectionIds` の値は、有効化した **deliveryCustomization のメタフィールド**(`$app:delivery-cool-split` / `variables`、型 `json`)から Shopify が埋めます。`shopify.extension.toml` の `[extensions.targeting.input_variables]` で、このメタフィールドを指定しています。

```json
{ "collectionIds": ["gid://shopify/Collection/123456789"] }
```

- メタフィールドは**ショップやアプリ設置ではなく、deliveryCustomization 自体**に設定します。
- コレクションを変えるときはメタフィールドの値を更新するだけで、再デプロイは不要です。
- メタフィールドが未設定のとき `$collectionIds` は空配列になり、全商品が「通常品」扱いになります(つまりクール便が隠れます)。**有効化したらメタフィールドの設定を忘れないでください。**

## 4. セットアップ

1. 配送プロファイルに「クール便」「通常便」「通常便＋クール便」の3つの料金があることを確認する(名前は上の表記と一致させる)。
2. 管理画面で「クール便対象」コレクションを作り、クール品を入れる。テスト用に、クール品と通常品を1点ずつ用意する。
3. `shopify.app.toml` のスコープに `write_delivery_customizations` を追加済み。**スコープを増やしたため、`shopify app dev` / `deploy` 後にストアでアプリの権限を承認し直す必要があります。**
4. `shopify app dev` で開発ストアに反映する。
5. Function を有効化する(Admin GraphQL、`functionHandle` は `delivery-cool-split`)。

   ```graphql
   mutation {
     deliveryCustomizationCreate(deliveryCustomization: {
       title: "クール便/通常便の出し分け"
       enabled: true
       functionHandle: "delivery-cool-split"
     }) {
       deliveryCustomization { id }
       userErrors { field message }
     }
   }
   ```

6. 返ってきた `id`(`gid://shopify/DeliveryCustomization/…`)に、コレクション ID を設定する。

   ```graphql
   mutation {
     metafieldsSet(metafields: [{
       ownerId: "gid://shopify/DeliveryCustomization/…"
       namespace: "$app:delivery-cool-split"
       key: "variables"
       type: "json"
       value: "{\"collectionIds\":[\"gid://shopify/Collection/…\"]}"
     }]) {
       metafields { id }
       userErrors { field message }
     }
   }
   ```

7. チェックアウトで次の3パターンを確認する。
   - 通常品だけ → 「通常便」のみ
   - クール品だけ → 「クール便」のみ
   - 両方 → 「クール便」と「通常便＋クール便」

## 5. テスト

```bash
(cd extensions/delivery-cool-split && npx vitest run)
```

`tests/fixtures/` に9ケースあります(通常のみ・クールのみ・混在・複数行・空カート・`CustomProduct` のみ・料金名が違う・半角 `+`・複数の配送グループ)。混在ケースのルールを壊すと `mixed-hides-normal-only` が失敗することを確認済みです。

## 6. 注意点・今後の課題

- **title 一致のため、配送料金の名前を変えると出し分けが効かなくなります**(配送は止まらず、全部表示される)。本運用では handle をメタフィールドで指定する方式が安全です。
- 判定は `cart.lines` 全体で行っています。配送グループごと(`deliveryGroups[].cartLines`)に判定する方が厳密です。グループが分かれる構成を使うなら見直してください。
- 配送オプションは Shopify 側でキャッシュされる場合があります。カートを変えたあとに表示が更新されるかは、実機で確認してください。
- 配送オプションを並べ替える(`move`)場合は、最安のオプションを先頭にする制約があります。この Function は `hide` だけを使います。
