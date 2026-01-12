mod gui;

use std::process::Command;
use std::rc::Rc;

fn main() {
    let callback = Rc::new(|selected: &str| {
        println!("Aus main.rs: {}", selected);
        run_rektal(vec![selected]).expect("Ok");
    });

    gui::run(callback);
}


fn run_rektal(args:Vec<&str>) -> std::io::Result<()> {
    let status = Command::new("echo")
        .args(args)
        .status()?;

    println!("R.E.K.T.A.L. Exit-Code: {}", status);
    Ok(())
}
