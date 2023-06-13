use clap::{arg, command, value_parser, Command};
pub mod config;
pub mod get;
pub mod init;
pub mod serve;
pub mod sql;

#[async_std::main]
async fn main() {
    // initialize configuration
    let mut config: config::Config = config::Config::new().await.unwrap();

    let level = match config.logging_level {
        config::LoggingLevel::DEBUG => tracing::Level::DEBUG,
        config::LoggingLevel::INFO => tracing::Level::INFO,
        config::LoggingLevel::WARN => tracing::Level::WARN,
        config::LoggingLevel::ERROR => tracing::Level::ERROR,
    };

    // initialize tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("init").about("Initialises things").arg(
                arg!(
                    -d --database <FILE> "Specifies a custom database name"
                )
                .required(false)
                .value_parser(value_parser!(String)),
            ),
        )
        .subcommand(Command::new("config").about("Configures things"))
        .subcommand(
            Command::new("get")
                .about("Gets things from a table")
                .arg(
                    arg!(<TABLE> "A database table")
                        .required(true)
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    arg!(-s --shape <SHAPE> "Specifies a data 'shape', e.g. value_rows")
                        .required(false)
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    arg!(-f --format <FORMAT> "Specifies an output format, e.g. json")
                        .required(false)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(Command::new("serve").about("Run HTTP server"))
        .get_matches();

    let exit_result = match matches.subcommand() {
        Some(("init", sub_matches)) => match sub_matches.get_one::<String>("database") {
            Some(x) => {
                //update config
                config.connection(x);

                init::init(&config).await
            }
            _ => init::init(&config).await,
        },
        Some(("config", _sub_matches)) => Ok(config.to_string()),
        Some(("get", sub_matches)) => {
            let table = match sub_matches.get_one::<String>("TABLE") {
                Some(x) => x,
                _ => panic!("No table given"),
            };
            let shape = match sub_matches.get_one::<String>("shape") {
                Some(x) => x,
                _ => "value_rows",
            };
            let format = match sub_matches.get_one::<String>("format") {
                Some(x) => x,
                _ => "text",
            };
            let result =
                match get::get_table(config.init().await.unwrap(), table, shape, format).await {
                    Ok(x) => x,
                    Err(x) => format!("ERROR: {:?}", x),
                };
            Ok(result)
        }
        Some(("serve", _sub_matches)) => serve::app(config.init().await.unwrap()),
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };

    //print exit message
    match exit_result {
        Err(x) => {
            println!("{}", x);
            std::process::exit(1)
        }

        Ok(x) => println!("{}", x),
    }
}
