use alloy_primitives::{map::HashMap, Address, U256};
use reth_chainspec::EthereumHardforks;
use reth_consensus_common::calc;
use reth_primitives::{Block, Withdrawal, Withdrawals};
use reth_execution_errors::{BlockExecutionError, BlockValidationError};
use reth_primitives::{
    revm::env::fill_tx_env_with_beacon_root_contract_call, Address, ChainSpec, Header,
    ShadowReceipt, ShadowResult, Withdrawal, B256, U256,
};
use revm::{
    interpreter::Host,
    primitives::{
        commitment::{CommitmentStatus, IrysTxId, Stake},
        shadow::{ShadowTx, ShadowTxType, Shadows, TransferShadow},
        AccountInfo, EVMError, LastTx, ShadowTxTypeId,
    },
    Database, DatabaseCommit, Evm, JournalEntry, JournaledState,
};
use std::collections::HashMap;
use tracing::{info, trace};

/// Collect all balance changes at the end of the block.
///
/// Balance changes might include the block reward, uncle rewards, withdrawals, or irregular
/// state changes (DAO fork).
#[inline]
pub fn post_block_balance_increments<ChainSpec: EthereumHardforks>(
    chain_spec: &ChainSpec,
    block: &Block,
    total_difficulty: U256,
) -> HashMap<Address, u128> {
    let mut balance_increments = HashMap::default();

    // Add block rewards if they are enabled.
    if let Some(base_block_reward) =
        calc::base_block_reward(chain_spec, block.number, block.difficulty, total_difficulty)
    {
        // Ommer rewards
        for ommer in &block.body.ommers {
            *balance_increments.entry(ommer.beneficiary).or_default() +=
                calc::ommer_reward(base_block_reward, block.number, ommer.number);
        }

        // Full block reward
        *balance_increments.entry(block.beneficiary).or_default() +=
            calc::block_reward(base_block_reward, block.body.ommers.len());
    }

    // process withdrawals
    insert_post_block_withdrawals_balance_increments(
        chain_spec,
        block.timestamp,
        block.body.withdrawals.as_ref().map(Withdrawals::as_ref),
        &mut balance_increments,
    );

    balance_increments
}


/ Applies the pre-block Irys transaction shadows, using the given block,
/// [ChainSpec], EVM.
///

#[inline]
pub fn apply_block_shadows<EXT, DB: Database + DatabaseCommit>(
    // _chain_spec: &ChainSpec,
    // block: &BlockWithSenders,
    shadows: Option<&Shadows>,
    evm: &mut Evm<'_, EXT, DB>,
) -> Result<Option<Vec<ShadowReceipt>>, EVMError<DB::Error>>
where
    DB::Error: std::fmt::Display + std::fmt::Debug,
{
    // skip if the are no shadows
    let Some(shadows) = shadows else { return Ok(None) };
    let mut receipts = Vec::with_capacity(shadows.len());

    // TODO: fix this clone
    for shadow in shadows.clone().into_iter() {
        let checkpoint = evm.context.evm.inner.journaled_state.checkpoint();

        // match apply_shadow(shadow, evm) {
        match apply_shadow(
            shadow.clone(),
            &mut evm.context.evm.inner.journaled_state,
            &mut evm.context.evm.inner.db,
        ) {
            Ok(res) => {
                trace!(target: "shadows", tx_id = ?shadow.tx_id, "shadow execution successful");
                match res.result {
                    ShadowResult::Success => {
                        trace!(target: "shadows", tx_id = ?shadow.tx_id, "shadow execution successful")
                    }
                    _ => {
                        trace!(target: "shadows", tx_id = ?shadow.tx_id, "shadow execution succeeded, with error {:?}", res.result )
                    }
                }
                evm.context.evm.inner.journaled_state.checkpoint_commit();
                receipts.push(res);
            }
            Err(e) => {
                trace!(target: "shadows", tx_id = ?shadow.tx_id, "shadow execution errored");

                evm.context.evm.inner.journaled_state.checkpoint_revert(checkpoint);
                return Err(e);
            }
        }
    }

    Ok(Some(receipts))
}

pub fn simulate_apply_shadow<EXT, DB: Database + DatabaseCommit>(
    shadow: ShadowTx,
    evm: &mut Evm<'_, EXT, DB>,
) -> Result<ShadowReceipt, EVMError<DB::Error>> {
    // create a checkpoint, try to apply the shadow, and always revert
    let checkpoint = evm.context.evm.inner.journaled_state.checkpoint();
    // let result = apply_shadow(shadow, evm);
    let result = apply_shadow(
        shadow,
        &mut evm.context.evm.inner.journaled_state,
        &mut evm.context.evm.inner.db,
    );
    evm.context.evm.inner.journaled_state.checkpoint_revert(checkpoint);
    return result;
}

pub fn simulate_apply_shadow_thin<DB: Database + DatabaseCommit>(
    shadow: ShadowTx,
    journaled_state: &mut JournaledState,
    db: &mut DB,
) -> Result<ShadowReceipt, EVMError<DB::Error>> {
    // create a checkpoint, try to apply the shadow, and always revert
    let checkpoint = journaled_state.checkpoint();
    let result = apply_shadow(shadow, journaled_state, db);
    journaled_state.checkpoint_revert(checkpoint);
    return result;
}

pub fn apply_shadow<DB: Database + DatabaseCommit>(
    shadow: ShadowTx,
    journaled_state: &mut JournaledState,
    db: &mut DB,
) -> Result<ShadowReceipt, EVMError<DB::Error>> {
    let address = shadow.address.clone();
    // account load procedure: load, then touch/get
    journaled_state.load_account(address, db)?;
    // load primary account
    // accounts need to be marked as `touched` if any state change happens to/from them
    // if they're purely read-only, they can be left untouched.
    journaled_state.touch(&address);
    // we can't use get_mut here due to the `Transfer` function
    let mut primary_account = journaled_state.state.get(&address).unwrap().clone();

    // // tx fee
    // let Some(new_balance) = primary_account.info.balance.checked_sub(shadow.fee) else {
    //     return Ok(ShadowReceipt {
    //         tx_id: shadow.tx_id,
    //         result: ShadowResult::OutOfFunds,
    //         tx_type: shadow.tx,
    //     });
    // };
    // primary_account.info.balance = new_balance;

    // we use case breaks so we can return early from a case block without returning the entire function
    let res = match shadow.tx {
        // ShadowTxType::Null => ShadowResult::Success,
        // ShadowTxType::Data(data_shadow) => 'data_shadow: {
        //     let Some(new_balance) = primary_account.info.balance.checked_sub(data_shadow.fee)
        //     else {
        //         break 'data_shadow ShadowResult::OutOfFunds;
        //     };
        //     primary_account.info.balance = new_balance;
        //     journaled_state.state.insert(address, primary_account);
        //     journaled_state
        //         .journal
        //         .last_mut()
        //         .unwrap()
        //         .push(JournalEntry::DataCostChange { address, cost: data_shadow.fee });
        //     ShadowResult::Success
        // }

        // ShadowTxType::Transfer(transfer) => 'transfer_shadow: {
        //     let TransferShadow { to, amount } = transfer;
        //     journaled_state.load_account(to, db)?;
        //     let mut to_account = journaled_state.state.get(&to).unwrap().clone();
        //     journaled_state.touch(&to);

        //     let Some(new_from_balance) = primary_account.info.balance.checked_sub(amount) else {
        //         break 'transfer_shadow ShadowResult::OutOfFunds;
        //     };

        //     let Some(new_to_balance) = to_account.info.balance.checked_add(amount) else {
        //         break 'transfer_shadow ShadowResult::OverflowPayment;
        //     };

        //     primary_account.info.balance = new_from_balance;
        //     to_account.info.balance = new_to_balance;

        //     journaled_state.state.insert(address, primary_account);
        //     journaled_state.state.insert(to, to_account);
        //     journaled_state.journal.last_mut().unwrap().push(JournalEntry::BalanceTransfer {
        //         from: address,
        //         to,
        //         balance: amount,
        //     });

        //     ShadowResult::Success
        // }

        // ShadowTxType::MiningAddressStake(stake) => {
        //     // check if this account already has a stake
        //     match primary_account.info.stake {
        //         Some(_) => ShadowResult::AlreadyStaked,
        //         None => 'mining_address_stake: {
        //             // check account has the balance required for the stake
        //             let Some(new_balance) = primary_account.info.balance.checked_sub(stake.value)
        //             else {
        //                 break 'mining_address_stake ShadowResult::OutOfFunds;
        //             };
        //             primary_account.info.balance = new_balance;
        //             // add active stake to account
        //             primary_account.info.stake = Some(Stake {
        //                 tx_id: shadow.tx_id,
        //                 quantity: stake.value,
        //                 height: stake.height,
        //                 status: CommitmentStatus::Pending,
        //             });
        //             // add revert record to journal
        //             journaled_state.state.insert(address, primary_account);
        //             journaled_state
        //                 .journal
        //                 .last_mut()
        //                 .unwrap()
        //                 .push(JournalEntry::AddressStaked { address });

        //             ShadowResult::Success
        //         }
        //     }
        // }

        // ShadowTxType::PartitionPledge(pledge_shadow) => 'partition_pledge: {
        //     // assume higher-level validation of pledge is done on erlang side

        //     // // make sure the account has enough balance for this stake
        //     // let Some(new_balance) =
        //     //     primary_account.info.balance.checked_sub(pledge_shadow.quantity)
        //     // else {
        //     //     break 'partition_pledge ShadowResult::OutOfFunds;
        //     // };
        //     // // check a pledge for this target (`dest_hash`) doesn't already exist
        //     // match &mut primary_account.info.commitments {
        //     //     Some(commitments) => {
        //     //         if commitments
        //     //             .iter()
        //     //             .find(|p| {
        //     //                 p.dest_hash.is_part_hash() && p.dest_hash == pledge_shadow.part_hash
        //     //             })
        //     //             .is_some()
        //     //         {
        //     //             break 'partition_pledge ShadowResult::AlreadyPledged;
        //     //         }
        //     //     }
        //     //     None => (),
        //     // }

        //     // primary_account.info.balance = new_balance;
        //     // let pledge = Commitment {
        //     //     tx_id: shadow.tx_id,
        //     //     quantity: pledge_shadow.quantity,
        //     //     dest_hash: pledge_shadow.part_hash,
        //     //     height: pledge_shadow.height,
        //     //     status: CommitmentStatus::Pending,
        //     //     tx_type: CommitmentType::Pledge,
        //     // };
        //     // match &mut primary_account.info.commitments {
        //     //     Some(commitments) => commitments.push(pledge),
        //     //     None => primary_account.info.commitments = Some(vec![pledge].into()),
        //     // };

        //     // journaled_state.state.insert(address, primary_account);

        //     // // add revert record to journal
        //     // journaled_state.journal.last_mut().unwrap().push(JournalEntry::PartitionPledged {
        //     //     address,
        //     //     dest_hash: pledge_shadow.part_hash,
        //     // });

        //     ShadowResult::Success
        // }
        // ShadowTxType::PartitionUnPledge(unpledge) => 'partition_unpledge: {
        //     match &mut primary_account.info.commitments {
        //         None => ShadowResult::NoPledges,
        //         Some(pledges) => {
        //             // // find relevant pledge - only refund `Active` pledges
        //             // // TODO: should we allow instant refunds for pending pledges?
        //             // let pledge = match pledges.iter_mut().find(|p| {
        //             //     p.dest_hash == unpledge.part_hash && p.status == CommitmentStatus::Active
        //             // }) {
        //             //     Some(p) => p,
        //             //     None => break 'partition_unpledge ShadowResult::NoMatchingPledge,
        //             // };
        //             // // check we can add refund the account without overflowing
        //             // let Some(new_balance) =
        //             //     primary_account.info.balance.checked_add(pledge.quantity)
        //             // else {
        //             //     break 'partition_unpledge ShadowResult::OverflowPayment;
        //             // };

        //             // primary_account.info.balance = new_balance;
        //             // // change this to an unpledge record

        //             // pledge.update_status(CommitmentStatus::Active);

        //             // journaled_state.state.insert(address, primary_account);

        //             // // add revert record to journal
        //             // journaled_state.journal.last_mut().unwrap().push(
        //             //     JournalEntry::PartitionUnPledge { address, dest_hash: unpledge.part_hash },
        //             // );

        //             ShadowResult::Success
        //         }
        //     }
        // }
        // ShadowTxType::Unstake(_unpledge_shadow) => 'unpledge_all: {
        //     // // remove/refund account stake
        //     // match &mut primary_account.info.stake {
        //     //     None => (),
        //     //     Some(stake) => {
        //     //         if stake.status == CommitmentStatus::Active {
        //     //             let Some(new_balance) =
        //     //                 primary_account.info.balance.checked_add(stake.quantity)
        //     //             else {
        //     //                 break 'unpledge_all ShadowResult::OverflowPayment;
        //     //             };
        //     //             stake.update_status(CommitmentStatus::Inactive);
        //     //             primary_account.info.balance = new_balance;
        //     //         }
        //     //     }
        //     // }

        //     // // remove/refund all `Active` pledges
        //     // // TODO: handling for other pledge states
        //     // let original_pledges: Option<Vec<(IrysTxId, CommitmentStatus)>> =
        //     //     match &mut primary_account.info.commitments {
        //     //         None => None,
        //     //         Some(pledges) => {
        //     //             let mut original_pledges = vec![];
        //     //             for pledge in
        //     //                 pledges.iter_mut().filter(|p| p.status == CommitmentStatus::Active)
        //     //             {
        //     //                 original_pledges.push((pledge.tx_id.clone(), pledge.status.clone()));
        //     //                 let Some(new_balance) =
        //     //                     primary_account.info.balance.checked_add(pledge.quantity)
        //     //                 else {
        //     //                     break 'unpledge_all ShadowResult::OverflowPayment;
        //     //                 };
        //     //                 pledge.update_status(CommitmentStatus::Active);
        //     //                 primary_account.info.balance = new_balance;
        //     //             }
        //     //             Some(original_pledges)
        //     //         }
        //     //     };

        //     // journaled_state.state.insert(address, primary_account);

        //     // // add revert record to journal
        //     // journaled_state.journal.last_mut().unwrap().push(JournalEntry::AddressUnstake {
        //     //     address,
        //     //     deactivated_pledges: original_pledges,
        //     // });
        //     ShadowResult::Success
        // }
        // ShadowTxType::Slash(_slash_shadow) => 'slash: {
        //     // remove/refund account stake
        //     match &mut primary_account.info.stake {
        //         None => (),
        //         Some(stake) => {
        //             if stake.status == CommitmentStatus::Active {
        //                 stake.update_status(CommitmentStatus::Slashed);
        //                 // TODO: transfer slashed tokens to slasher(s)
        //             }
        //         }
        //     }

        //     // remove all `Active` pledges
        //     // TODO: handling for other pledge states
        //     let _original_pledges: Option<Vec<(IrysTxId, CommitmentStatus)>> =
        //         match &mut primary_account.info.commitments {
        //             None => None,
        //             Some(pledges) => {
        //                 let mut original_pledges = vec![];
        //                 for pledge in
        //                     pledges.iter_mut().filter(|p| p.status == CommitmentStatus::Active)
        //                 {
        //                     original_pledges.push((pledge.tx_id.clone(), pledge.status.clone()));
        //                     let Some(_new_balance) =
        //                         primary_account.info.balance.checked_add(pledge.quantity)
        //                     else {
        //                         break 'slash ShadowResult::OverflowPayment;
        //                     };
        //                     pledge.update_status(CommitmentStatus::Slashed);
        //                     // TODO: transfer slashed tokens to slasher(s)

        //                     // primary_account.info.balance = new_balance;
        //                 }
        //                 Some(original_pledges)
        //             }
        //         };

        //     // add revert record to journal
        //     journaled_state.state.insert(address, primary_account);
        //     journaled_state.journal.last_mut().unwrap().push(JournalEntry::AddressSlashed {});
        //     ShadowResult::Success
        // }
        // ShadowTxType::BlockReward(reward) => 'block_reward: {
        //     let Some(new_producer_balance) =
        //         primary_account.info.balance.checked_add(reward.reward)
        //     else {
        //         break 'block_reward ShadowResult::OverflowPayment;
        //     };
        //     primary_account.info.balance = new_producer_balance;
        //     journaled_state.state.insert(address, primary_account);
        //     journaled_state
        //         .journal
        //         .last_mut()
        //         .unwrap()
        //         .push(JournalEntry::BlockReward { address, reward: reward.reward });
        //     ShadowResult::Success
        // }
        ShadowTxType::Diff(ref new_state) => {
            let og = primary_account.info.clone();
            let og_unmoved = primary_account.info.clone();
            let new = new_state.clone().new_state;
            // "diff"
            let new_account_info = AccountInfo {
                balance: new.balance.unwrap_or(og.balance),
                nonce: new.nonce.unwrap_or(og.nonce),
                code_hash: og.code_hash,
                code: og.code,
                stake: new.stake.map_or(og.stake, |c| c.0),
                commitments: new.commitments.map_or(og.commitments, |c| c.0),
                last_tx: new.last_tx.map_or(og.last_tx, |c| c.0),
                mining_permission: new.mining_permission.map_or(og.mining_permission, |c| Some(c)),
            };
            // info!("New Account State {} {:#?} {:#?}", &address, &new_account_info, &og_unmoved);
            primary_account.info = new_account_info;
            journaled_state.state.insert(address, primary_account);
            journaled_state
                .journal
                .last_mut()
                .unwrap()
                .push(JournalEntry::AccountDiff { address, old_account: og_unmoved });
            ShadowResult::Success
        }
        // other pledge types are disabled for now
        _ => todo!(),
    };

    // if res == ShadowResult::Success && !(shadow.tx.type_id() as u8 == ShadowTxTypeId::Diff as u8) {
    //     // update last_tx on successful tx - DO NOT UPDATE FOR DIFF SHADOWS
    //     let mut primary_account = journaled_state.state.get(&address).unwrap().clone();
    //     let prev_last = primary_account.info.last_tx.clone();
    //     primary_account.info.last_tx = Some(LastTx::TxId(shadow.tx_id.clone()));

    //     journaled_state.state.insert(address, primary_account);

    //     journaled_state
    //         .journal
    //         .last_mut()
    //         .unwrap()
    //         .push(JournalEntry::UpdateLastTx { address, prev_last_tx: prev_last })
    // }
    // journaled_state.state.insert(address, primary_account);
    Ok(ShadowReceipt { tx_id: shadow.tx_id, result: res, tx_type: shadow.tx })
}

//

/// Returns a map of addresses to their balance increments if the Shanghai hardfork is active at the
/// given timestamp.
///
/// Zero-valued withdrawals are filtered out.
#[inline]
pub fn post_block_withdrawals_balance_increments<ChainSpec: EthereumHardforks>(
    chain_spec: &ChainSpec,
    block_timestamp: u64,
    withdrawals: &[Withdrawal],
) -> HashMap<Address, u128> {
    let mut balance_increments =
        HashMap::with_capacity_and_hasher(withdrawals.len(), Default::default());
    insert_post_block_withdrawals_balance_increments(
        chain_spec,
        block_timestamp,
        Some(withdrawals),
        &mut balance_increments,
    );
    balance_increments
}

/// Applies all withdrawal balance increments if shanghai is active at the given timestamp to the
/// given `balance_increments` map.
///
/// Zero-valued withdrawals are filtered out.
#[inline]
pub fn insert_post_block_withdrawals_balance_increments<ChainSpec: EthereumHardforks>(
    chain_spec: &ChainSpec,
    block_timestamp: u64,
    withdrawals: Option<&[Withdrawal]>,
    balance_increments: &mut HashMap<Address, u128>,
) {
    // Process withdrawals
    if chain_spec.is_shanghai_active_at_timestamp(block_timestamp) {
        if let Some(withdrawals) = withdrawals {
            for withdrawal in withdrawals {
                if withdrawal.amount > 0 {
                    *balance_increments.entry(withdrawal.address).or_default() +=
                        withdrawal.amount_wei().to::<u128>();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth_chainspec::ChainSpec;
    use reth_ethereum_forks::{ChainHardforks, EthereumHardfork, ForkCondition};
    use reth_primitives::constants::GWEI_TO_WEI;

    /// Tests that the function correctly inserts balance increments when the Shanghai hardfork is
    /// active and there are withdrawals.
    #[test]
    fn test_insert_post_block_withdrawals_balance_increments_shanghai_active_with_withdrawals() {
        // Arrange
        // Create a ChainSpec with the Shanghai hardfork active at timestamp 100
        let chain_spec = ChainSpec {
            hardforks: ChainHardforks::new(vec![(
                Box::new(EthereumHardfork::Shanghai),
                ForkCondition::Timestamp(100),
            )]),
            ..Default::default()
        };

        // Define the block timestamp and withdrawals
        let block_timestamp = 1000;
        let withdrawals = vec![
            Withdrawal {
                address: Address::from([1; 20]),
                amount: 1000,
                index: 45,
                validator_index: 12,
            },
            Withdrawal {
                address: Address::from([2; 20]),
                amount: 500,
                index: 412,
                validator_index: 123,
            },
        ];

        // Create an empty HashMap to hold the balance increments
        let mut balance_increments = HashMap::default();

        // Act
        // Call the function with the prepared inputs
        insert_post_block_withdrawals_balance_increments(
            &chain_spec,
            block_timestamp,
            Some(&withdrawals),
            &mut balance_increments,
        );

        // Assert
        // Verify that the balance increments map has the correct number of entries
        assert_eq!(balance_increments.len(), 2);
        // Verify that the balance increments map contains the correct values for each address
        assert_eq!(
            *balance_increments.get(&Address::from([1; 20])).unwrap(),
            (1000 * GWEI_TO_WEI).into()
        );
        assert_eq!(
            *balance_increments.get(&Address::from([2; 20])).unwrap(),
            (500 * GWEI_TO_WEI).into()
        );
    }

    /// Tests that the function correctly handles the case when Shanghai is active but there are no
    /// withdrawals.
    #[test]
    fn test_insert_post_block_withdrawals_balance_increments_shanghai_active_no_withdrawals() {
        // Arrange
        // Create a ChainSpec with the Shanghai hardfork active
        let chain_spec = ChainSpec {
            hardforks: ChainHardforks::new(vec![(
                Box::new(EthereumHardfork::Shanghai),
                ForkCondition::Timestamp(100),
            )]),
            ..Default::default()
        };

        // Define the block timestamp and an empty list of withdrawals
        let block_timestamp = 1000;
        let withdrawals = Vec::<Withdrawal>::new();

        // Create an empty HashMap to hold the balance increments
        let mut balance_increments = HashMap::default();

        // Act
        // Call the function with the prepared inputs
        insert_post_block_withdrawals_balance_increments(
            &chain_spec,
            block_timestamp,
            Some(&withdrawals),
            &mut balance_increments,
        );

        // Assert
        // Verify that the balance increments map is empty
        assert!(balance_increments.is_empty());
    }

    /// Tests that the function correctly handles the case when Shanghai is not active even if there
    /// are withdrawals.
    #[test]
    fn test_insert_post_block_withdrawals_balance_increments_shanghai_not_active_with_withdrawals()
    {
        // Arrange
        // Create a ChainSpec without the Shanghai hardfork active
        let chain_spec = ChainSpec::default(); // Mock chain spec with Shanghai not active

        // Define the block timestamp and withdrawals
        let block_timestamp = 1000;
        let withdrawals = vec![
            Withdrawal {
                address: Address::from([1; 20]),
                amount: 1000,
                index: 45,
                validator_index: 12,
            },
            Withdrawal {
                address: Address::from([2; 20]),
                amount: 500,
                index: 412,
                validator_index: 123,
            },
        ];

        // Create an empty HashMap to hold the balance increments
        let mut balance_increments = HashMap::default();

        // Act
        // Call the function with the prepared inputs
        insert_post_block_withdrawals_balance_increments(
            &chain_spec,
            block_timestamp,
            Some(&withdrawals),
            &mut balance_increments,
        );

        // Assert
        // Verify that the balance increments map is empty
        assert!(balance_increments.is_empty());
    }

    /// Tests that the function correctly handles the case when Shanghai is active but all
    /// withdrawals have zero amounts.
    #[test]
    fn test_insert_post_block_withdrawals_balance_increments_shanghai_active_with_zero_withdrawals()
    {
        // Arrange
        // Create a ChainSpec with the Shanghai hardfork active
        let chain_spec = ChainSpec {
            hardforks: ChainHardforks::new(vec![(
                Box::new(EthereumHardfork::Shanghai),
                ForkCondition::Timestamp(100),
            )]),
            ..Default::default()
        };

        // Define the block timestamp and withdrawals with zero amounts
        let block_timestamp = 1000;
        let withdrawals = vec![
            Withdrawal {
                address: Address::from([1; 20]),
                amount: 0, // Zero withdrawal amount
                index: 45,
                validator_index: 12,
            },
            Withdrawal {
                address: Address::from([2; 20]),
                amount: 0, // Zero withdrawal amount
                index: 412,
                validator_index: 123,
            },
        ];

        // Create an empty HashMap to hold the balance increments
        let mut balance_increments = HashMap::default();

        // Act
        // Call the function with the prepared inputs
        insert_post_block_withdrawals_balance_increments(
            &chain_spec,
            block_timestamp,
            Some(&withdrawals),
            &mut balance_increments,
        );

        // Assert
        // Verify that the balance increments map is empty
        assert!(balance_increments.is_empty());
    }

    /// Tests that the function correctly handles the case when Shanghai is active but there are no
    /// withdrawals provided.
    #[test]
    fn test_insert_post_block_withdrawals_balance_increments_shanghai_active_with_empty_withdrawals(
    ) {
        // Arrange
        // Create a ChainSpec with the Shanghai hardfork active
        let chain_spec = ChainSpec {
            hardforks: ChainHardforks::new(vec![(
                Box::new(EthereumHardfork::Shanghai),
                ForkCondition::Timestamp(100),
            )]),
            ..Default::default()
        };

        // Define the block timestamp and no withdrawals
        let block_timestamp = 1000;
        let withdrawals = None; // No withdrawals provided

        // Create an empty HashMap to hold the balance increments
        let mut balance_increments = HashMap::default();

        // Act
        // Call the function with the prepared inputs
        insert_post_block_withdrawals_balance_increments(
            &chain_spec,
            block_timestamp,
            withdrawals,
            &mut balance_increments,
        );

        // Assert
        // Verify that the balance increments map is empty
        assert!(balance_increments.is_empty());
    }
}
