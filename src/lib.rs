use log::debug;
use std::ops::Range;
use std::time::Instant;

#[cfg(test)]
mod tests;

// RE-EXPORTS
// ================================================================================================
pub mod crypto;
pub mod math;
pub mod utils;

mod stark;
pub use stark::{ StarkProof, ProofOptions };

mod processor;
pub use processor::{ OpCode, OpHint };

mod programs;
pub use programs::{ Program, ProgramInputs, assembly, blocks };

// EXECUTOR
// ================================================================================================

/// Executes the specified `program` and returns the result together with program hash
/// and STARK-based proof of execution.
/// 
/// * `inputs` specifies the initial stack state and provides secret input tapes;
/// * `num_outputs` specifies the number of elements from the top of the stack to be returned;
pub fn execute(program: &Program, inputs: &ProgramInputs, num_outputs: usize, options: &ProofOptions) -> (Vec<u128>, StarkProof)
{
    assert!(num_outputs <= MAX_OUTPUTS, 
        "cannot produce more than {} outputs, but requested {}", MAX_OUTPUTS, num_outputs);

    let proc_index = 0; // TODO

    // execute the program to create an execution trace
    let now = Instant::now();
    let (trace, ctx_depth, loop_depth) = processor::execute(program, proc_index, inputs);
    let mut trace = stark::TraceTable::new(trace, ctx_depth, loop_depth, options.extension_factor());
    debug!("Generated execution trace of {} registers and {} steps in {} ms",
        trace.register_count(),
        trace.unextended_length(),
        now.elapsed().as_millis());

    // copy the user stack state the the last step to return as output
    let last_state = trace.get_state(trace.unextended_length() - 1);
    let outputs = last_state.user_stack()[..num_outputs].to_vec();

    // generate STARK proof
    let mut proof = stark::prove(&mut trace, inputs.get_public_inputs(), &outputs, options);

    // build Merkle authentication path for procedure within the program and attach it to the proof
    let mut execution_path_hash = [0u128; ACC_STATE_RATE];
    execution_path_hash.copy_from_slice(&trace.get_program_hash());
    let proc_path = program.get_proc_path(proc_index);
    proof.set_proc_path(proc_path, proc_index);

    return (outputs, proof);
}

// VERIFIER
// ================================================================================================

/// Verifies that if a program with the specified `program_hash` is executed with the 
/// provided `public_inputs` and some secret inputs, the result is equal to the `outputs`.
pub fn verify(program_hash: &[u8; 32], public_inputs: &[u128], outputs: &[u128], proof: &StarkProof) -> Result<bool, String>
{
    return stark::verify(program_hash, public_inputs, outputs, proof);
}

// GLOBAL CONSTANTS
// ================================================================================================

const MIN_TRACE_LENGTH      : usize = 16;
const MAX_REGISTER_COUNT    : usize = 128;
const MAX_CONTEXT_DEPTH     : usize = 16;
const MAX_LOOP_DEPTH        : usize = 8;
const MIN_EXTENSION_FACTOR  : usize = 16;

// HASH OPERATION
// ------------------------------------------------------------------------------------------------
const HASH_STATE_RATE       : usize = 4;
const HASH_STATE_CAPACITY   : usize = 2;
const HASH_STATE_WIDTH      : usize = HASH_STATE_RATE + HASH_STATE_CAPACITY;
const HASH_CYCLE_LENGTH     : usize = 16;
const HASH_NUM_ROUNDS       : usize = 10;
const HASH_DIGEST_SIZE      : usize = 2;

// OPERATION SPONGE
// ------------------------------------------------------------------------------------------------
const ACC_STATE_RATE        : usize = 2;
const SPONGE_WIDTH          : usize = 4;
const SPONGE_CYCLE_LENGTH   : usize = 16;

// DECODER LAYOUT
// ------------------------------------------------------------------------------------------------
//
// ╒═════ sponge ══════╕╒═══ cf_ops ══╕╒═══════ ld_ops ═══════╕╒═ hd_ops ╕╒═ ctx ══╕╒═ loop ═╕
//   0    1    2    3    4    5    6    7    8    9    10   11   12   13   14   ..   ..   ..
// ├────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┤

const NUM_CF_OP_BITS        : usize = 3;
const NUM_LD_OP_BITS        : usize = 5;
const NUM_HD_OP_BITS        : usize = 2;

const NUM_CF_OPS            : usize = 8;
const NUM_LD_OPS            : usize = 32;
const NUM_HD_OPS            : usize = 4;

const SPONGE_RANGE          : Range<usize> = Range { start:  0, end:  4 };
const CF_OP_BITS_RANGE      : Range<usize> = Range { start:  4, end:  7 };
const LD_OP_BITS_RANGE      : Range<usize> = Range { start:  7, end: 12 };
const HD_OP_BITS_RANGE      : Range<usize> = Range { start: 12, end: 14 };

// STACK LAYOUT
// ------------------------------------------------------------------------------------------------
//
// ╒═══════════════════ user registers ════════════════════════╕
//    0      1    2    .................................    31
// ├─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┤

pub const MAX_PUBLIC_INPUTS : usize = 8;
pub const MAX_OUTPUTS       : usize = 8;
const MIN_STACK_DEPTH       : usize = 8;
const MAX_STACK_DEPTH       : usize = 32;