use std::{ env, io::Write, time::Instant };
use distaff::{ self, StarkProof };

mod examples;
use examples::{ Example };

fn main() {

    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug).init();

    // determine the example to run based on command-line inputs
    let ex: Example;
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        ex = examples::fibonacci::get_example(&args);
    }
    else {
        ex = match args[1].as_str() {
            "conditional"   => examples::conditional::get_example(&args[1..]),
            "fibonacci"     => examples::fibonacci::get_example(&args[1..]),
            "merkle"        => examples::merkle::get_example(&args[1..]),
            "rangecheck"    => examples::range::get_example(&args[1..]),
            _ => panic!("Could not find example program for '{}'", args[1])
        }
    }
    let Example { program, inputs, num_outputs, options, expected_result } = ex;
    println!("--------------------------------");

    // execute the program and generate the proof of execution
    let now = Instant::now();
    let (outputs, proof) = distaff::execute(&program, &inputs, num_outputs, &options);
    println!("--------------------------------");
    println!("Executed program with hash {} in {} ms", 
        hex::encode(program.hash()),
        now.elapsed().as_millis());
    println!("Program output: {:?}", outputs);
    assert_eq!(expected_result, outputs, "Program result was computed incorrectly");

    // serialize the proof to see how big it is
    let proof_bytes = bincode::serialize(&proof).unwrap();
    println!("Execution proof size: {} KB", proof_bytes.len() / 1024);
    println!("Execution proof security: {} bits", options.security_level(true));
    println!("--------------------------------");

    // verify that executing a program with a given hash and given inputs
    // results in the expected output
    let proof = bincode::deserialize::<StarkProof<u128>>(&proof_bytes).unwrap();
    let now = Instant::now();
    match distaff::verify(program.hash(), inputs.get_public_inputs(), &outputs, &proof) {
        Ok(_) => println!("Execution verified in {} ms", now.elapsed().as_millis()),
        Err(msg) => println!("Failed to verify execution: {}", msg)
    }
}