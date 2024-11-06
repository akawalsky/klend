use std::convert::TryFrom;

use anchor_lang::prelude::*;
use stork_sdk::temporal_numeric_value::{TemporalNumericValueFeed, TemporalNumericValue};
use super::{
    types::{Price, TimestampedPriceWithTwap},
    utils, TimestampedPrice,
};
use crate::LendingError;

pub(super) fn get_stork_price_and_twap(
    stork_price_info: &AccountInfo,
    stork_twap_info: Option<&AccountInfo>,
) -> Result<TimestampedPriceWithTwap> {
    let price_feed = TemporalNumericValueFeed::try_from(stork_price_info).map_err(|e| {
        msg!("Error loading stork price feed: {:?}", e);
        error!(LendingError::PriceNotValid)
    })?;
    let feed_id = price_feed.id;
    let price = price_feed
        .get_latest_canonical_temporal_numeric_value_unchecked(&feed_id)
        .map_err(|e| {
            msg!("Error getting stork price: {:?}", e);
            error!(LendingError::PriceNotValid)
        })?;
    validate_stork_price(price.quantized_value)?;

    let twap_tsp: Option<TimestampedPrice> = stork_twap_info.and_then(|twap_info| TemporalNumericValueFeed::try_from_slice(&mut &twap_info.data.borrow()[..])
                .ok())
                .and_then(|feed| feed.get_latest_canonical_temporal_numeric_value_unchecked(&feed_id).ok())
                .filter(|t| validate_stork_price(t.quantized_value).is_ok())
                .map(|t| t.into());

    Ok(TimestampedPriceWithTwap {
        price: price.into(),
        twap: twap_tsp,
    })
}

pub(super) fn validate_stork_price(
    price_value: i128,
) -> Result<()> {
    let abs_value = price_value.unsigned_abs() / 10u128.pow(18);
    let price = u64::try_from(abs_value).unwrap_or(0);
    if price == 0 {
        return err!(LendingError::PriceIsZero);
    }
    Ok(())
}

impl From<TemporalNumericValue> for TimestampedPrice {
    fn from(stork_price: TemporalNumericValue) -> Self {
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
