use reth_consensus_common::calc;
use reth_execution_errors::{BlockExecutionError, BlockValidationError};
use reth_primitives::{
    revm::env::fill_tx_env_with_beacon_root_contract_call, Address, ChainSpec, Header,
    ShadowReceipt, ShadowResult, Withdrawal, B256, U256,
};
use revm::{
    interpreter::Host,
    primitives::{
        commitment::{Commitment, CommitmentStatus, Commitments, IrysTxId, LastTx, Stake},
        shadow::{ShadowTx, ShadowTxType, Shadows, TransferShadow},
        Account, EVMError, State,
    },
    Database, DatabaseCommit, Evm, JournalEntry, JournaledState,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Error,
    ops::DerefMut,
};
use tracing::trace;

/// Collect all balance changes at the end of the block.
///
/// Balance changes might include the block reward, uncle rewards, withdrawals, or irregular
/// state changes (DAO fork).
#[allow(clippy::too_many_arguments)]
#[inline]
pub fn post_block_balance_increments(
    chain_spec: &ChainSpec,
    block_number: u64,
    block_difficulty: U256,
    beneficiary: Address,
    block_timestamp: u64,
    total_difficulty: U256,
    ommers: &[Header],
    withdrawals: Option<&[Withdrawal]>,
) -> HashMap<Address, u128> {
    let mut balance_increments = HashMap::new();

    // Add block rewards if they are enabled.
    if let Some(base_block_reward) =
        calc::base_block_reward(chain_spec, block_number, block_difficulty, total_difficulty)
    {
        // Ommer rewards
        for ommer in ommers {
            *balance_increments.entry(ommer.beneficiary).or_default() +=
                calc::ommer_reward(base_block_reward, block_number, ommer.number);
        }

        // Full block reward
        *balance_increments.entry(beneficiary).or_default() +=
            calc::block_reward(base_block_reward, ommers.len());
    }

    // process withdrawals
    insert_post_block_withdrawals_balance_increments(
        chain_spec,
        block_timestamp,
        withdrawals,
        &mut balance_increments,
    );

    balance_increments
}

/// Applies the pre-block call to the EIP-4788 beacon block root contract, using the given block,
/// [ChainSpec], EVM.
///
/// If cancun is not activated or the block is the genesis block, then this is a no-op, and no
/// state changes are made.
#[inline]
pub fn apply_beacon_root_contract_call<EXT, DB: Database + DatabaseCommit>(
    chain_spec: &ChainSpec,
    block_timestamp: u64,
    block_number: u64,
    parent_beacon_block_root: Option<B256>,
    evm: &mut Evm<'_, EXT, DB>,
) -> Result<(), BlockExecutionError>
where
    DB::Error: std::fmt::Display,
{
    if !chain_spec.is_cancun_active_at_timestamp(block_timestamp) {
        return Ok(());
    }

    let parent_beacon_block_root =
        parent_beacon_block_root.ok_or(BlockValidationError::MissingParentBeaconBlockRoot)?;

    // if the block number is zero (genesis block) then the parent beacon block root must
    // be 0x0 and no system transaction may occur as per EIP-4788
    if block_number == 0 {
        if parent_beacon_block_root != B256::ZERO {
            return Err(BlockValidationError::CancunGenesisParentBeaconBlockRootNotZero {
                parent_beacon_block_root,
            }
            .into());
        }
        return Ok(());
    }

    // get previous env
    let previous_env = Box::new(evm.context.env().clone());

    // modify env for pre block call
    fill_tx_env_with_beacon_root_contract_call(&mut evm.context.evm.env, parent_beacon_block_root);

    let mut state = match evm.transact() {
        Ok(res) => res.state,
        Err(e) => {
            evm.context.evm.env = previous_env;
            return Err(BlockValidationError::BeaconRootContractCall {
                parent_beacon_block_root: Box::new(parent_beacon_block_root),
                message: e.to_string(),
            }
            .into());
        }
    };

    state.remove(&alloy_eips::eip4788::SYSTEM_ADDRESS);
    state.remove(&evm.block().coinbase);

    evm.context.evm.db.commit(state);

    // re-set the previous env
    evm.context.evm.env = previous_env;

    Ok(())
}

// Applies the pre-block Irys transaction shadows, using the given block,
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
            shadow,
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

    // tx fee
    let Some(new_balance) = primary_account.info.balance.checked_sub(shadow.fee) else {
        return Ok(ShadowReceipt {
            tx_id: shadow.tx_id,
            result: ShadowResult::OutOfFunds,
            tx_type: shadow.tx,
        });
    };
    primary_account.info.balance = new_balance;

    // we use case breaks so we can return early from a case block without returning the entire function
    let res = match shadow.tx {
        ShadowTxType::Null => ShadowResult::Success,
        ShadowTxType::Data(data_shadow) => 'data_shadow: {
            let Some(new_balance) = primary_account.info.balance.checked_sub(data_shadow.fee)
            else {
                break 'data_shadow ShadowResult::OutOfFunds;
            };
            primary_account.info.balance = new_balance;
            journaled_state.state.insert(address, primary_account);
            journaled_state
                .journal
                .last_mut()
                .unwrap()
                .push(JournalEntry::DataCostChange { address, cost: data_shadow.fee });
            ShadowResult::Success
        }

        ShadowTxType::Transfer(transfer) => 'transfer_shadow: {
            let TransferShadow { to, amount } = transfer;
            journaled_state.load_account(to, db)?;
            let mut to_account = journaled_state.state.get(&to).unwrap().clone();
            journaled_state.touch(&to);

            let Some(new_from_balance) = primary_account.info.balance.checked_sub(amount) else {
                break 'transfer_shadow ShadowResult::OutOfFunds;
            };

            let Some(new_to_balance) = to_account.info.balance.checked_add(amount) else {
                break 'transfer_shadow ShadowResult::OverflowPayment;
            };

            primary_account.info.balance = new_from_balance;
            to_account.info.balance = new_to_balance;

            journaled_state.state.insert(address, primary_account);
            journaled_state.state.insert(to, to_account);
            journaled_state.journal.last_mut().unwrap().push(JournalEntry::BalanceTransfer {
                from: address,
                to,
                balance: amount,
            });

            ShadowResult::Success
        }

        ShadowTxType::MiningAddressStake(stake) => {
            // check if this account already has a stake
            match primary_account.info.stake {
                Some(_) => ShadowResult::AlreadyStaked,
                None => 'mining_address_stake: {
                    // check account has the balance required for the stake
                    let Some(new_balance) = primary_account.info.balance.checked_sub(stake.value)
                    else {
                        break 'mining_address_stake ShadowResult::OutOfFunds;
                    };
                    primary_account.info.balance = new_balance;
                    // add active stake to account
                    primary_account.info.stake = Some(Stake {
                        tx_id: shadow.tx_id,
                        quantity: stake.value,
                        height: stake.height,
                        status: CommitmentStatus::Pending,
                    });
                    // add revert record to journal
                    journaled_state.state.insert(address, primary_account);
                    journaled_state
                        .journal
                        .last_mut()
                        .unwrap()
                        .push(JournalEntry::AddressStaked { address });

                    ShadowResult::Success
                }
            }
        }

        ShadowTxType::PartitionPledge(pledge_shadow) => 'partition_pledge: {
            // assume higher-level validation of pledge is done on erlang side

            // make sure the account has enough balance for this stake
            let Some(new_balance) =
                primary_account.info.balance.checked_sub(pledge_shadow.quantity)
            else {
                break 'partition_pledge ShadowResult::OutOfFunds;
            };
            // check a pledge for this target (`dest_hash`) doesn't already exist
            match &mut primary_account.info.commitments {
                Some(commitments) => {
                    if commitments.iter().find(|p| p.dest_hash == pledge_shadow.part_hash).is_some()
                    {
                        break 'partition_pledge ShadowResult::AlreadyPledged;
                    }
                }
                None => (),
            }

            primary_account.info.balance = new_balance;
            let pledge = Commitment {
                tx_id: shadow.tx_id,
                quantity: pledge_shadow.quantity,
                dest_hash: pledge_shadow.part_hash,
                height: pledge_shadow.height,
                status: CommitmentStatus::Pending,
            };
            match &mut primary_account.info.commitments {
                Some(commitments) => commitments.push(pledge),
                None => primary_account.info.commitments = Some(vec![pledge].into()),
            };

            journaled_state.state.insert(address, primary_account);

            // add revert record to journal
            journaled_state.journal.last_mut().unwrap().push(JournalEntry::PartitionPledged {
                address,
                dest_hash: pledge_shadow.part_hash,
            });

            ShadowResult::Success
        }
        ShadowTxType::PartitionUnPledge(unpledge) => 'partition_unpledge: {
            match &mut primary_account.info.commitments {
                None => ShadowResult::NoPledges,
                Some(pledges) => {
                    // find relevant pledge - only refund `Active` pledges
                    // TODO: should we allow instant refunds for pending pledges?
                    let pledge = match pledges.iter_mut().find(|p| {
                        p.dest_hash == unpledge.part_hash && p.status == CommitmentStatus::Active
                    }) {
                        Some(p) => p,
                        None => break 'partition_unpledge ShadowResult::NoMatchingPledge,
                    };
                    // check we can add refund the account without overflowing
                    let Some(new_balance) =
                        primary_account.info.balance.checked_add(pledge.quantity)
                    else {
                        break 'partition_unpledge ShadowResult::OverflowPayment;
                    };

                    primary_account.info.balance = new_balance;
                    // change this to an unpledge record

                    pledge.update_status(CommitmentStatus::Active);

                    journaled_state.state.insert(address, primary_account);

                    // add revert record to journal
                    journaled_state.journal.last_mut().unwrap().push(
                        JournalEntry::PartitionUnPledge { address, dest_hash: unpledge.part_hash },
                    );

                    ShadowResult::Success
                }
            }
        }
        ShadowTxType::Unstake(_unpledge_shadow) => 'unpledge_all: {
            // remove/refund account stake
            match &mut primary_account.info.stake {
                None => (),
                Some(stake) => {
                    if stake.status == CommitmentStatus::Active {
                        let Some(new_balance) =
                            primary_account.info.balance.checked_add(stake.quantity)
                        else {
                            break 'unpledge_all ShadowResult::OverflowPayment;
                        };
                        stake.update_status(CommitmentStatus::Inactive);
                        primary_account.info.balance = new_balance;
                    }
                }
            }

            // remove/refund all `Active` pledges
            // TODO: handling for other pledge states
            let original_pledges: Option<Vec<(IrysTxId, CommitmentStatus)>> =
                match &mut primary_account.info.commitments {
                    None => None,
                    Some(pledges) => {
                        let mut original_pledges = vec![];
                        for pledge in
                            pledges.iter_mut().filter(|p| p.status == CommitmentStatus::Active)
                        {
                            original_pledges.push((pledge.tx_id.clone(), pledge.status.clone()));
                            let Some(new_balance) =
                                primary_account.info.balance.checked_add(pledge.quantity)
                            else {
                                break 'unpledge_all ShadowResult::OverflowPayment;
                            };
                            pledge.update_status(CommitmentStatus::Active);
                            primary_account.info.balance = new_balance;
                        }
                        Some(original_pledges)
                    }
                };

            journaled_state.state.insert(address, primary_account);

            // add revert record to journal
            journaled_state.journal.last_mut().unwrap().push(JournalEntry::AddressUnstake {
                address,
                deactivated_pledges: original_pledges,
            });
            ShadowResult::Success
        }
        ShadowTxType::Slash(_slash_shadow) => 'slash: {
            // remove/refund account stake
            match &mut primary_account.info.stake {
                None => (),
                Some(stake) => {
                    if stake.status == CommitmentStatus::Active {
                        stake.update_status(CommitmentStatus::Slashed);
                        // TODO: transfer slashed tokens to slasher(s)
                    }
                }
            }

            // remove all `Active` pledges
            // TODO: handling for other pledge states
            let _original_pledges: Option<Vec<(IrysTxId, CommitmentStatus)>> =
                match &mut primary_account.info.commitments {
                    None => None,
                    Some(pledges) => {
                        let mut original_pledges = vec![];
                        for pledge in
                            pledges.iter_mut().filter(|p| p.status == CommitmentStatus::Active)
                        {
                            original_pledges.push((pledge.tx_id.clone(), pledge.status.clone()));
                            let Some(_new_balance) =
                                primary_account.info.balance.checked_add(pledge.quantity)
                            else {
                                break 'slash ShadowResult::OverflowPayment;
                            };
                            pledge.update_status(CommitmentStatus::Slashed);
                            // TODO: transfer slashed tokens to slasher(s)

                            // primary_account.info.balance = new_balance;
                        }
                        Some(original_pledges)
                    }
                };

            // add revert record to journal
            journaled_state.state.insert(address, primary_account);
            journaled_state.journal.last_mut().unwrap().push(JournalEntry::AddressSlashed {});
            ShadowResult::Success
        }
        ShadowTxType::BlockReward(reward) => 'block_reward: {
            let Some(new_producer_balance) =
                primary_account.info.balance.checked_add(reward.reward)
            else {
                break 'block_reward ShadowResult::OverflowPayment;
            };

            ShadowResult::Success
        }
    };

    if res == ShadowResult::Success {
        // update last_tx on successful tx
        let mut primary_account = journaled_state.state.get(&address).unwrap().clone();
        let prev_last = primary_account.info.last_tx.clone();
        primary_account.info.last_tx = Some(LastTx::TxId(shadow.tx_id.clone()));

        journaled_state.state.insert(address, primary_account);

        journaled_state
            .journal
            .last_mut()
            .unwrap()
            .push(JournalEntry::UpdateLastTx { address, prev_last_tx: prev_last })
    }
    Ok(ShadowReceipt { tx_id: shadow.tx_id, result: res, tx_type: shadow.tx })
}

//

/// Returns a map of addresses to their balance increments if the Shanghai hardfork is active at the
/// given timestamp.
///
/// Zero-valued withdrawals are filtered out.
#[inline]
pub fn post_block_withdrawals_balance_increments(
    chain_spec: &ChainSpec,
    block_timestamp: u64,
    withdrawals: &[Withdrawal],
) -> HashMap<Address, u128> {
    let mut balance_increments = HashMap::with_capacity(withdrawals.len());
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
pub fn insert_post_block_withdrawals_balance_increments(
    chain_spec: &ChainSpec,
    block_timestamp: u64,
    withdrawals: Option<&[Withdrawal]>,
    balance_increments: &mut HashMap<Address, u128>,
) {
    // Process withdrawals
    if chain_spec.is_shanghai_active_at_timestamp(block_timestamp) {
        if let Some(withdrawals) = withdrawals {
            for withdrawal in withdrawals.iter() {
                if withdrawal.amount > 0 {
                    *balance_increments.entry(withdrawal.address).or_default() +=
                        withdrawal.amount_wei().to::<u128>();
                }
            }
        }
    }
}

// pub fn simulate_apply_shadow<EXT, DB: Database + DatabaseCommit>(
//     shadow: ShadowTx,
//     evm: &mut Evm<'_, EXT, DB>,
// ) -> Result<ShadowReceipt, EVMError<DB::Error>> {
//     // create a checkpoint, try to apply the shadow, and always revert
//     let checkpoint = evm.context.evm.inner.journaled_state.checkpoint();
//     // let result = apply_shadow(shadow, evm);
//     let result = apply_shadow(shadow, evm);
//     evm.context.evm.inner.journaled_state.checkpoint_revert(checkpoint);
//     return result;
// }
