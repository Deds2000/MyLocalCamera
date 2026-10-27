//! Protocolo LocalCamera (implementação nos próximos cartões).

#[cfg(test)]
mod infra_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_infra_works(a in any::<u32>(), b in any::<u32>()) {
            prop_assert_eq!(a as u64 + b as u64, b as u64 + a as u64);
        }
    }
}
