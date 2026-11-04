use super::schema;
use shopify_function::prelude::*;
use shopify_function::Result;

use schema::cart_validations_generate_run::input::cart::lines::Merchandise;

/// 1 商品しかないバリアントに Shopify が付けるタイトル。表示名には含めない。
const DEFAULT_VARIANT_TITLE: &str = "Default Title";

/// 同じ上限として数量を合算する単位(SKU、SKU が空ならバリアント ID)ごとの集計結果。
struct SkuTotal {
    key: String,
    display_name: String,
    quantity: i32,
    max_quantity: Option<i32>,
}

#[shopify_function]
fn cart_validations_generate_run(
    input: schema::cart_validations_generate_run::Input,
) -> Result<schema::CartValidationsGenerateRunResult> {
    // エラーをカートの行順で返すため、HashMap ではなく出現順を保つ Vec で集計する。
    let mut totals: Vec<SkuTotal> = vec![];

    for line in input.cart().lines() {
        let Merchandise::ProductVariant(variant) = line.merchandise() else {
            continue;
        };

        let key = match variant.sku().map(|sku| sku.trim()) {
            Some(sku) if !sku.is_empty() => sku.to_owned(),
            _ => variant.id().to_owned(),
        };
        let max_quantity = variant
            .metafield()
            .and_then(|metafield| parse_max_quantity(metafield.value()));

        match totals.iter_mut().find(|total| total.key == key) {
            Some(total) => {
                total.quantity += *line.quantity();
                total.max_quantity = min_limit(total.max_quantity, max_quantity);
            }
            None => totals.push(SkuTotal {
                key,
                display_name: display_name(
                    variant.product().title(),
                    variant.title().map(String::as_str),
                ),
                quantity: *line.quantity(),
                max_quantity,
            }),
        }
    }

    let errors = totals
        .iter()
        .filter_map(|total| {
            let max_quantity = total.max_quantity?;
            (total.quantity > max_quantity).then(|| schema::ValidationError {
                message: format!(
                    "「{}」は{}点までご購入いただけます(現在{}点)。数量を減らしてください。",
                    total.display_name, max_quantity, total.quantity
                ),
                target: "$.cart".to_owned(),
            })
        })
        .collect();

    let operation = schema::ValidationAddOperation { errors };

    Ok(schema::CartValidationsGenerateRunResult {
        operations: vec![schema::Operation::ValidationAdd(operation)],
    })
}

/// メタフィールドの値を上限値として読む。数値でない値や 1 未満の値は「上限なし」として扱う。
fn parse_max_quantity(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok().filter(|limit| *limit >= 1)
}

/// 同じ SKU に複数の上限がある場合は、厳しい方(小さい方)を採用する。
fn min_limit(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn display_name(product_title: &str, variant_title: Option<&str>) -> String {
    match variant_title.map(|title| title.trim()) {
        Some(title) if !title.is_empty() && title != DEFAULT_VARIANT_TITLE => {
            format!("{} {}", product_title, title)
        }
        _ => product_title.to_owned(),
    }
}
