use super::{
    internal::{
        inc_pc,
        internal_contract_or_default,
    },
    receipts::ReceiptsCtx,
    ExecutableTransaction,
    Interpreter,
    Memory,
    MemoryRange,
};
use crate::{
    constraints::reg_key::*,
    context::Context,
    error::SimpleResult,
};

use fuel_tx::Receipt;
use fuel_types::Word;

#[cfg(test)]
mod tests;

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn log(&mut self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let (SystemRegisters { fp, is, pc, .. }, _) =
            split_registers(&mut self.registers);
        let input = LogInput {
            memory: &mut self.memory,
            context: &self.context,
            receipts: &mut self.receipts,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log(a, b, c, d)
    }

    pub(crate) fn log_data(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { fp, is, pc, .. }, _) =
            split_registers(&mut self.registers);
        let input = LogInput {
            memory: &mut self.memory,
            context: &self.context,
            receipts: &mut self.receipts,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log_data(a, b, c, d)
    }
}

struct LogInput<'vm> {
    memory: &'vm mut Memory,
    context: &'vm Context,
    receipts: &'vm mut ReceiptsCtx,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
}

impl LogInput<'_> {
    pub(crate) fn log(self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let receipt = Receipt::log(
            internal_contract_or_default(self.context, self.fp, self.memory),
            a,
            b,
            c,
            d,
            *self.pc,
            *self.is,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }

    pub(crate) fn log_data(self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let range = MemoryRange::new(c, d)?;

        let receipt = Receipt::log_data(
            internal_contract_or_default(self.context, self.fp, self.memory),
            a,
            b,
            c,
            *self.pc,
            *self.is,
            self.memory[range.usizes()].to_vec(),
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}
