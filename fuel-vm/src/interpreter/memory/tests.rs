use std::ops::Range;

use super::*;
use crate::checked_transaction::Checked;
use crate::prelude::*;
use fuel_asm::op;
use fuel_tx::Script;
use test_case::test_case;

#[test]
fn memcopy() {
    let mut vm = Interpreter::with_memory_storage();
    let params = ConsensusParameters::default().with_max_gas_per_tx(Word::MAX / 2);
    let tx = Transaction::script(
        0,
        params.max_gas_per_tx,
        0,
        op::ret(0x10).to_bytes().to_vec(),
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let tx = tx
        .into_checked(Default::default(), &params, vm.gas_costs())
        .expect("default tx should produce a valid checked transaction");

    vm.init_script(tx).expect("Failed to init VM");

    let alloc = 1024;

    // r[0x10] := 1024
    vm.instruction(op::addi(0x10, RegId::ZERO, alloc)).unwrap();
    vm.instruction(op::aloc(0x10)).unwrap();

    // r[0x20] := 128
    vm.instruction(op::addi(0x20, 0x20, 128)).unwrap();

    for i in 0..alloc {
        vm.instruction(op::addi(0x21, RegId::ZERO, i)).unwrap();
        vm.instruction(op::sb(RegId::HP, 0x21, (i + 1) as Immediate12)).unwrap();
    }

    // r[0x23] := m[$hp, 0x20] == m[0x12, 0x20]
    vm.instruction(op::meq(0x23, RegId::HP, 0x12, 0x20)).unwrap();

    assert_eq!(0, vm.registers()[0x23]);

    // r[0x12] := $hp + r[0x20]
    vm.instruction(op::add(0x12, RegId::HP, 0x20)).unwrap();
    vm.instruction(op::add(0x12, RegId::ONE, 0x12)).unwrap();

    // Test ownership
    vm.instruction(op::add(0x30, RegId::HP, RegId::ONE)).unwrap();
    vm.instruction(op::mcp(0x30, 0x12, 0x20)).unwrap();

    // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
    vm.instruction(op::meq(0x23, 0x30, 0x12, 0x20)).unwrap();

    assert_eq!(1, vm.registers()[0x23]);

    // Assert ownership
    vm.instruction(op::subi(0x24, RegId::HP, 1)).unwrap();
    let ownership_violated = vm.instruction(op::mcp(0x24, 0x12, 0x20));

    assert!(ownership_violated.is_err());

    // Assert no panic on overlapping
    vm.instruction(op::subi(0x25, 0x12, 1)).unwrap();
    let overlapping = vm.instruction(op::mcp(RegId::HP, 0x25, 0x20));

    assert!(overlapping.is_err());
}

#[test]
fn memrange() {
    let m = MemoryRange::from(..1024);
    let m_p = MemoryRange::new(0, 1024);
    assert_eq!(m, m_p);

    let mut vm = Interpreter::with_memory_storage();
    vm.init_script(Checked::<Script>::default()).expect("Failed to init VM");

    let bytes = 1024;
    vm.instruction(op::addi(0x10, RegId::ZERO, bytes as Immediate12))
        .unwrap();
    vm.instruction(op::aloc(0x10)).unwrap();

    let m = MemoryRange::new(vm.registers()[RegId::HP], bytes);
    assert!(!vm.ownership_registers().has_ownership_range(&m));

    let m = MemoryRange::new(vm.registers()[RegId::HP] + 1, bytes);
    assert!(vm.ownership_registers().has_ownership_range(&m));

    let m = MemoryRange::new(vm.registers()[RegId::HP] + 1, bytes + 1);
    assert!(!vm.ownership_registers().has_ownership_range(&m));

    let m = MemoryRange::new(0, bytes).to_heap(&vm);
    assert!(vm.ownership_registers().has_ownership_range(&m));

    let m = MemoryRange::new(0, bytes + 1).to_heap(&vm);
    assert!(!vm.ownership_registers().has_ownership_range(&m));
}

#[test]
fn stack_alloc_ownership() {
    let mut vm = Interpreter::with_memory_storage();

    vm.init_script(Checked::<Script>::default()).expect("Failed to init VM");

    vm.instruction(op::move_(0x10, RegId::SP)).unwrap();
    vm.instruction(op::cfei(2)).unwrap();

    // Assert allocated stack is writable
    vm.instruction(op::mcli(0x10, 2)).unwrap();
}

#[test_case(
    OwnershipRegisters::test(0..0, 0..0, Context::Call{ block_height: 0}), 0..0
    => false ; "empty mem range"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 0..0
    => false ; "empty mem range (external)"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0, Context::Call{ block_height: 0}), 0..1
    => false ; "empty stack and heap"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 0..1
    => false ; "empty stack and heap (external)"
)]
#[test_case(
    OwnershipRegisters::test(0..1, 0..0, Context::Call{ block_height: 0}), 0..1
    => true ; "in range for stack"
)]
#[test_case(
    OwnershipRegisters::test(0..1, 0..0, Context::Call{ block_height: 0}), 0..2
    => false; "above stack range"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..2, Context::Call{ block_height: 0}), 1..2
    => true ; "in range for heap"
)]
#[test_case(
    OwnershipRegisters::test(0..2, 1..2, Context::Call{ block_height: 0}), 0..2
    => true ; "crosses stack and heap"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 1..2
    => true ; "in heap range (external)"
)]
#[test_case(
    OwnershipRegisters::test(0..19, 31..100, Context::Script{ block_height: 0}), 20..30
    => false; "between ranges (external)"
)]
#[test_case(
    OwnershipRegisters::test(0..19, 31..100, Context::Script{ block_height: 0}), 0..1
    => true; "in stack range (external)"
)]
fn test_ownership(reg: OwnershipRegisters, range: Range<u64>) -> bool {
    let range = MemoryRange::new(range.start, range.end - range.start);
    reg.has_ownership_range(&range)
}

fn set_index(index: usize, val: u8, mut array: [u8; 100]) -> [u8; 100] {
    array[index] = val;
    array
}

#[test_case(
    1, &[],
    OwnershipRegisters::test(0..1, 100..100, Context::Script{ block_height: 0})
    => (false, [0u8; 100]); "External errors when write is empty"
)]
#[test_case(
    1, &[],
    OwnershipRegisters::test(0..1, 100..100, Context::Call{ block_height: 0})
    => (false, [0u8; 100]); "Internal errors when write is empty"
)]
#[test_case(
    1, &[2],
    OwnershipRegisters::test(0..2, 100..100, Context::Script{ block_height: 0})
    => (true, set_index(1, 2, [0u8; 100])); "External writes to stack"
)]
#[test_case(
    98, &[2],
    OwnershipRegisters::test(0..2, 97..100, Context::Script{ block_height: 0})
    => (true, set_index(98, 2, [0u8; 100])); "External writes to heap"
)]
#[test_case(
    1, &[2],
    OwnershipRegisters::test(0..2, 100..100, Context::Call { block_height: 0})
    => (true, set_index(1, 2, [0u8; 100])); "Internal writes to stack"
)]
#[test_case(
    98, &[2],
    OwnershipRegisters::test(0..2, 97..100, Context::Call { block_height: 0})
    => (true, set_index(98, 2, [0u8; 100])); "Internal writes to heap"
)]
#[test_case(
    1, &[2; 50],
    OwnershipRegisters::test(0..40, 100..100, Context::Script{ block_height: 0})
    => (false, [0u8; 100]); "External too large for stack"
)]
#[test_case(
    1, &[2; 50],
    OwnershipRegisters::test(0..40, 100..100, Context::Call{ block_height: 0})
    => (false, [0u8; 100]); "Internal too large for stack"
)]
#[test_case(
    61, &[2; 50],
    OwnershipRegisters::test(0..0, 60..100, Context::Call{ block_height: 0})
    => (false, [0u8; 100]); "Internal too large for heap"
)]
fn test_mem_write(addr: usize, data: &[u8], registers: OwnershipRegisters) -> (bool, [u8; 100]) {
    let mut memory: Memory<MEM_SIZE> = vec![0u8; MEM_SIZE].try_into().unwrap();
    let r = try_mem_write(addr, data, registers, &mut memory).is_ok();
    let memory: [u8; 100] = memory[..100].try_into().unwrap();
    (r, memory)
}

#[test_case(
    1, 0,
    OwnershipRegisters::test(0..1, 100..100, Context::Script{ block_height: 0})
    => (false, [1u8; 100]); "External errors when write is empty"
)]
#[test_case(
    1, 0,
    OwnershipRegisters::test(0..1, 100..100, Context::Call{ block_height: 0})
    => (false, [1u8; 100]); "Internal errors when write is empty"
)]
#[test_case(
    1, 1,
    OwnershipRegisters::test(0..2, 100..100, Context::Script{ block_height: 0})
    => (true, set_index(1, 0, [1u8; 100])); "External writes to stack"
)]
#[test_case(
    98, 1,
    OwnershipRegisters::test(0..2, 97..100, Context::Script{ block_height: 0})
    => (true, set_index(98, 0, [1u8; 100])); "External writes to heap"
)]
#[test_case(
    1, 1,
    OwnershipRegisters::test(0..2, 100..100, Context::Call { block_height: 0})
    => (true, set_index(1, 0, [1u8; 100])); "Internal writes to stack"
)]
#[test_case(
    98, 1,
    OwnershipRegisters::test(0..2, 97..100, Context::Call { block_height: 0})
    => (true, set_index(98, 0, [1u8; 100])); "Internal writes to heap"
)]
#[test_case(
    1, 50,
    OwnershipRegisters::test(0..40, 100..100, Context::Script{ block_height: 0})
    => (false, [1u8; 100]); "External too large for stack"
)]
#[test_case(
    1, 50,
    OwnershipRegisters::test(0..40, 100..100, Context::Call{ block_height: 0})
    => (false, [1u8; 100]); "Internal too large for stack"
)]
#[test_case(
    61, 50,
    OwnershipRegisters::test(0..0, 60..100, Context::Call{ block_height: 0})
    => (false, [1u8; 100]); "Internal too large for heap"
)]
fn test_try_zeroize(addr: usize, len: usize, registers: OwnershipRegisters) -> (bool, [u8; 100]) {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let r = try_zeroize(addr, len, registers, &mut memory).is_ok();
    let memory: [u8; 100] = memory[..100].try_into().unwrap();
    (r, memory)
}