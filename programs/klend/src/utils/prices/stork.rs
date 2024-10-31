use std::convert::TryFrom;

use anchor_lang::prelude::*;
use stork_sdk::temporal_numeric_value::TemporalNumericValueFeed;
use super::{
    types::{Price, TimestampedPriceWithTwap},
    utils, TimestampedPrice,
};
use crate::LendingError;

pub(super) fn get_stork_price_and_twap(
    stork_price_info: &AccountInfo,
    stork_twap_info: &AccountInfo,
) -> Result<TimestampedPriceWithTwap> {
    let price_feed: TemporalNumericValueFeed = stork_sdk::temporal_numeric_value::TemporalNumericValueFeed::try_deserialize(
        &mut &stork_price_info.data.borrow()[..]
    ).map_err(|e| {
        msg!("Error loading stork price feed: {:?}", e);
        error!(LendingError::PriceNotValid)
    })?;

    let feed_id = price_feed.id;
    let price = price_feed
        .get_latest_canonical_temporal_numeric_value_unchecked(&feed_id)
        .ok()?;

    validate_stork_price(price.quantized_value)?;

    let twap: TemporalNumericValueFeed = stork_sdk::temporal_numeric_value::TemporalNumericValueFeed::try_deserialize(
        &mut &stork_twap_info.data.borrow()[..]
    )?;

    validate_stork_price(twap.quantized_value)?;

    let twap = twap
        .get_latest_canonical_temporal_numeric_value_unchecked(&feed_id)
        .ok()?;
    Ok(TimestampedPriceWithTwap {
        price: price.into(),
        twap: Some(twap.into()),
    })
}

pub(super) fn validate_stork_price(
    price_value: i128,
) -> Result<()> {
    let abs_value = price_value.unsigned_abs();
    let price = u64::try_from(abs_value).unwrap_or(0);
    if price == 0 {
        return err!(LendingError::PriceIsZero);
    }
    Ok(())
}

impl From<stork_sdk::temporal_numeric_value::TemporalNumericValue> for TimestampedPrice {
    fn from(stork_price: stork_sdk::temporal_numeric_value::TemporalNumericValue) -> Self {
        let exp = 18u32;  // 10^18 decimal places
        let quantized_value = stork_price.quantized_value;

        // Convert nanoseconds to seconds
        let timestamp = stork_price.timestamp_ns / 1_000_000_000;

        let price_load = Box::new(move || {
            let price = Price { value: quantized_value, exp };
            Ok(utils::price_to_fraction(price))
        });

        TimestampedPrice {
            price_load,
            timestamp,
        }
    }
}
