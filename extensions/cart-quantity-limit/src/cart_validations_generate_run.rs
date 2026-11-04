use super::schema;
use shopify_function::prelude::*;
use shopify_function::Result;

/// カート全体の合計数量の上限(30 点まで許可、31 点以上でブロック)。
const MAX_QUANTITY_PER_CART: i32 = 30;

#[shopify_function]
fn cart_validations_generate_run(
    input: schema::cart_validations_generate_run::Input,
) -> Result<schema::CartValidationsGenerateRunResult> {
    // カートの各行(= バリアント単位。サイズや色が違えば別の行)の数量を、全行ぶん合計する。
    let total_quantity: i32 = input.cart().lines().iter().map(|line| *line.quantity()).sum();

    let mut errors = vec![];
    if total_quantity > MAX_QUANTITY_PER_CART {
        errors.push(schema::ValidationError {
            message: format!(
                "カート内の商品は合計{}点までご購入いただけます。数量を減らしてください。",
                MAX_QUANTITY_PER_CART
            ),
            target: "$.cart".to_owned(),
        });
    }

    let operation = schema::ValidationAddOperation { errors };

    Ok(schema::CartValidationsGenerateRunResult {
        operations: vec![schema::Operation::ValidationAdd(operation)],
    })
}
