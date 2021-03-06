use alloc::{collections::BTreeMap, vec::Vec};

use serde::{Deserialize, Serialize};

use crate::{
    auction::{types::DelegationRate, Delegator},
    bytesrepr::{self, FromBytes, ToBytes},
    system_contract_errors::auction::Error,
    CLType, CLTyped, PublicKey, URef, U512,
};

/// An entry in a founding validator map.
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Bid {
    /// The purse that was used for bonding.
    bonding_purse: URef,
    /// The amount of tokens staked by a validator (not including delegators).
    staked_amount: U512,
    /// Delegation rate
    delegation_rate: DelegationRate,
    /// Timestamp (in milliseconds since epoch format) at which a given bid is unlocked.  If
    /// `None`, bid is unlocked.
    release_timestamp_millis: Option<u64>,
    /// This validator's delegators, indexed by their public keys
    delegators: BTreeMap<PublicKey, Delegator>,
    /// This validator's seigniorage reward
    reward: U512,
}

impl Bid {
    /// Creates new instance of a bid with locked funds.
    pub fn locked(bonding_purse: URef, staked_amount: U512, release_timestamp_millis: u64) -> Self {
        let delegation_rate = 0;
        let release_timestamp_millis = Some(release_timestamp_millis);
        let delegators = BTreeMap::new();
        let reward = U512::zero();
        Self {
            bonding_purse,
            staked_amount,
            delegation_rate,
            release_timestamp_millis,
            delegators,
            reward,
        }
    }

    /// Creates new instance of a bid with unlocked funds.
    pub fn unlocked(
        bonding_purse: URef,
        staked_amount: U512,
        delegation_rate: DelegationRate,
    ) -> Self {
        let release_timestamp_millis = None;
        let delegators = BTreeMap::new();
        let reward = U512::zero();
        Self {
            bonding_purse,
            staked_amount,
            delegation_rate,
            release_timestamp_millis,
            delegators,
            reward,
        }
    }

    /// Gets the bonding purse of the provided bid
    pub fn bonding_purse(&self) -> &URef {
        &self.bonding_purse
    }

    /// Gets the staked amount of the provided bid
    pub fn staked_amount(&self) -> &U512 {
        &self.staked_amount
    }

    /// Gets the delegation rate of the provided bid
    pub fn delegation_rate(&self) -> &DelegationRate {
        &self.delegation_rate
    }

    /// Returns `true` if the provided bid is locked.
    pub fn is_locked(&self) -> bool {
        self.release_timestamp_millis.is_some()
    }

    /// Returns the timestamp (in milliseconds since epoch format) at which a given bid is unlocked.
    /// If `None`, bid is unlocked.
    pub fn release_timestamp_millis(&self) -> Option<u64> {
        self.release_timestamp_millis
    }

    /// Returns a reference to the delegators of the provided bid
    pub fn delegators(&self) -> &BTreeMap<PublicKey, Delegator> {
        &self.delegators
    }

    /// Returns a mutable reference to the delegators of the provided bid
    pub fn delegators_mut(&mut self) -> &mut BTreeMap<PublicKey, Delegator> {
        &mut self.delegators
    }

    /// Returns the seigniorage reward of the provided bid
    pub fn reward(&self) -> &U512 {
        &self.reward
    }

    /// Decreases the stake of the provided bid
    pub fn decrease_stake(&mut self, amount: U512) -> Result<U512, Error> {
        if self.is_locked() {
            return Err(Error::ValidatorFundsLocked);
        }

        let updated_staked_amount = self
            .staked_amount
            .checked_sub(amount)
            .ok_or(Error::InvalidAmount)?;

        self.staked_amount = updated_staked_amount;

        Ok(updated_staked_amount)
    }

    /// Increases the stake of the provided bid
    pub fn increase_stake(&mut self, amount: U512) -> Result<U512, Error> {
        let updated_staked_amount = self
            .staked_amount
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        self.staked_amount = updated_staked_amount;

        Ok(updated_staked_amount)
    }

    /// Increases the seigniorage reward of the provided bid
    pub fn increase_reward(&mut self, amount: U512) -> Result<U512, Error> {
        let updated_reward = self
            .reward
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        self.reward = updated_reward;

        Ok(updated_reward)
    }

    /// Zeros the seigniorage reward of the provided bid
    pub fn zero_reward(&mut self) {
        self.reward = U512::zero()
    }

    /// Updates the delegation rate of the provided bid
    pub fn with_delegation_rate(&mut self, delegation_rate: DelegationRate) -> &mut Self {
        self.delegation_rate = delegation_rate;
        self
    }

    /// Unlocks the provided bid if the provided timestamp is greater than or equal to the bid's
    /// release timestamp.
    ///
    /// Returns `true` if the provided bid was unlocked.
    pub fn unlock(&mut self, era_end_timestamp_millis: u64) -> bool {
        let release_timestamp_millis = match self.release_timestamp_millis {
            Some(release_timestamp_millis) => release_timestamp_millis,
            None => return false,
        };
        if era_end_timestamp_millis < release_timestamp_millis {
            return false;
        }
        self.release_timestamp_millis = None;
        true
    }

    /// Returns the total staked amount of validator + all delegators
    pub fn total_staked_amount(&self) -> Result<U512, Error> {
        self.delegators
            .iter()
            .fold(Some(U512::zero()), |maybe_a, (_, b)| {
                maybe_a.and_then(|a| a.checked_add(*b.staked_amount()))
            })
            .and_then(|delegators_sum| delegators_sum.checked_add(*self.staked_amount()))
            .ok_or(Error::InvalidAmount)
    }
}

impl CLTyped for Bid {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for Bid {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut result = bytesrepr::allocate_buffer(self)?;
        result.extend(self.bonding_purse.to_bytes()?);
        result.extend(self.staked_amount.to_bytes()?);
        result.extend(self.delegation_rate.to_bytes()?);
        result.extend(self.release_timestamp_millis.to_bytes()?);
        result.extend(self.delegators.to_bytes()?);
        result.extend(self.reward.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        self.bonding_purse.serialized_length()
            + self.staked_amount.serialized_length()
            + self.delegation_rate.serialized_length()
            + self.release_timestamp_millis.serialized_length()
            + self.delegators.serialized_length()
            + self.reward.serialized_length()
    }
}

impl FromBytes for Bid {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (bonding_purse, bytes) = FromBytes::from_bytes(bytes)?;
        let (staked_amount, bytes) = FromBytes::from_bytes(bytes)?;
        let (delegation_rate, bytes) = FromBytes::from_bytes(bytes)?;
        let (release_timestamp_millis, bytes) = FromBytes::from_bytes(bytes)?;
        let (delegators, bytes) = FromBytes::from_bytes(bytes)?;
        let (reward, bytes) = FromBytes::from_bytes(bytes)?;
        Ok((
            Bid {
                bonding_purse,
                staked_amount,
                delegation_rate,
                release_timestamp_millis,
                delegators,
                reward,
            },
            bytes,
        ))
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use crate::{
        auction::{Bid, DelegationRate},
        bytesrepr, AccessRights, URef, U512,
    };

    #[test]
    fn serialization_roundtrip() {
        let founding_validator = Bid {
            bonding_purse: URef::new([42; 32], AccessRights::READ_ADD_WRITE),
            staked_amount: U512::one(),
            delegation_rate: DelegationRate::max_value(),
            release_timestamp_millis: Some(u64::max_value() - 1),
            delegators: BTreeMap::default(),
            reward: U512::one(),
        };
        bytesrepr::test_serialization_roundtrip(&founding_validator);
    }
}
