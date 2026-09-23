use std::env;
use std::process::ExitCode;
use std::time::Instant;

fn l1_norm_by_columns(matrix: &[u64], size: usize) -> u64 {
    let mut result: u64 = 0;
    for i in 0..size {
        let mut col_sum: u64 = 0;
        for j in 0..size {
            col_sum += matrix[j * size + i];
        }
        if col_sum > result {
            result = col_sum;
        }
    }

    result
}

fn l1_norm_by_rows(matrix: &[u64], size: usize) -> u64 {
    let mut sums = vec![0; size];
    for i in 0..size {
        for j in 0..size {
            sums[j] += matrix[i * size + j];
        }
    }

    sums.into_iter().max().unwrap_or(0 as u64)
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1); // pass program name
    let method = match args.next() {
        Some(arg) => arg,
        None => {
            println!("Argument \"rows\" or \"columns\" is needed!");
            return ExitCode::FAILURE;
        }
    };

    let f = match method.as_str() {
        "rows" => l1_norm_by_rows,
        "columns" => l1_norm_by_columns,
        _ => {
            println!("Argument must be \"rows\" or \"columns\"!");
            return ExitCode::FAILURE;
        }
    };

    let matrix_size = 4096;
    let matrix: Vec<u64> = (0..matrix_size * matrix_size).collect();

    let start = Instant::now();
    println!("{}", f(&matrix, matrix_size as usize));
    println!("Elapsed: {:?}", start.elapsed());

    ExitCode::SUCCESS
}
