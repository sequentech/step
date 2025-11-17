// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use num_bigint::{BigUint, ToBigUint};
use num_traits::{One, ToPrimitive, Zero};

pub fn encode(values: &Vec<u64>, bases: &Vec<u64>) -> Result<BigUint, String> {
    if bases.len() != values.len() {
        return Err(
            format!("Invalid parameters: 'valueList' (size = {}) and 'baseList' (size = {}) must have the same length.", values.len(), bases.len())
        );
    }
    let mut encoded: BigUint = Zero::zero();
    let mut acc_base: BigUint = One::one();

    for i in 0..bases.len() {
        encoded += &acc_base * values[i].to_biguint().unwrap();
        acc_base = &acc_base * bases[i].to_biguint().unwrap();
    }
    Ok(encoded)
}

pub fn decode(
    bases: &Vec<u64>,
    encoded_value: &BigUint,
    last_base: u64,
) -> Result<Vec<u64>, String> {
    let mut values: Vec<u64> = vec![];
    let mut accumulator: BigUint = encoded_value.clone();
    let mut index = 0usize;

    while accumulator > Zero::zero() {
        let base: BigUint = (if index < bases.len() {
            bases[index]
        } else {
            last_base
        })
        .to_biguint()
        .unwrap();
        let remainder = &accumulator % &base;
        values.push(remainder.to_u64().unwrap());
        accumulator = (&accumulator - &remainder) / &base;
        index += 1;
    }

    // If we didn't run all the bases, fill the rest with zeros
    while index < bases.len() {
        values.push(0);
        index += 1;
    }

    // finish last write-in with a 0
    if (bases.len() < values.len() || bases.last().unwrap_or(&0) == &last_base)
        && values.last().unwrap_or(&0) != &0u64
    {
        values.push(0);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use crate::mixed_radix::*;
    use num_bigint::{BigUint, ToBigUint};
    use std::str::FromStr;

    struct TestData {
        pub value_list: Vec<u64>,
        pub base_list: Vec<u64>,
        pub encoded_value: BigUint,
        pub last_base: u64,
    }

    fn get_fixture1() -> Vec<TestData> {
        vec![
            TestData {
                value_list: vec![29, 23, 59],
                base_list: vec![30, 24, 60],
                encoded_value: 43199.to_biguint().unwrap(), // = (29 + 30*(23 + 24*59))
                last_base: 256u64,
            },
            TestData {
                value_list: vec![29, 23, 59, 2, 100, 0],
                base_list: vec![30, 24, 60],
                encoded_value: 1106049599.to_biguint().unwrap(), /* = 29 + 23*30 + 59*30*24 + 2*30*24*60 + 100*30*24*60*256 */
                last_base: 256u64,
            },
            TestData {
                value_list: vec![10, 10, 10],
                base_list: vec![30, 24, 60],
                encoded_value: 7510.to_biguint().unwrap(), //  = (10 + 30*(10 + 24*10))
                last_base: 256u64,
            },
            TestData {
                value_list: vec![21, 10, 11],
                base_list: vec![30, 24, 60],
                encoded_value: 8241.to_biguint().unwrap(), // = 21 + 30*(10 + 24*11))
                last_base: 256u64,
            },
        ]
    }

    fn get_fixture2() -> Vec<TestData> {
        vec![
            TestData {
                value_list: vec![21, 10, 11],
                base_list: vec![30, 24, 60],
                encoded_value: 8241.to_biguint().unwrap(),
                last_base: 256u64,
            },
            TestData {
                value_list: vec![3, 2, 1],
                base_list: vec![5, 10, 10],
                encoded_value: 63.to_biguint().unwrap(),
                last_base: 256u64,
            },
            TestData {
                value_list: vec![1, 0, 2, 2, 128, 125, 0, 0],
                base_list: vec![3, 3, 3, 3, 256, 256, 256, 256],
                encoded_value: 2602441.to_biguint().unwrap(),
                last_base: 256u64,
            },
            TestData {
                value_list: vec![0, 1, 2, 0],
                base_list: vec![3, 3, 3, 3],
                encoded_value: 21.to_biguint().unwrap(),
                last_base: 256u64,
            },
            TestData {
                value_list: vec![1, 0, 0, 0, 0, 0, 0],
                base_list: vec![2, 2, 256, 256, 256, 256, 256],
                encoded_value: 1.to_biguint().unwrap(),
                last_base: 256u64,
            },
            TestData {
                value_list: vec![0, 1, 0, 0, 1, 0, 1, 69, 0],
                base_list: vec![2, 2, 2, 2, 2, 2, 2, 256, 256],
                encoded_value: 8914.to_biguint().unwrap(), // (0 + 2*(1 + 2*(0 + 2*(0 + 2*(1
                // + 2*(0+ 2*(1 + 2*(69))))))))
                last_base: 256u64,
            },
            TestData {
                value_list: vec![
                    0, 1, 0, 0, 1, 0, 1, 69, 0, 0, 195, 132, 32, 98, 99, 0,
                ],
                base_list: vec![
                    2, 2, 2, 2, 2, 2, 2, 256, 256, 256, 256, 256, 256, 256,
                    256, 256,
                ],
                // Value calculated in python3 that uses by default big ints for
                // integers. The formula is:
                // (0 + 2*(1 + 2*(0 + 2*(0 + 2*(1 + 2*(0+ 2*(1 + 2*(69 + 256*(0
                // + 256*(0 + 256*(195 + 256*(132 + 256*(32 +
                // 256*(98+ 256*99))))))))))))))
                encoded_value: BigUint::from_str("916649230342635397842")
                    .unwrap(),
                last_base: 256u64,
            },
        ]
    }

    #[test]
    fn test_encode_fixture1() {
        let fixtures: Vec<TestData> = get_fixture1();

        for fixture in fixtures.into_iter() {
            let mut base_list: Vec<u64> = fixture.base_list.clone();
            while base_list.len() < fixture.value_list.len() {
                base_list.push(fixture.last_base);
            }
            let encoded_value =
                encode(&fixture.value_list, &base_list).unwrap();

            assert_eq!(encoded_value, fixture.encoded_value);
        }
    }

    #[test]
    fn test_decode_fixture1() {
        let fixtures: Vec<TestData> = get_fixture1();

        for fixture in fixtures.into_iter() {
            let decoded_value = decode(
                &fixture.base_list,
                &fixture.encoded_value,
                fixture.last_base,
            )
            .unwrap();

            assert_eq!(decoded_value, fixture.value_list);
        }
    }

    #[test]
    fn test_encode_then_decode_fixture2() {
        let fixtures: Vec<TestData> = get_fixture2();

        for fixture in fixtures.into_iter() {
            let encoded_value =
                encode(&fixture.value_list, &fixture.base_list).unwrap();
            let decoded_value =
                decode(&fixture.base_list, &encoded_value, fixture.last_base)
                    .unwrap();

            assert_eq!(encoded_value, fixture.encoded_value);
            assert_eq!(decoded_value, fixture.value_list);
        }
    }

    /**
     * Test to Ensure encode function raises exception when value_list and
     * base_list don't have the same length.
     */
    #[test]
    fn test_encode_error() {
        let encoded_result1 = encode(&vec![1, 2], &vec![5, 5, 5]);
        assert!(encoded_result1.is_err());

        let encoded_result2 = encode(&vec![1, 2, 3, 3], &vec![6, 6, 6]);
        assert!(encoded_result2.is_err());
    }
}
