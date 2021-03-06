use ashpaper_plus::Program;
use clap::{App, Arg, ArgMatches};
use std::fs;

#[cfg(feature = "jit")]
fn conditional_jit_arg<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name("jit")
            .short("j")
            .long("jit")
            .help("Enable high performace jit compilation with cranelift (disables debugging)"),
    )
}

#[cfg(not(feature = "jit"))]
fn conditional_jit_arg<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app
}

#[cfg(feature = "jit")]
fn execute_program(matches: &ArgMatches, program: &Program) {
    if matches.is_present("jit") {
        println!("jit executing");
        if let Err(err) = program.jit_execute() {
            eprintln!("{}", err);
        }
    } else {
        println!("executing");
        print!("{}", program.execute());
    }
}

#[cfg(not(feature = "jit"))]
fn execute_program(_matches: &ArgMatches, program: &Program) {
    println!("executing");
    print!("{}", program.execute())
}

#[cfg(not(tarpaulin_include))]
pub fn main() {
    let app = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!(", "))
        .about(clap::crate_description!())
        .args(&[
            Arg::with_name("INPUT")
                .help(".eso file to compile")
                .required_unless("syllables")
                .index(1),
            Arg::with_name("syllables")
                .short("s")
                .long("syllables")
                .value_name("STRING")
                .help("Count number of syllables in a string and exit")
                .takes_value(true),
        ]);

    let app = conditional_jit_arg(app);

    let matches = app.get_matches();

    if let Some(syl_str) = matches.value_of("syllables") {
        println!("{}", ashpaper_plus::count_syllables(syl_str));
        return;
    }

    env_logger::init();

    let fname = matches.value_of("INPUT").unwrap();
    let contents = fs::read_to_string(fname).expect("Something went wrong reading input file!");

    let program = Program::create(&contents);
    execute_program(&matches, &program);
}
