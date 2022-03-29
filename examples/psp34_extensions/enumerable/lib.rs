#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[brush::contract]
pub mod my_psp34_enumerable {
    use brush::contracts::psp34::extensions::{
        burnable::*,
        enumerable::*,
        mintable::*,
    };
    use ink_storage::traits::SpreadAllocate;

    #[derive(Default, SpreadAllocate, PSP34Storage, PSP34EnumerableStorage)]
    #[ink(storage)]
    pub struct MyPSP34 {
        #[PSP34StorageField]
        psp34: PSP34Data,
        #[PSP34EnumerableStorageField]
        metadata: PSP34EnumerableData,
    }

    impl PSP34 for MyPSP34 {}

    impl PSP34Mintable for MyPSP34 {}

    impl PSP34Burnable for MyPSP34 {}

    impl PSP34Enumerable for MyPSP34 {}

    impl PSP34Transfer for MyPSP34 {
        fn _before_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            _id: &Id,
        ) -> Result<(), PSP34Error> {
            self._track_id_transfer(_from, _to, _id)
        }

        fn _after_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            _id: &Id,
        ) -> Result<(), PSP34Error> {
            Ok(())
        }
    }

    impl MyPSP34 {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::codegen::initialize_contract(|_instance: &mut Self| {})
        }
    }
}
