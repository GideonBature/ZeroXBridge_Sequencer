use crate::error::TreeBuilderError;
use crate::types::Result;
use num_bigint::BigUint;
use num_traits::Num;

pub fn felt252_to_hex(felt: &str) -> Result<String> {
    let big_uint = BigUint::from_str_radix(felt, 10).map_err(|e| {
        TreeBuilderError::InvalidLeafHash(format!("Failed to parse felt252: {}", e))
    })?;

    let mut hex_str = format!("{:x}", big_uint);
    if hex_str.len() % 2 != 0 {
        hex_str = format!("0{}", hex_str);
    }
    while hex_str.len() < 64 {
        hex_str = format!("0{}", hex_str);
    }

    Ok(hex_str)
}
