use crate::{
    extensions::PSP3156FlashBorrowerStub,
    traits::{
        PSP22Error,
        PSP22,
    },
};
use brush::traits::{
    AccountId,
    Balance,
};
use ink_env::call::FromAccountId;
use ink_prelude::{
    string::String,
    vec::Vec,
};

#[brush::trait_definition]
pub trait PSP22FlashMint: PSP22 {
    const RETURN_VALUE: [u8; 32] = brush::blake2b_256!("PSP3156FlashBorrower.onFlashLoan");

    #[ink(message)]
    fn max_flashloan(&mut self, token: AccountId) -> Balance {
        if token == Self::env().account_id() {
            Balance::MAX - self.total_supply()
        } else {
            0
        }
    }

    #[ink(message)]
    fn flash_fee(&mut self, token: AccountId, amount: Balance) -> Result<Balance, PSP22Error> {
        if token != Self::env().account_id() {
            return Err(PSP22Error::Custom(String::from("Wrong token")))
        }
        Ok(self.get_fee(amount))
    }

    #[ink(message)]
    fn flashloan(
        &mut self,
        receiver_account: AccountId,
        token: AccountId,
        amount: Balance,
        data: Vec<u8>,
    ) -> Result<(), PSP22Error> {
        let fee = self.flash_fee(token, amount)?;
        self._mint(receiver_account, amount)?;
        self.on_flashloan(receiver_account, token, fee, amount, data)?;
        let current_allowance = self.allowance(receiver_account, Self::env().account_id());
        if current_allowance < amount + fee {
            return Err(PSP22Error::Custom(String::from("Allowance does not allow refund")))
        }
        self._approve_from_to(
            receiver_account,
            Self::env().account_id(),
            current_allowance - amount - fee,
        )?;
        self._burn(receiver_account, amount + fee)
    }

    fn get_fee(&mut self, _amount: Balance) -> Balance {
        0
    }

    fn on_flashloan(
        &mut self,
        receiver_account: AccountId,
        token: AccountId,
        fee: Balance,
        amount: Balance,
        data: Vec<u8>,
    ) -> Result<(), PSP22Error> {
        let mut receiver: PSP3156FlashBorrowerStub = FromAccountId::from_account_id(receiver_account);
        if receiver.on_flashloan(Self::env().caller(), token, amount, fee, data) != Self::RETURN_VALUE {
            return Err(PSP22Error::Custom(String::from("Invalid return value")))
        }
        Ok(())
    }
}

#[brush::trait_definition]
pub trait PSP3156FlashBorrower {
    #[ink(message)]
    fn on_flashloan(
        &mut self,
        initiator: AccountId,
        token: AccountId,
        amount: Balance,
        fee: Balance,
        data: Vec<u8>,
    ) -> [u8; 32];
}
