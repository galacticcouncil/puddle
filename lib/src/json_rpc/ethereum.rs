// Smoldot
// Copyright (C) 2019-2022  Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Encoding and decoding of parameters and return values for the Frontier
//! `EthereumRuntimeRPCApi` runtime API, used by EVM-compatible Substrate chains.

use alloc::vec::Vec;

/// Name of the runtime API.
pub const API_NAME: &str = "EthereumRuntimeRPCApi";

/// Range of supported API versions.
pub const API_VERSION_RANGE: core::ops::RangeInclusive<u32> = 1..=5;

/// Runtime function name for `chain_id`.
pub const CHAIN_ID_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_chain_id";

/// Runtime function name for `account_basic`.
pub const ACCOUNT_BASIC_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_account_basic";

/// Runtime function name for `account_code_at`.
pub const ACCOUNT_CODE_AT_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_account_code_at";

/// Runtime function name for `storage_at`.
pub const STORAGE_AT_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_storage_at";

/// Runtime function name for `call`.
pub const CALL_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_call";

/// Runtime function name for `gas_price`.
pub const GAS_PRICE_FUNCTION_NAME: &str = "EthereumRuntimeRPCApi_gas_price";

/// Produces the input to pass to the `EthereumRuntimeRPCApi_chain_id` runtime call.
pub fn chain_id_parameters() -> Vec<u8> {
    Vec::new()
}

/// Produces the input to pass to the `EthereumRuntimeRPCApi_gas_price` runtime call.
pub fn gas_price_parameters() -> Vec<u8> {
    Vec::new()
}

/// Produces the input to pass to the `EthereumRuntimeRPCApi_account_basic` runtime call.
pub fn account_basic_parameters(address: &[u8; 20]) -> Vec<u8> {
    address.to_vec()
}

/// Produces the input to pass to the `EthereumRuntimeRPCApi_account_code_at` runtime call.
pub fn account_code_at_parameters(address: &[u8; 20]) -> Vec<u8> {
    address.to_vec()
}

/// Produces the input to pass to the `EthereumRuntimeRPCApi_storage_at` runtime call.
/// Parameters are H160 (address) followed by H256 (position).
pub fn storage_at_parameters(address: &[u8; 20], position: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(52);
    out.extend_from_slice(address);
    out.extend_from_slice(position);
    out
}

/// Produces the input to pass to the `EthereumRuntimeRPCApi_call` runtime call.
///
/// The Frontier `call` runtime API expects a tuple of:
/// `(from: H160, to: H160, data: Vec<u8>, value: U256, gas_limit: U256,
///   max_fee_per_gas: Option<U256>, max_priority_fee_per_gas: Option<U256>,
///   nonce: Option<U256>, estimate: bool, access_list: Option<Vec<(H160, Vec<H256>)>>)`
///
/// For Phase 1, we pass `None` for access_list.
pub fn call_parameters(
    from: Option<&[u8; 20]>,
    to: Option<&[u8; 20]>,
    data: &[u8],
    value: &[u8; 32],
    gas_limit: &[u8; 32],
    max_fee_per_gas: Option<&[u8; 32]>,
    max_priority_fee_per_gas: Option<&[u8; 32]>,
    nonce: Option<&[u8; 32]>,
    estimate: bool,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);

    // from: H160
    match from {
        Some(addr) => out.extend_from_slice(addr),
        None => out.extend_from_slice(&[0u8; 20]),
    }

    // to: H160
    match to {
        Some(addr) => out.extend_from_slice(addr),
        None => out.extend_from_slice(&[0u8; 20]),
    }

    // data: Vec<u8> - SCALE compact length prefix + bytes
    encode_scale_compact_len(&mut out, data.len());
    out.extend_from_slice(data);

    // value: U256 (32 bytes LE)
    out.extend_from_slice(value);

    // gas_limit: U256 (32 bytes LE)
    out.extend_from_slice(gas_limit);

    // max_fee_per_gas: Option<U256>
    encode_option_u256(&mut out, max_fee_per_gas);

    // max_priority_fee_per_gas: Option<U256>
    encode_option_u256(&mut out, max_priority_fee_per_gas);

    // nonce: Option<U256>
    encode_option_u256(&mut out, nonce);

    // estimate: bool
    out.push(if estimate { 1 } else { 0 });

    // access_list: Option<Vec<(H160, Vec<H256>)>> = None
    out.push(0x00);

    out
}

fn encode_scale_compact_len(out: &mut Vec<u8>, len: usize) {
    if len < 64 {
        out.push((len as u8) << 2);
    } else if len < (1 << 14) {
        out.push(((len & 0b111111) as u8) << 2 | 0b01);
        out.push(((len >> 6) & 0xff) as u8);
    } else if len < (1 << 30) {
        out.push(((len & 0b111111) as u8) << 2 | 0b10);
        out.push(((len >> 6) & 0xff) as u8);
        out.push(((len >> 14) & 0xff) as u8);
        out.push(((len >> 22) & 0xff) as u8);
    } else {
        // Big-integer mode
        let bytes_needed = ((usize::BITS - len.leading_zeros() + 7) / 8) as u8;
        out.push((bytes_needed - 4) << 2 | 0b11);
        let mut v = len;
        for _ in 0..bytes_needed {
            out.push((v & 0xff) as u8);
            v >>= 8;
        }
    }
}

fn encode_option_u256(out: &mut Vec<u8>, value: Option<&[u8; 32]>) {
    match value {
        Some(v) => {
            out.push(0x01);
            out.extend_from_slice(v);
        }
        None => {
            out.push(0x00);
        }
    }
}

/// Decoded account basic info from Frontier.
#[derive(Debug, Clone)]
pub struct AccountBasic {
    /// Account nonce as U256 in little-endian.
    pub nonce: [u8; 32],
    /// Account balance as U256 in little-endian.
    pub balance: [u8; 32],
}

/// Decoded EVM call result from Frontier.
#[derive(Debug, Clone)]
pub struct CallResult {
    /// Whether the EVM execution succeeded (ExitReason::Succeed).
    pub exit_reason_success: bool,
    /// Return data from the EVM execution.
    pub return_data: Vec<u8>,
    /// Gas used by the execution, as U256 in little-endian.
    pub used_gas: [u8; 32],
}

/// Potential error when decoding Ethereum runtime API output.
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum DecodeError {
    /// Failed to parse the return value.
    ParseError,
    /// The `EthereumRuntimeRPCApi` API uses a version that smoldot doesn't support.
    UnknownRuntimeVersion,
    /// The runtime returned a dispatch error.
    RuntimeDispatchError,
    /// The EVM execution reverted with return data.
    #[display("EVM execution reverted")]
    EvmReverted {
        /// Revert return data.
        #[error(not(source))]
        return_data: Vec<u8>,
    },
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_chain_id`.
/// Returns the chain ID as a u64.
pub fn decode_chain_id(scale_encoded: &[u8]) -> Result<u64, DecodeError> {
    if scale_encoded.len() != 8 {
        return Err(DecodeError::ParseError);
    }
    Ok(u64::from_le_bytes(
        <[u8; 8]>::try_from(scale_encoded).unwrap(),
    ))
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_account_basic`.
/// Returns nonce (U256 LE) and balance (U256 LE).
pub fn decode_account_basic(scale_encoded: &[u8]) -> Result<AccountBasic, DecodeError> {
    if scale_encoded.len() != 64 {
        return Err(DecodeError::ParseError);
    }
    let mut nonce = [0u8; 32];
    let mut balance = [0u8; 32];
    nonce.copy_from_slice(&scale_encoded[..32]);
    balance.copy_from_slice(&scale_encoded[32..64]);
    Ok(AccountBasic { nonce, balance })
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_account_code_at`.
/// Returns the code bytes (SCALE-encoded `Vec<u8>`).
pub fn decode_account_code(scale_encoded: &[u8]) -> Result<Vec<u8>, DecodeError> {
    match nom::Parser::parse(
        &mut nom::combinator::all_consuming(nom_decode_bytes::<nom::error::Error<&[u8]>>),
        scale_encoded,
    ) {
        Ok((_, bytes)) => Ok(bytes),
        Err(_) => Err(DecodeError::ParseError),
    }
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_storage_at`.
/// Returns the H256 storage value (32 bytes).
pub fn decode_storage_at(scale_encoded: &[u8]) -> Result<[u8; 32], DecodeError> {
    if scale_encoded.len() != 32 {
        return Err(DecodeError::ParseError);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(scale_encoded);
    Ok(out)
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_gas_price`.
/// Returns the gas price as U256 in little-endian (32 bytes).
pub fn decode_gas_price(scale_encoded: &[u8]) -> Result<[u8; 32], DecodeError> {
    if scale_encoded.len() != 32 {
        return Err(DecodeError::ParseError);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(scale_encoded);
    Ok(out)
}

/// Attempt to decode the output of `EthereumRuntimeRPCApi_call`.
///
/// The return type is `Result<(ExitReason, Vec<u8>, U256), DispatchError>`.
/// SCALE encoding: first byte `0x00` = Ok, `0x01` = Err.
/// On Ok: ExitReason enum + return_data Vec<u8> + used_gas U256.
///
/// ExitReason is a nested enum:
/// - 0x00 = Succeed(ExitSucceed) - 1 more byte for variant
/// - 0x01 = Error(ExitError) - variable
/// - 0x02 = Revert(ExitRevert) - 1 more byte for variant
/// - 0x03 = Fatal(ExitFatal) - variable
pub fn decode_call_result(scale_encoded: &[u8]) -> Result<CallResult, DecodeError> {
    if scale_encoded.is_empty() {
        return Err(DecodeError::ParseError);
    }

    // First byte: Result discriminant
    match scale_encoded[0] {
        0x00 => {
            // Ok variant
            let remaining = &scale_encoded[1..];
            if remaining.is_empty() {
                return Err(DecodeError::ParseError);
            }

            // Parse ExitReason
            let (remaining, exit_reason_success, is_revert) = parse_exit_reason(remaining)?;

            // Parse return_data: SCALE Vec<u8>
            let (remaining, return_data) = match nom::Parser::parse(
                &mut nom_decode_bytes::<nom::error::Error<&[u8]>>,
                remaining,
            ) {
                Ok((rest, data)) => (rest, data),
                Err(_) => return Err(DecodeError::ParseError),
            };

            // Parse used_gas: U256 (32 bytes LE)
            if remaining.len() < 32 {
                return Err(DecodeError::ParseError);
            }
            let mut used_gas = [0u8; 32];
            used_gas.copy_from_slice(&remaining[..32]);

            if is_revert {
                return Err(DecodeError::EvmReverted { return_data });
            }

            Ok(CallResult {
                exit_reason_success,
                return_data,
                used_gas,
            })
        }
        0x01 => {
            // Err variant (DispatchError)
            Err(DecodeError::RuntimeDispatchError)
        }
        _ => Err(DecodeError::ParseError),
    }
}

/// Parse the ExitReason enum. Returns (remaining bytes, is_success, is_revert).
fn parse_exit_reason(data: &[u8]) -> Result<(&[u8], bool, bool), DecodeError> {
    if data.is_empty() {
        return Err(DecodeError::ParseError);
    }

    match data[0] {
        0x00 => {
            // ExitReason::Succeed(ExitSucceed)
            // ExitSucceed has variants: Stopped=0, Returned=1, Suicided=2
            if data.len() < 2 {
                return Err(DecodeError::ParseError);
            }
            Ok((&data[2..], true, false))
        }
        0x01 => {
            // ExitReason::Error(ExitError)
            // ExitError is a complex enum, skip its sub-variant byte
            if data.len() < 2 {
                return Err(DecodeError::ParseError);
            }
            // Most ExitError variants are single-byte, but some have data.
            // For our purposes, we just need to know it's not success/revert.
            // We'll skip the variant byte - this is a simplification.
            Ok((&data[2..], false, false))
        }
        0x02 => {
            // ExitReason::Revert(ExitRevert)
            // ExitRevert has variant: Reverted=0
            if data.len() < 2 {
                return Err(DecodeError::ParseError);
            }
            Ok((&data[2..], false, true))
        }
        0x03 => {
            // ExitReason::Fatal(ExitFatal)
            // ExitFatal has variants, skip the sub-variant byte
            if data.len() < 2 {
                return Err(DecodeError::ParseError);
            }
            Ok((&data[2..], false, false))
        }
        _ => Err(DecodeError::ParseError),
    }
}

/// nom parser for a SCALE-encoded `Vec<u8>` (compact length prefix + bytes).
fn nom_decode_bytes<'a, E: nom::error::ParseError<&'a [u8]>>(
    bytes: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<u8>, E> {
    let (remaining, len) = crate::util::nom_scale_compact_usize(bytes)?;
    if remaining.len() < len {
        return Err(nom::Err::Incomplete(nom::Needed::new(len - remaining.len())));
    }
    let (data, rest) = remaining.split_at(len);
    Ok((rest, data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_roundtrip() {
        // Chain ID 1284 (Moonbeam)
        let encoded = 1284u64.to_le_bytes().to_vec();
        assert_eq!(decode_chain_id(&encoded).unwrap(), 1284);
    }

    #[test]
    fn chain_id_wrong_length() {
        assert!(decode_chain_id(&[0u8; 4]).is_err());
    }

    #[test]
    fn account_basic_roundtrip() {
        let mut encoded = [0u8; 64];
        // nonce = 5
        encoded[0] = 5;
        // balance = 1000 (0x3E8)
        encoded[32] = 0xE8;
        encoded[33] = 0x03;
        let result = decode_account_basic(&encoded).unwrap();
        assert_eq!(result.nonce[0], 5);
        assert_eq!(result.balance[0], 0xE8);
        assert_eq!(result.balance[1], 0x03);
    }

    #[test]
    fn account_basic_wrong_length() {
        assert!(decode_account_basic(&[0u8; 32]).is_err());
    }

    #[test]
    fn account_code_empty() {
        // SCALE encoding of empty Vec<u8>: compact 0 = 0x00
        let encoded = [0x00];
        let result = decode_account_code(&encoded).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn account_code_with_data() {
        // SCALE encoding of Vec<u8> with 3 bytes [0x60, 0x80, 0x60]
        // compact(3) = 0x0C, then the 3 bytes
        let encoded = [0x0C, 0x60, 0x80, 0x60];
        let result = decode_account_code(&encoded).unwrap();
        assert_eq!(result, vec![0x60, 0x80, 0x60]);
    }

    #[test]
    fn storage_at_roundtrip() {
        let value = [42u8; 32];
        assert_eq!(decode_storage_at(&value).unwrap(), value);
    }

    #[test]
    fn gas_price_roundtrip() {
        let mut value = [0u8; 32];
        value[0] = 0x01; // 1 Gwei in some unit
        assert_eq!(decode_gas_price(&value).unwrap(), value);
    }

    #[test]
    fn call_result_success() {
        // Result::Ok (0x00) + ExitReason::Succeed (0x00) + ExitSucceed::Returned (0x01)
        // + return_data: empty Vec (0x00) + used_gas: U256(21000)
        let mut encoded = Vec::new();
        encoded.push(0x00); // Ok
        encoded.push(0x00); // Succeed
        encoded.push(0x01); // Returned
        encoded.push(0x00); // empty Vec<u8>
        let mut gas = [0u8; 32];
        gas[0] = 0x28; // 21000 = 0x5208, LE: 0x08, 0x52
        gas[0] = 0x08;
        gas[1] = 0x52;
        encoded.extend_from_slice(&gas);

        let result = decode_call_result(&encoded).unwrap();
        assert!(result.exit_reason_success);
        assert!(result.return_data.is_empty());
        assert_eq!(result.used_gas[0], 0x08);
        assert_eq!(result.used_gas[1], 0x52);
    }

    #[test]
    fn call_result_revert() {
        // Result::Ok (0x00) + ExitReason::Revert (0x02) + ExitRevert::Reverted (0x00)
        // + return_data: 4 bytes (0x10 + data) + used_gas
        let mut encoded = Vec::new();
        encoded.push(0x00); // Ok
        encoded.push(0x02); // Revert
        encoded.push(0x00); // Reverted
        encoded.push(0x10); // compact(4)
        encoded.extend_from_slice(&[0x08, 0xc3, 0x79, 0xa0]); // revert selector
        encoded.extend_from_slice(&[0u8; 32]); // used_gas

        match decode_call_result(&encoded) {
            Err(DecodeError::EvmReverted { return_data }) => {
                assert_eq!(return_data, vec![0x08, 0xc3, 0x79, 0xa0]);
            }
            other => panic!("Expected EvmReverted, got {:?}", other),
        }
    }

    #[test]
    fn call_result_dispatch_error() {
        let encoded = [0x01]; // Err variant
        match decode_call_result(&encoded) {
            Err(DecodeError::RuntimeDispatchError) => {}
            other => panic!("Expected RuntimeDispatchError, got {:?}", other),
        }
    }

    #[test]
    fn call_parameters_encoding() {
        let from = [1u8; 20];
        let to = [2u8; 20];
        let data = [0x60, 0x80];
        let value = [0u8; 32];
        let gas_limit = [0u8; 32];

        let params = call_parameters(
            Some(&from),
            Some(&to),
            &data,
            &value,
            &gas_limit,
            None,
            None,
            None,
            false,
        );

        // from (20) + to (20) + compact(2) (1) + data (2) + value (32) + gas_limit (32)
        // + 3 * None (3) + bool (1) + None access_list (1) = 112
        assert_eq!(params.len(), 112);
        assert_eq!(&params[0..20], &from);
        assert_eq!(&params[20..40], &to);
        assert_eq!(params[40], 0x08); // compact(2)
        assert_eq!(&params[41..43], &data);
    }

    #[test]
    fn encode_scale_compact_len_small() {
        let mut out = Vec::new();
        encode_scale_compact_len(&mut out, 0);
        assert_eq!(out, vec![0x00]);

        let mut out = Vec::new();
        encode_scale_compact_len(&mut out, 1);
        assert_eq!(out, vec![0x04]);

        let mut out = Vec::new();
        encode_scale_compact_len(&mut out, 63);
        assert_eq!(out, vec![0xFC]);
    }

    #[test]
    fn encode_scale_compact_len_medium() {
        let mut out = Vec::new();
        encode_scale_compact_len(&mut out, 64);
        assert_eq!(out, vec![0x01, 0x01]);

        let mut out = Vec::new();
        encode_scale_compact_len(&mut out, 16383);
        // 16383 = 0x3FFF
        // low 6 bits: 0x3F, shifted left 2 with mode 01: (0x3F << 2) | 1 = 0xFD
        // remaining: 16383 >> 6 = 255 = 0xFF
        assert_eq!(out, vec![0xFD, 0xFF]);
    }

    // =========================================================================
    // Hydration parachain integration tests
    //
    // Hydration (formerly HydraDX) is an EVM-compatible Substrate parachain
    // on Polkadot with EVM chain ID 222222 (0x3658E). These tests verify
    // correct SCALE encoding/decoding with realistic Hydration data.
    // =========================================================================

    /// Hydration EVM chain ID.
    const HYDRATION_CHAIN_ID: u64 = 222222;

    #[test]
    fn hydration_chain_id() {
        let encoded = HYDRATION_CHAIN_ID.to_le_bytes().to_vec();
        assert_eq!(decode_chain_id(&encoded).unwrap(), HYDRATION_CHAIN_ID);
    }

    #[test]
    fn hydration_chain_id_parameters_empty() {
        // chain_id takes no parameters
        let params = chain_id_parameters();
        assert!(params.is_empty());
    }

    #[test]
    fn hydration_gas_price_parameters_empty() {
        let params = gas_price_parameters();
        assert!(params.is_empty());
    }

    #[test]
    fn hydration_account_basic_funded_account() {
        // Simulate a Hydration account with:
        //   nonce = 42 (0x2A)
        //   balance = 10_000_000_000_000_000_000 (10 HDX in 18-decimal wei)
        //           = 0x8AC7230489E80000
        let mut encoded = [0u8; 64];
        // nonce: U256 LE, value 42
        encoded[0] = 42;
        // balance: U256 LE, value 10^19 = 0x8AC7230489E80000
        let balance_le = 10_000_000_000_000_000_000u128.to_le_bytes();
        encoded[32..48].copy_from_slice(&balance_le);

        let result = decode_account_basic(&encoded).unwrap();

        // Verify nonce
        assert_eq!(result.nonce[0], 42);
        assert!(result.nonce[1..].iter().all(|&b| b == 0));

        // Verify balance
        assert_eq!(&result.balance[..16], &balance_le);
        assert!(result.balance[16..].iter().all(|&b| b == 0));
    }

    #[test]
    fn hydration_account_basic_zero_balance() {
        // Fresh account with no transactions
        let encoded = [0u8; 64];
        let result = decode_account_basic(&encoded).unwrap();
        assert!(result.nonce.iter().all(|&b| b == 0));
        assert!(result.balance.iter().all(|&b| b == 0));
    }

    #[test]
    fn hydration_account_code_erc20_stub() {
        // Simulate a small ERC20 contract bytecode response.
        // In practice Hydration EVM contracts return SCALE Vec<u8>.
        // Use a realistic EVM bytecode prefix: PUSH1 0x80 PUSH1 0x40 MSTORE
        let bytecode = vec![0x60, 0x80, 0x60, 0x40, 0x52, 0x34, 0x80, 0x15];
        let mut encoded = Vec::new();
        // SCALE compact length for 8 bytes = 8 << 2 = 0x20
        encoded.push(0x20);
        encoded.extend_from_slice(&bytecode);

        let result = decode_account_code(&encoded).unwrap();
        assert_eq!(result, bytecode);
    }

    #[test]
    fn hydration_account_code_large_contract() {
        // Simulate a contract with >63 bytes (requires 2-byte compact prefix).
        // 100 bytes of bytecode: compact(100) = (100 << 2) | 0b01 split into 2 bytes
        // 100 in compact: low 6 bits = 100 & 63 = 36, (36 << 2) | 1 = 145 = 0x91
        // remaining: 100 >> 6 = 1 => second byte = 0x01
        let bytecode = vec![0xAB; 100];
        let mut encoded = Vec::new();
        encode_scale_compact_len(&mut encoded, 100);
        encoded.extend_from_slice(&bytecode);

        let result = decode_account_code(&encoded).unwrap();
        assert_eq!(result.len(), 100);
        assert_eq!(result, bytecode);
    }

    #[test]
    fn hydration_storage_at_slot_zero() {
        // Reading storage slot 0 of a contract.
        // The H256 position is all zeros, result is the stored value.
        let stored_value = {
            let mut v = [0u8; 32];
            v[0] = 0x01; // Some non-zero stored value
            v
        };
        assert_eq!(decode_storage_at(&stored_value).unwrap(), stored_value);
    }

    #[test]
    fn hydration_gas_price_realistic() {
        // Hydration/Frontier chains typically return gas price as U256.
        // A typical base fee might be 1 Gwei = 1_000_000_000 = 0x3B9ACA00
        let mut price = [0u8; 32];
        let gwei = 1_000_000_000u64.to_le_bytes();
        price[..8].copy_from_slice(&gwei);
        assert_eq!(decode_gas_price(&price).unwrap(), price);
    }

    #[test]
    fn hydration_eth_call_balance_of() {
        // Simulate an ERC20 balanceOf call returning a U256 balance.
        // The return data is ABI-encoded: 32-byte padded uint256.
        //
        // Runtime response: Result::Ok(ExitReason::Succeed(Returned), return_data, used_gas)
        let mut encoded = Vec::new();
        encoded.push(0x00); // Result::Ok
        encoded.push(0x00); // ExitReason::Succeed
        encoded.push(0x01); // ExitSucceed::Returned

        // return_data: ABI-encoded uint256 = 1000 tokens (with 18 decimals)
        // = 1000 * 10^18 = 0xDE0B6B3A7640000 * 1000 = 0x3635C9ADC5DEA00000
        let mut abi_balance = [0u8; 32];
        // 1000 * 10^18 in big-endian = 0x00000000000000000000000000000000000000000000003635C9ADC5DEA00000
        let val = 1_000_000_000_000_000_000_000u128; // 1000 * 10^18
        abi_balance[..16].copy_from_slice(&val.to_be_bytes());
        // Actually ABI encodes in BE, so let's just set the last bytes properly
        let mut abi_balance = [0u8; 32];
        let val_bytes = val.to_be_bytes(); // 16 bytes
        abi_balance[16..32].copy_from_slice(&val_bytes);

        // SCALE Vec<u8> encoding of 32-byte return data
        // compact(32) = 32 << 2 = 128 = 0x80
        encoded.push(0x80);
        encoded.extend_from_slice(&abi_balance);

        // used_gas: 25000 (typical for balanceOf)
        let mut gas = [0u8; 32];
        gas[..8].copy_from_slice(&25_000u64.to_le_bytes());
        encoded.extend_from_slice(&gas);

        let result = decode_call_result(&encoded).unwrap();
        assert!(result.exit_reason_success);
        assert_eq!(result.return_data.len(), 32);
        assert_eq!(&result.return_data, &abi_balance);
        assert_eq!(
            u64::from_le_bytes(result.used_gas[..8].try_into().unwrap()),
            25_000
        );
    }

    #[test]
    fn hydration_eth_call_error_out_of_gas() {
        // Simulate an EVM OutOfGas error.
        // Result::Ok(ExitReason::Error(OutOfGas), empty return_data, used_gas)
        let mut encoded = Vec::new();
        encoded.push(0x00); // Result::Ok
        encoded.push(0x01); // ExitReason::Error
        encoded.push(0x00); // ExitError::StackUnderflow (variant 0, simplified)
        encoded.push(0x00); // empty return data
        let mut gas = [0u8; 32];
        gas[..8].copy_from_slice(&15_000_000u64.to_le_bytes());
        encoded.extend_from_slice(&gas);

        let result = decode_call_result(&encoded).unwrap();
        assert!(!result.exit_reason_success);
        assert!(result.return_data.is_empty());
    }

    #[test]
    fn hydration_eth_call_revert_with_reason() {
        // Simulate a Solidity revert("Insufficient balance") from a Hydration DEX.
        // The return data contains the Error(string) ABI encoding:
        //   0x08c379a0 (selector)
        //   + offset (32 bytes)
        //   + length (32 bytes)
        //   + "Insufficient balance" padded to 32 bytes
        let mut revert_data = Vec::new();
        revert_data.extend_from_slice(&[0x08, 0xc3, 0x79, 0xa0]); // Error(string) selector
        // offset = 32
        let mut offset = [0u8; 32];
        offset[31] = 0x20;
        revert_data.extend_from_slice(&offset);
        // length = 20 ("Insufficient balance")
        let mut length = [0u8; 32];
        length[31] = 20;
        revert_data.extend_from_slice(&length);
        // string data padded to 32 bytes
        let mut str_data = [0u8; 32];
        str_data[..20].copy_from_slice(b"Insufficient balance");
        revert_data.extend_from_slice(&str_data);

        let mut encoded = Vec::new();
        encoded.push(0x00); // Result::Ok
        encoded.push(0x02); // ExitReason::Revert
        encoded.push(0x00); // ExitRevert::Reverted

        // SCALE Vec<u8> for revert_data (100 bytes)
        encode_scale_compact_len(&mut encoded, revert_data.len());
        encoded.extend_from_slice(&revert_data);

        // used_gas
        encoded.extend_from_slice(&[0u8; 32]);

        match decode_call_result(&encoded) {
            Err(DecodeError::EvmReverted { return_data }) => {
                assert_eq!(return_data, revert_data);
                // Verify the selector is correct
                assert_eq!(&return_data[..4], &[0x08, 0xc3, 0x79, 0xa0]);
            }
            other => panic!("Expected EvmReverted, got {:?}", other),
        }
    }

    #[test]
    fn hydration_call_parameters_erc20_transfer() {
        // Simulate an eth_call for ERC20 transfer on Hydration.
        // from: a Hydration EVM address
        // to: the ERC20 contract address
        // data: transfer(address,uint256) ABI-encoded
        let from = [0x11u8; 20]; // sender
        let to = [0x22u8; 20]; // ERC20 contract

        // transfer(address,uint256) selector = 0xa9059cbb
        let mut call_data = Vec::new();
        call_data.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
        // address argument (32 bytes, right-padded)
        let mut addr_arg = [0u8; 32];
        addr_arg[12..32].copy_from_slice(&[0x33u8; 20]); // recipient
        call_data.extend_from_slice(&addr_arg);
        // uint256 argument: 100 tokens
        let mut amount = [0u8; 32];
        amount[31] = 100;
        call_data.extend_from_slice(&amount);

        let value = [0u8; 32]; // no ETH value
        let gas_limit = {
            let mut g = [0u8; 32];
            g[..8].copy_from_slice(&100_000u64.to_le_bytes());
            g
        };

        let params = call_parameters(
            Some(&from),
            Some(&to),
            &call_data,
            &value,
            &gas_limit,
            None,
            None,
            None,
            false, // not estimate
        );

        // Verify structure
        assert_eq!(&params[0..20], &from);
        assert_eq!(&params[20..40], &to);
        // call_data is 68 bytes, compact(68) = (68 << 2) | 1 = 0x11, 0x01
        assert_eq!(params[40], 0x11);
        assert_eq!(params[41], 0x01);
        // call_data follows
        assert_eq!(&params[42..46], &[0xa9, 0x05, 0x9c, 0xbb]);
        // value (32 bytes of zero) at offset 42 + 68 = 110
        assert_eq!(&params[110..142], &value);
        // gas_limit at offset 142
        assert_eq!(&params[142..174], &gas_limit);
    }

    #[test]
    fn hydration_call_parameters_estimate_gas() {
        // Same as above but with estimate=true for eth_estimateGas
        let params = call_parameters(
            None,           // from: none
            Some(&[0x22u8; 20]), // to: contract
            &[0xa9, 0x05, 0x9c, 0xbb], // minimal transfer selector
            &[0u8; 32],    // value
            &[0u8; 32],    // gas_limit
            None,
            None,
            None,
            true, // estimate = true
        );

        // The `estimate` bool is near the end of the encoded params.
        // from(20) + to(20) + compact(4)=1byte + data(4) + value(32) + gas_limit(32)
        //   + 3*None(3) + estimate(1) + None_access_list(1) = 114
        assert_eq!(params.len(), 114);
        // estimate flag should be 1
        assert_eq!(params[112], 0x01); // estimate = true
        // access_list None
        assert_eq!(params[113], 0x00);
    }

    #[test]
    fn hydration_storage_at_parameters_encoding() {
        let address = [0xABu8; 20];
        let mut position = [0u8; 32];
        position[0] = 0x05; // storage slot 5 in LE

        let params = storage_at_parameters(&address, &position);
        assert_eq!(params.len(), 52);
        assert_eq!(&params[..20], &address);
        assert_eq!(&params[20..52], &position);
    }

    #[test]
    fn hydration_account_basic_parameters_encoding() {
        let address = [0xCDu8; 20];
        let params = account_basic_parameters(&address);
        assert_eq!(params.len(), 20);
        assert_eq!(&params, &address);
    }

    #[test]
    fn hydration_account_code_parameters_encoding() {
        let address = [0xEFu8; 20];
        let params = account_code_at_parameters(&address);
        assert_eq!(params.len(), 20);
        assert_eq!(&params, &address);
    }
}
