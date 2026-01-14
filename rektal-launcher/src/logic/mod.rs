///the central logic module for R.E.K.T.A.L. launcher
use std::process::Command;

/// Run the R.E.K.T.A.L. application with the provided arguments
/// # Arguments
/// * `args` - A vector of string slices representing the command-line arguments to pass to R.E.K.T.A.L.
///This function constructs a command to execute the R.E.K.T.A.L. application
///with the specified arguments and prints the exit code upon completion.
pub(crate) fn run_rektal(args:Vec<&str>) -> std::io::Result<()> {
    let status = Command::new("echo")
        .args(args)
        .status()?;

    println!("R.E.K.T.A.L. Exit-Code: {}", status);
    Ok(())
}
