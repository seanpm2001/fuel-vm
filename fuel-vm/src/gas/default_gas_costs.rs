use super::*;
/// File generated by fuel-core: benches/src/bin/collect.rs:440. With the following git hash
pub const GIT: &str = "8d16a9266c2c6945b320bfc959633c7c57cdfa10";
pub fn default_gas_costs() -> GasCostsValues {
    GasCostsValues {
        add: 1,
        addi: 1,
        aloc: 1,
        and: 1,
        andi: 1,
        bal: 21,
        bhei: 1,
        bhsh: 1,
        burn: 35,
        cb: 2,
        cfei: 1,
        cfsi: 1,
        croo: 28,
        div: 1,
        divi: 1,
        ecr: 1703,
        eq: 1,
        exp: 1,
        expi: 1,
        flag: 1,
        gm: 1,
        gt: 1,
        gtf: 1,
        ji: 1,
        jmp: 1,
        jne: 1,
        jnei: 1,
        jnzi: 1,
        jmpf: 1,
        jmpb: 1,
        jnzf: 1,
        jnzb: 1,
        jnef: 1,
        jneb: 1,
        k256: 19,
        lb: 1,
        log: 40,
        lt: 1,
        lw: 1,
        mcpi: 3,
        mint: 35,
        mlog: 1,
        srwq: DependentCost {
            base: 54,
            dep_per_unit: 2,
        },
        modi: 1,
        mod_op: 1,
        movi: 1,
        mroo: 2,
        mul: 1,
        muli: 1,
        mldv: 2,
        noop: 1,
        not: 1,
        or: 1,
        ori: 1,
        move_op: 1,
        ret: 61,
        s256: 5,
        sb: 1,
        scwq: 11,
        sll: 1,
        slli: 1,
        srl: 1,
        srli: 1,
        srw: 23,
        sub: 1,
        subi: 1,
        sw: 1,
        sww: 79,
        swwq: 72,
        time: 1,
        tr: 120,
        tro: 99,
        wdcm: 2,
        wqcm: 4,
        wdop: 2,
        wqop: 4,
        wdml: 2,
        wqml: 4,
        wddv: 4,
        wqdv: 8,
        wdmd: 5,
        wqmd: 10,
        wdam: 2,
        wqam: 4,
        wdmm: 4,
        wqmm: 8,
        xor: 1,
        xori: 1,
        call: DependentCost {
            base: 116,
            dep_per_unit: 14,
        },
        ccp: DependentCost {
            base: 24,
            dep_per_unit: 13,
        },
        csiz: DependentCost {
            base: 17,
            dep_per_unit: 15,
        },
        ldc: DependentCost {
            base: 23,
            dep_per_unit: 14,
        },
        logd: DependentCost {
            base: 46,
            dep_per_unit: 19,
        },
        mcl: DependentCost {
            base: 1,
            dep_per_unit: 2359,
        },
        mcli: DependentCost {
            base: 1,
            dep_per_unit: 2322,
        },
        mcp: DependentCost {
            base: 1,
            dep_per_unit: 1235,
        },
        meq: DependentCost {
            base: 1,
            dep_per_unit: 2343,
        },
        rvrt: 61,
        smo: DependentCost {
            base: 84,
            dep_per_unit: 13,
        },
        retd: DependentCost {
            base: 65,
            dep_per_unit: 19,
        },
        memory_page: 1,
    }
}
