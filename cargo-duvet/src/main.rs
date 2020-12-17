use anyhow::Result;

mod manifest;
mod process;
mod project;
mod test;

fn main() -> Result<()> {
    let project = project::Builder::default().build()?;
    let tests = test::list::List::from_project(&project)?;

    tests.run(|test| {
        let mut export: duvet::export::Export = test.run(&project)?;
        export.trim();
        dbg!(export);
        Ok(())
    })?;

    Ok(())
}

#[cfg(test)]
macro_rules! hello {
    () => {
        eprintln!("hello")
    };
}

#[cfg(test)]
fn hello() {
    hello!();
}

#[test]
fn hello_test() {
    hello();
    hello!();
}
