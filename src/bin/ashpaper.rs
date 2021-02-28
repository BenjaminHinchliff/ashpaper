use ashpaper_plus::Program;
use clap::{App, Arg};
use std::fs;

#[cfg(not(tarpaulin_include))]
pub fn main() {
    let matches = App::new("ashpaper")
        .version(clap::crate_version!())
        .author(clap::crate_authors!(", "))
        .about("An AshPaper interpreter that executes 'poetry'")
        .arg(
            Arg::with_name("INPUT")
                .help(".eso file to compile")
                .required_unless("syllables")
                .index(1),
        )
        .arg(
            Arg::with_name("syllables")
                .short("s")
                .long("syllables")
                .value_name("STRING")
                .help("Count number of syllables in a string and exit")
                .takes_value(true),
        )
        .get_matches();

    if let Some(syl_str) = matches.value_of("syllables") {
        println!("{}", ashpaper_plus::count_syllables(syl_str));
        return;
    }

    env_logger::init();

    let fname = matches.value_of("INPUT").unwrap();
    let contents = fs::read_to_string(fname).expect("Something went wrong reading input file!");

    let program = Program::create(&contents);
    print!("{}", program.execute());
}
