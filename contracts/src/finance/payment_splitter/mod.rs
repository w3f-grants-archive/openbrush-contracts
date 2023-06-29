// Copyright (c) 2012-2022 Supercolony
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the"Software"),
// to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

pub use crate::{
    payment_splitter,
    traits::payment_splitter::*,
};
use ink::{
    prelude::vec::Vec,
    storage::{
        traits::ManualKey,
        Lazy,
    },
};
use openbrush::{
    storage::Mapping,
    traits::{
        AccountId,
        Balance,
        Storage,
    },
};
pub use payment_splitter::Internal as _;

pub const STORAGE_KEY_1: u32 = openbrush::storage_unique_key2!("payment_splitter::total_shares");
pub const STORAGE_KEY_2: u32 = openbrush::storage_unique_key2!("payment_splitter::total_released");
pub const STORAGE_KEY_3: u32 = openbrush::storage_unique_key2!("payment_splitter::shares");
pub const STORAGE_KEY_4: u32 = openbrush::storage_unique_key2!("payment_splitter::released");
pub const STORAGE_KEY_5: u32 = openbrush::storage_unique_key2!("payment_splitter::payees");

#[derive(Default, Debug)]
#[ink::storage_item]
pub struct Data {
    pub total_shares: Lazy<Balance, ManualKey<STORAGE_KEY_1>>,
    pub total_released: Lazy<Balance, ManualKey<STORAGE_KEY_2>>,
    pub shares: Mapping<AccountId, Balance, ManualKey<STORAGE_KEY_3>>,
    pub released: Mapping<AccountId, Balance, ManualKey<STORAGE_KEY_4>>,
    pub payees: Lazy<Vec<AccountId>, ManualKey<STORAGE_KEY_5>>,
}

pub trait PaymentSplitterImpl: Storage<Data> + Internal {
    fn total_shares(&self) -> Balance {
        self.data().total_shares.get_or_default()
    }

    fn total_released(&self) -> Balance {
        self.data().total_released.get_or_default()
    }

    fn shares(&self, account: AccountId) -> Balance {
        self.data().shares.get(&account).unwrap_or(0)
    }

    fn released(&self, account: AccountId) -> Balance {
        self.data().released.get(&account).unwrap_or(0)
    }

    fn payee(&self, index: u32) -> Option<AccountId> {
        self.data().payees.get_or_default().get(index as usize).cloned()
    }

    fn receive(&mut self) {
        self._emit_payee_added_event(Self::env().caller(), Self::env().transferred_value())
    }

    fn release(&mut self, account: AccountId) -> Result<(), PaymentSplitterError> {
        self._release(account)
    }
}

pub trait Internal {
    /// User must override those methods in their contract.
    fn _emit_payee_added_event(&self, account: AccountId, shares: Balance);

    fn _emit_payment_received_event(&self, from: AccountId, amount: Balance);

    fn _emit_payment_released_event(&self, to: AccountId, amount: Balance);

    /// Inits an instance of `PaymentSplitter` where each account in `payees` is assigned the number of shares at
    /// the matching position in the `shares` array.
    ///
    /// All addresses in `payees` must be non-zero. Both arrays must have the same non-zero length, and there must be no
    /// duplicates in `payees`.
    ///
    /// Emits `PayeeAdded` on each account.
    fn _init(&mut self, payees_and_shares: Vec<(AccountId, Balance)>) -> Result<(), PaymentSplitterError>;

    fn _add_payee(&mut self, payee: AccountId, share: Balance) -> Result<(), PaymentSplitterError>;

    /// Calls the `release` method for each `AccountId` in the `payees` vec.
    fn _release_all(&mut self) -> Result<(), PaymentSplitterError>;

    fn _release(&mut self, account: AccountId) -> Result<(), PaymentSplitterError>;
}

pub trait InternalImpl: Storage<Data> + Internal {
    fn _emit_payee_added_event(&self, _account: AccountId, _shares: Balance) {}

    fn _emit_payment_received_event(&self, _from: AccountId, _amount: Balance) {}

    fn _emit_payment_released_event(&self, _to: AccountId, _amount: Balance) {}

    fn _init(&mut self, payees_and_shares: Vec<(AccountId, Balance)>) -> Result<(), PaymentSplitterError> {
        if payees_and_shares.is_empty() {
            return Err(PaymentSplitterError::NoPayees)
        }

        for (payee, share) in payees_and_shares.into_iter() {
            Internal::_add_payee(self, payee, share)?;
        }
        Ok(())
    }

    fn _add_payee(&mut self, payee: AccountId, share: Balance) -> Result<(), PaymentSplitterError> {
        if share == 0 {
            return Err(PaymentSplitterError::SharesAreZero)
        }
        if self.data().shares.get(&payee).is_some() {
            return Err(PaymentSplitterError::AlreadyHasShares)
        }

        let mut payees = self.data().payees.get_or_default();
        payees.push(payee.clone());
        self.data().payees.set(&payees);

        self.data().shares.insert(&payee, &share);

        let new_shares = self.data().total_shares.get_or_default() + share;
        self.data().total_shares.set(&new_shares);

        Internal::_emit_payee_added_event(self, payee, share);
        Ok(())
    }

    fn _release_all(&mut self) -> Result<(), PaymentSplitterError> {
        let payees = self.data().payees.get_or_default();
        let len = payees.len();

        for i in 0..len {
            let account = payees[i];
            Internal::_release(self, account)?;
        }

        Ok(())
    }

    fn _release(&mut self, account: AccountId) -> Result<(), PaymentSplitterError> {
        if !self.data().shares.get(&account).is_some() {
            return Err(PaymentSplitterError::AccountHasNoShares)
        }

        let balance = Self::env().balance();
        let current_balance = balance.checked_sub(Self::env().minimum_balance()).unwrap_or_default();
        let total_released = self.data().total_released.get_or_default();
        let total_received = current_balance + total_released;
        let shares = self.data().shares.get(&account).unwrap().clone();
        let total_shares = self.data().total_shares.get_or_default();
        let released = self.data().released.get(&account).unwrap_or_default();
        let payment = total_received * shares / total_shares - released;

        if payment == 0 {
            return Err(PaymentSplitterError::AccountIsNotDuePayment)
        }

        self.data().released.insert(&account, &(released + payment));
        self.data().total_released.set(&(total_released + payment));

        let transfer_result = Self::env().transfer(account.clone(), payment);
        if transfer_result.is_err() {
            return Err(PaymentSplitterError::TransferFailed)
        }
        Internal::_emit_payment_released_event(self, account, payment);
        Ok(())
    }
}
