use std::collections::BTreeMap;

use super::schema;
use shopify_function::prelude::*;
use shopify_function::Result;

/// 同一 SKU の合計数量がこの値以上になるとブロックする(29 点まで許可)。
const BLOCK_QUANTITY_PER_SKU: i32 = 30;

#[shopify_function]
fn cart_validations_generate_run(
    input: schema::cart_validations_generate_run::Input,
) -> Result<schema::CartValidationsGenerateRunResult> {
    // SKU ごとに、カート内の複数行にまたがる数量を合算する。
    // SKU が未設定のバリアントやカスタム商品(バリアントを持たない)は対象外。
    let mut quantity_by_sku: BTreeMap<String, i32> = BTreeMap::new();
    for line in input.cart().lines() {
        if let schema::cart_validations_generate_run::input::cart::lines::Merchandise::ProductVariant(variant) =
            line.merchandise()
        {
            if let Some(sku) = variant.sku() {
                *quantity_by_sku.entry(sku.clone()).or_insert(0) += *line.quantity();
            }
        }
    }

    let errors = quantity_by_sku
        .into_iter()
        .filter(|(_, quantity)| *quantity >= BLOCK_QUANTITY_PER_SKU)
        .map(|(sku, _)| schema::ValidationError {
            message: format!(
                "SKU「{}」は合計{}点以上ご購入いただけません。カート内の数量を減らしてください。",
                sku, BLOCK_QUANTITY_PER_SKU
            ),
            target: "$.cart".to_owned(),
        })
        .collect();

    let operation = schema::ValidationAddOperation { errors };

    Ok(schema::CartValidationsGenerateRunResult {
        operations: vec![schema::Operation::ValidationAdd(operation)],
    })
}
