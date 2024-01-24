use indexmap::map::IndexMap;
use ontodev_valve::Valve;
use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeValue;
use sqlx::any::AnyPool;
use std::{fmt, fs, path::Path};
use toml;

#[derive(Clone, Debug)]
pub struct Config {
    pub config_version: u16,
    pub port: u16,
    pub logging_level: LoggingLevel,
    pub valve: Valve,
    pub create_only: bool,
    pub connection: String,
    pub pool: AnyPool,
    pub asset_path: Option<String>,
    pub template_path: Option<String>,
    pub actions: IndexMap<String, ActionConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub enum LoggingLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl Default for LoggingLevel {
    fn default() -> LoggingLevel {
        LoggingLevel::WARN
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TomlConfig {
    pub nanobot: NanobotConfig,
    pub logging: Option<LoggingConfig>,
    pub database: Option<DatabaseConfig>,
    pub valve: Option<ValveTomlConfig>,
    pub assets: Option<AssetsConfig>,
    pub templates: Option<TemplatesConfig>,
    pub actions: Option<IndexMap<String, ActionConfig>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NanobotConfig {
    pub config_version: u16,
    pub port: Option<u16>,
}

impl Default for NanobotConfig {
    fn default() -> NanobotConfig {
        NanobotConfig {
            config_version: 1,
            port: Some(3000),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: Option<LoggingLevel>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub connection: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> DatabaseConfig {
        DatabaseConfig {
            connection: Some(".nanobot.db".into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValveTomlConfig {
    pub path: Option<String>,
}

impl Default for ValveTomlConfig {
    fn default() -> ValveTomlConfig {
        ValveTomlConfig {
            path: Some("src/schema/table.tsv".into()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AssetsConfig {
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TemplatesConfig {
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ActionConfig {
    pub label: String,
    pub inputs: Option<Vec<InputConfig>>,
    pub commands: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InputConfig {
    pub name: String,
    pub label: String,
    pub value: Option<String>,
    pub default: Option<String>,
    pub placeholder: Option<String>,
    pub test: Option<String>,
}

#[derive(Debug)]
pub enum NanobotError {
    GeneralError(String),
    ValveError(ontodev_valve::ValveError),
    TomlError(toml::de::Error),
}

impl From<ontodev_valve::ValveError> for NanobotError {
    fn from(e: ontodev_valve::ValveError) -> Self {
        Self::ValveError(e)
    }
}

impl From<toml::de::Error> for NanobotError {
    fn from(e: toml::de::Error) -> Self {
        Self::TomlError(e)
    }
}

pub type SerdeMap = serde_json::Map<String, SerdeValue>;

pub const DEFAULT_TOML: &str = "[nanobot]
config_version = 1";

impl Config {
    pub async fn new() -> Result<Config, NanobotError> {
        let user_config_file = match fs::read_to_string("nanobot.toml") {
            Ok(x) => x,
            Err(_) => DEFAULT_TOML.into(),
        };
        let user: TomlConfig = toml::from_str(user_config_file.as_str())?;
        let connection = user
            .database
            .unwrap_or_default()
            .connection
            .unwrap_or(".nanobot.db".into());
        let valve_path = user
            .valve
            .unwrap_or_default()
            .path
            .unwrap_or("src/schema/table.tsv".into());
        let valve = Valve::build(&valve_path, &connection, false, false).await?;
        let pool = valve.pool.clone();

        let config = Config {
            config_version: user.nanobot.config_version,
            port: user.nanobot.port.unwrap_or(3000),
            logging_level: user.logging.unwrap_or_default().level.unwrap_or_default(),
            valve: valve,
            create_only: false,
            connection: connection,
            pool: pool,
            asset_path: {
                match user.assets.unwrap_or_default().path {
                    Some(p) => {
                        if Path::new(&p).is_dir() {
                            Some(p)
                        } else {
                            eprintln!(
                                "WARNING: Configuration specifies an assets directory \
                                '{}' but it does not exist.",
                                p
                            );
                            None
                        }
                    }
                    None => None,
                }
            },
            template_path: {
                match user.templates.unwrap_or_default().path {
                    Some(p) => {
                        if Path::new(&p).is_dir() {
                            Some(p)
                        } else {
                            eprintln!(
                                "WARNING: Configuration specifies a template directory \
                                '{}' but it does not exist. Using default templates.",
                                p
                            );
                            None
                        }
                    }
                    None => None,
                }
            },
            actions: user.actions.unwrap_or_default(),
        };

        Ok(config)
    }

    pub fn connection<S: Into<String>>(&mut self, connection: S) -> &mut Config {
        self.connection = connection.into();
        self
    }

    pub fn create_only(&mut self, value: bool) -> &mut Config {
        self.create_only = value;
        self
    }

    pub fn initial_load(&mut self, value: bool) -> &mut Config {
        self.valve.initial_load = value;
        self
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", toml::to_string(&to_toml(&self)).unwrap())
    }
}

pub fn to_toml(config: &Config) -> TomlConfig {
    TomlConfig {
        nanobot: NanobotConfig {
            config_version: config.config_version.clone(),
            port: Some(config.port.clone()),
        },
        logging: Some(LoggingConfig {
            level: Some(config.logging_level.clone()),
        }),
        database: Some(DatabaseConfig {
            connection: Some(config.connection.clone()),
        }),
        valve: Some(ValveTomlConfig {
            path: Some(config.valve.get_path()),
        }),
        assets: Some(AssetsConfig {
            path: config.asset_path.clone(),
        }),
        templates: Some(TemplatesConfig {
            path: config.template_path.clone(),
        }),
        actions: Some(config.actions.clone()),
    }
}
