use crate::consts::*;
use crate::context::Context;
use crate::crypto;
use crate::error::{Bug, BugId, BugVariant, InterpreterError, RuntimeError};
use crate::interpreter::Interpreter;
use crate::predicate::RuntimePredicate;
use crate::state::StateTransition;
use crate::state::{ExecuteState, ProgramState, StateTransitionRef};
use crate::storage::{InterpreterStorage, PredicateStorage};

use fuel_asm::PanicReason;
use fuel_tx::canonical::Serialize;
use fuel_tx::CheckedTransaction;
use fuel_tx::{ConsensusParameters, Contract, Input, Output, Receipt, ScriptExecutionResult, Transaction};
use fuel_types::Word;

// FIXME replace for a type-safe transaction
impl Interpreter<PredicateStorage> {
    /// Initialize the VM with the provided transaction and check all predicates defined in the
    /// inputs.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for predicates.
    /// This way, its possible, for the sake of simplicity, it is possible to use
    /// [unit](https://doc.rust-lang.org/core/primitive.unit.html) as storage provider.
    ///
    /// # Debug
    ///
    /// This is not a valid entrypoint for debug calls. It will only return a `bool`, and not the
    /// VM state required to trace the execution steps.
    pub fn check_predicates(tx: CheckedTransaction, params: ConsensusParameters) -> bool {
        let mut vm = Interpreter::with_storage(PredicateStorage::default(), params);

        if !tx.as_ref().check_predicate_owners() {
            return false;
        }

        let predicates: Vec<RuntimePredicate> = tx
            .as_ref()
            .inputs()
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| RuntimePredicate::from_tx(&params, tx.as_ref(), idx))
            .collect();

        predicates
            .into_iter()
            .fold(vm.init_predicate(tx), |result, predicate| -> bool {
                // VM is cloned because the state should be reset for every predicate verification
                result && vm.clone()._check_predicate(predicate)
            })
    }

    /// Initialize the VM with the provided transaction and check the input predicate indexed by
    /// `idx`. If the input isn't of type [`Input::CoinPredicate`], the function will return
    /// `false`.
    ///
    /// For additional information, check [`Self::check_predicates`]
    pub fn check_predicate(&mut self, tx: CheckedTransaction, idx: usize) -> bool {
        tx.as_ref()
            .check_predicate_owner(idx)
            .then(|| RuntimePredicate::from_tx(self.params(), tx.as_ref(), idx))
            .flatten()
            .map(|predicate| self.init_predicate(tx) && self._check_predicate(predicate))
            .unwrap_or(false)
    }

    /// Validate the predicate, assuming the interpreter is initialized
    fn _check_predicate(&mut self, predicate: RuntimePredicate) -> bool {
        self.context = Context::Predicate { program: predicate };

        match self.verify_predicate() {
            Ok(ProgramState::Return(0x01)) => true,
            _ => false,
        }
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError> {
        let tx = self.tx.transaction();
        let storage = &mut self.storage;

        let state = match tx {
            Transaction::Create {
                salt, storage_slots, ..
            } => {
                let contract = Contract::try_from(tx)?;
                let root = contract.root();
                let storage_root = Contract::initial_state_root(storage_slots.iter());
                let id = contract.id(salt, &root, &storage_root);

                if !tx
                    .outputs()
                    .iter()
                    .any(|output| matches!(output, Output::ContractCreated { contract_id, state_root } if contract_id == &id && state_root == &storage_root))
                {
                    return Err(InterpreterError::Panic(PanicReason::ContractNotInInputs));
                }

                storage
                    .deploy_contract_with_id(salt, storage_slots, &contract, &root, &id)
                    .map_err(InterpreterError::from_io)?;

                ProgramState::Return(1)
            }

            Transaction::Script { inputs, .. } => {
                if inputs.iter().any(|input| {
                    if let Input::Contract { contract_id, .. } = input {
                        !self.check_contract_exists(contract_id).unwrap_or(false)
                    } else {
                        false
                    }
                }) {
                    return Err(InterpreterError::Panic(PanicReason::ContractNotFound));
                }

                let offset = (self.tx_offset() + Transaction::script_offset()) as Word;

                self.registers[REG_PC] = offset;
                self.registers[REG_IS] = offset;

                // TODO set tree balance

                let program = self.run_program();
                let gas_used = self
                    .transaction()
                    .gas_limit()
                    .checked_sub(self.registers[REG_GGAS])
                    .ok_or_else(|| Bug::new(BugId::ID006, BugVariant::GlobalGasUnderflow))?;

                // Catch VM panic and don't propagate, generating a receipt
                let (status, program) = match program {
                    Ok(s) => {
                        // either a revert or success
                        let res = if let ProgramState::Revert(_) = &s {
                            ScriptExecutionResult::Revert
                        } else {
                            ScriptExecutionResult::Success
                        };
                        (res, s)
                    }

                    Err(e) => match e.instruction_result() {
                        Some(result) => {
                            self.append_panic_receipt(*result);

                            (ScriptExecutionResult::Panic, ProgramState::Revert(0))
                        }

                        // This isn't a specified case of an erroneous program and should be
                        // propagated. If applicable, OS errors will fall into this category.
                        None => {
                            return Err(e);
                        }
                    },
                };

                let receipt = Receipt::script_result(status, gas_used);

                self.append_receipt(receipt);

                program
            }
        };

        #[cfg(feature = "debug")]
        if state.is_debug() {
            self.debugger_set_last_state(state.clone());
        }

        // TODO optimize
        if self.transaction().receipts_root().is_some() {
            let receipts_root = if self.receipts().is_empty() {
                EMPTY_RECEIPTS_MERKLE_ROOT.into()
            } else {
                crypto::ephemeral_merkle_root(self.receipts().iter().map(|r| r.clone().to_bytes()))
            };

            // TODO: also set this on the serialized tx in memory to keep serialized form consistent
            // https://github.com/FuelLabs/fuel-vm/issues/97
            self.tx.tx_set_receipts_root(receipts_root);
        }

        let revert = matches!(state, ProgramState::Revert(_));

        self.finalize_outputs(revert)?;

        Ok(state)
    }

    pub(crate) fn run_call(&mut self) -> Result<ProgramState, RuntimeError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(PanicReason::MemoryOverflow.into());
            }

            let state = self.execute().map_err(|e| {
                e.panic_reason()
                    .map(RuntimeError::Recoverable)
                    .unwrap_or(RuntimeError::Halt(e.into()))
            })?;

            match state {
                ExecuteState::Return(r) => {
                    return Ok(ProgramState::Return(r));
                }

                ExecuteState::ReturnData(d) => {
                    return Ok(ProgramState::ReturnData(d));
                }

                ExecuteState::Revert(r) => {
                    return Ok(ProgramState::Revert(r));
                }

                ExecuteState::Proceed => (),

                #[cfg(feature = "debug")]
                ExecuteState::DebugEvent(d) => {
                    return Ok(ProgramState::RunProgram(d));
                }
            }
        }
    }

    pub(crate) fn run_program(&mut self) -> Result<ProgramState, InterpreterError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow));
            }

            match self.execute()? {
                ExecuteState::Return(r) => {
                    return Ok(ProgramState::Return(r));
                }

                ExecuteState::ReturnData(d) => {
                    return Ok(ProgramState::ReturnData(d));
                }

                ExecuteState::Revert(r) => {
                    return Ok(ProgramState::Revert(r));
                }

                ExecuteState::Proceed => (),

                #[cfg(feature = "debug")]
                ExecuteState::DebugEvent(d) => {
                    return Ok(ProgramState::RunProgram(d));
                }
            }
        }
    }

    /// Allocate internally a new instance of [`Interpreter`] with the provided
    /// storage, initialize it with the provided transaction and return the
    /// result of th execution in form of [`StateTransition`]
    pub fn transact_owned(
        storage: S,
        tx: CheckedTransaction,
        params: ConsensusParameters,
    ) -> Result<StateTransition, InterpreterError> {
        Interpreter::with_storage(storage, params)
            .transact(tx)
            .map(|st| st.into_owned())
    }

    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(&mut self, tx: CheckedTransaction) -> Result<StateTransitionRef<'_>, InterpreterError> {
        let state_result = self.init_script(tx).and_then(|_| self.run());

        #[cfg(feature = "profile-any")]
        self.profiler.on_transaction(&state_result);

        let state = state_result?;

        let transition = StateTransitionRef::new(state, self.transaction(), self.receipts());

        Ok(transition)
    }
}
