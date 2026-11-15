use super::schema;
use shopify_function::prelude::*;
use shopify_function::Result;

use schema::cart_delivery_options_transform_run::input::cart::lines::Merchandise;

/// ストアの配送料金名。`title` の完全一致で特定する。
const NORMAL: &str = "通常便";
const COOL: &str = "クール便";
const NORMAL_PLUS_COOL: &str = "通常便＋クール便";

#[shopify_function]
fn cart_delivery_options_transform_run(
    input: schema::cart_delivery_options_transform_run::Input,
) -> Result<schema::CartDeliveryOptionsTransformRunResult> {
    let mut has_cool = false;
    let mut has_normal = false;

    for line in input.cart().lines() {
        let Merchandise::ProductVariant(variant) = line.merchandise() else {
            continue;
        };
        if *variant.product().in_cool_collection() {
            has_cool = true;
        } else {
            has_normal = true;
        }
    }

    // カートに判定できる商品がなければ何も隠さない。
    let hidden_titles: &[&str] = match (has_cool, has_normal) {
        (false, true) => &[COOL, NORMAL_PLUS_COOL],
        (true, false) => &[NORMAL, NORMAL_PLUS_COOL],
        (true, true) => &[NORMAL],
        (false, false) => &[],
    };

    let operations = input
        .cart()
        .delivery_groups()
        .iter()
        .flat_map(|group| group.delivery_options())
        .filter(|option| {
            option
                .title()
                .is_some_and(|title| hidden_titles.contains(&normalize(title).as_str()))
        })
        .map(|option| {
            schema::Operation::DeliveryOptionHide(schema::DeliveryOptionHideOperation {
                delivery_option_handle: option.handle().to_owned(),
            })
        })
        .collect();

    Ok(schema::CartDeliveryOptionsTransformRunResult { operations })
}

/// 前後の空白を除き、半角の「+」を全角の「＋」にそろえる(料金名の表記ゆれ対策)。
fn normalize(title: &str) -> String {
    title.trim().replace('+', "＋")
}
