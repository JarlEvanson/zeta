//! Utilities to parse the configuration file, which must be a valid TOML file.

use core::error::Error;
use core::fmt::Display;
use core::{cell::OnceCell, fmt::Debug};

use digest::sha512::Digest;
use log::LevelFilter;

use crate::{config::parser::lexer::Lexer, vec::Vec};

use self::lexer::Token;
use self::strings::{MultiplexedStringIterator, StringLike};

use super::Config;

mod impls;
mod lexer;
mod strings;

use impls::*;

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct ConfigState<'config> {
    randomize_memory: OnceCell<bool>,

    // Logging table settings
    logging_declared: OnceCell<()>,

    global_log_level: OnceCell<LevelFilter>,
    framebuffer_log_level: OnceCell<LevelFilter>,
    serial_log_level: OnceCell<LevelFilter>,

    // Kernel table settings
    kernel_declared: OnceCell<()>,

    kernel_path: OnceCell<Token<'config>>,
    kernel_checksum: OnceCell<Digest>,
    loaded_modules: OnceCell<Vec<Token<'config>>>,
    kernel_args: OnceCell<Vec<Token<'config>>>,
    // Module table settings
    modules_declared: OnceCell<()>,
    modules: Vec<ModuleState<'config>>,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct ModuleState<'config> {
    name: OnceCell<Token<'config>>,
    path: OnceCell<Token<'config>>,
    checksum: OnceCell<Digest>,
    args: OnceCell<Vec<Token<'config>>>,
}

struct ConfigParser<'config> {
    current_table: Table,
    lexer: Lexer<'config>,
    toml_state: ConfigState<'config>,
}

pub fn parse_configuration_file(toml_str: &str) -> Result<Config, ParseConfigError> {
    ConfigParser::parse_configuration_file(toml_str)
}

impl<'config> ConfigParser<'config> {
    #[allow(clippy::missing_docs_in_private_items)]
    fn parse_configuration_file(toml_str: &'config str) -> Result<Config, ParseConfigError> {
        let mut parser = ConfigParser {
            current_table: Table::Global,
            lexer: Lexer::new(toml_str),
            toml_state: ConfigState::default(),
        };

        loop {
            let mut parsed_key = false;

            match parser.lexer.next() {
                Token::BareString(key) => {
                    parser.parse_key(&key).unwrap();
                    parsed_key = true;
                }
                Token::BasicString(key) => {
                    parser.parse_key(&key).unwrap();
                    parsed_key = true;
                }
                Token::Comment | Token::Whitespace | Token::Newline => {}
                Token::LeftSquareBracket => {
                    if parser
                        .lexer
                        .consume(|token| token == Token::LeftSquareBracket)
                        .is_ok()
                    {
                        parser.parse_module_header().unwrap();
                    } else {
                        parser.switch_table().unwrap();
                    }
                    parsed_key = true;
                }
                Token::Error => {
                    log::error!("lexing error occured");
                    todo!()
                }
                Token::Eof => {
                    break;
                }
                token => todo!("unhandled token: {:?}", token),
            }

            if parsed_key {
                loop {
                    let token = parser.lexer.next();

                    match token {
                        Token::Newline | Token::Eof => break,
                        Token::Whitespace | Token::Comment => {}
                        token => panic!(
                            "there must be a newline or EOF after a key-value pair: {:?}",
                            token
                        ),
                    }
                }
            }
        }

        todo!()
    }

    fn parse_module_header(&mut self) -> Result<(), ParseModuleHeaderError> {
        if self.toml_state.modules_declared.get().is_some() {
            return Err(ParseModuleHeaderError::AlreadyDeclared);
        }

        self.lexer.skip_whitespace();

        let key_token = self.lexer.next();

        let key: &dyn StringLike = match key_token {
            Token::BareString(ref key) => key,
            Token::BasicString(ref key) => key,
            token => {
                return Err(ParseModuleHeaderError::InvalidFormat(token));
            }
        };

        if !key.eq("module") {
            log::error!("{} is not a valid array of tables", key);
            return Err(ParseModuleHeaderError::InvalidTable);
        }

        log::trace!("setting up new `module` table");

        self.current_table = Table::Modules;

        if let Err(module) = self
            .toml_state
            .modules
            .push_within_capacity(ModuleState::default())
        {
            let mut new_capacity = self.toml_state.modules.capacity() * 2;
            if new_capacity == 0 {
                new_capacity = 4;
            }

            self.toml_state
                .modules
                .try_reserve(new_capacity)
                .expect("allocation error occurred");

            // PANIC SAFETY:
            // We just doubled capacity of `modules`, so it can store an extra module.
            self.toml_state
                .modules
                .push_within_capacity(module)
                .unwrap();
        }

        self.lexer.skip_whitespace();

        self.lexer
            .consume(|token| token == Token::RightSquareBracket)
            .map_err(ParseModuleHeaderError::InvalidFormat)?;
        self.lexer
            .consume(|token| token == Token::RightSquareBracket)
            .map_err(ParseModuleHeaderError::InvalidFormat)?;

        Ok(())
    }

    fn switch_table(&mut self) -> Result<(), SetTableError> {
        self.lexer.skip_whitespace();

        let key_token = self.lexer.next();

        let key: &dyn StringLike = match key_token {
            Token::BareString(ref key) => key,
            Token::BasicString(ref key) => key,
            token => {
                return Err(SetTableError::InvalidFormat(token));
            }
        };

        if key.eq("logging") {
            log::trace!("moving to the logging table");

            self.toml_state
                .logging_declared
                .set(())
                .map_err(|_| SetTableError::AlreadyDeclared(Table::Logging))?;
            self.current_table = Table::Logging;
        } else if key.eq("kernel") {
            log::trace!("moving to the kernel table");

            self.toml_state
                .kernel_declared
                .set(())
                .map_err(|_| SetTableError::AlreadyDeclared(Table::Kernel))?;
            self.current_table = Table::Kernel;
        } else {
            log::error!("`{}` is not a valid table", key);
            return Err(SetTableError::InvalidTable);
        }

        self.lexer.skip_whitespace();

        self.lexer
            .consume(|tok| tok == Token::RightSquareBracket)
            .map_err(|token| {
                if token == Token::Dot {
                    SetTableError::InvalidTable
                } else {
                    SetTableError::InvalidFormat(token)
                }
            })?;

        Ok(())
    }

    /// Multiplexer for the various table states.
    fn parse_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        match self.current_table {
            Table::Global => self.parse_global_key(key),
            Table::Logging => self.parse_logging_key(key),
            Table::Kernel => self.parse_kernel_key(key),
            Table::Modules => self.parse_module_key(key),
        }
    }

    /// Function to control parsing of the global table.
    fn parse_global_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        if key.eq("randomize_memory") {
            log::trace!("setting `randomize_memory`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.randomize_memory) {
                log::error!("error setting `randomize_memory`: {err}");
                return Err(err.into());
            }
        } else if key.eq("logging") {
            self.lexer.skip_whitespace();

            match self.lexer.next() {
                Token::Dot => {
                    let _ = self.toml_state.logging_declared.set(());

                    self.lexer.skip_whitespace();

                    match self.lexer.next() {
                        Token::BareString(key) => self.parse_logging_key(&key),
                        Token::BasicString(key) => self.parse_logging_key(&key),
                        token => return Err(SetKeyError::InvalidFormat(token)),
                    }?;
                }
                Token::Equals => {
                    todo!("implement inline tables");
                }
                token => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else if key.eq("kernel") {
            self.lexer.skip_whitespace();

            match self.lexer.next() {
                Token::Dot => {
                    let _ = self.toml_state.kernel_declared.set(());

                    self.lexer.skip_whitespace();

                    match self.lexer.next() {
                        Token::BareString(key) => self.parse_kernel_key(&key),
                        Token::BasicString(key) => self.parse_kernel_key(&key),
                        token => return Err(SetKeyError::InvalidFormat(token)),
                    }?;
                }
                Token::Equals => {
                    todo!("implement inline tables");
                }
                token => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else if key.eq("modules") {
            self.lexer.skip_whitespace();

            if self.toml_state.modules_declared.set(()).is_err() {
                log::error!("`modules` array of tables has already been declared");
                return Err(SetKeyError::AlreadySet);
            }

            match self.lexer.next() {
                Token::Equals => {
                    todo!("implement arrays");
                }
                token => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else {
            log::error!("`{}` is not a valid key for the global table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }

    /// Function to control parsing of the logging table.
    fn parse_logging_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        if key.eq("global") {
            log::trace!("setting `logging.global`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.global_log_level) {
                log::error!("error setting `logging.global`: {err}");
            }
        } else if key.eq("serial") {
            log::trace!("setting `logging.serial`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.serial_log_level) {
                log::error!("error setting `logging.serial`: {err}");
            }
        } else if key.eq("framebuffer") {
            log::trace!("setting `logging.framebuffer`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.framebuffer_log_level) {
                log::error!("error setting `logging.framebuffer`: {err}");
            }
        } else {
            log::error!("`{}` is not a valid key for the logging table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }

    fn parse_kernel_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        todo!()
    }

    fn parse_module_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseConfigError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetKeyError<'config> {
    InvalidKey,
    InvalidFormat(Token<'config>),
    InvalidValue,
    AlreadySet,
}

impl<'config, E: Error> From<SetValueError<'config, E>> for SetKeyError<'config> {
    fn from(value: SetValueError<'config, E>) -> Self {
        match value {
            SetValueError::AlreadySet => SetKeyError::AlreadySet,
            SetValueError::InvalidFormat(tok) => SetKeyError::InvalidFormat(tok),
            SetValueError::InvalidValue(_) => SetKeyError::InvalidValue,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseModuleHeaderError<'config> {
    AlreadyDeclared,
    InvalidFormat(Token<'config>),
    InvalidTable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Table {
    Global,
    Logging,
    Kernel,
    Modules,
}

impl Display for Table {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Table::Global => f.write_str("global"),
            Table::Logging => f.write_str("logging"),
            Table::Kernel => f.write_str("kernel"),
            Table::Modules => f.write_str("modules"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetTableError<'config> {
    InvalidFormat(Token<'config>),
    AlreadyDeclared(Table),
    InvalidTable,
}

impl Display for SetTableError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SetTableError::InvalidFormat(token) => {
                write!(f, "invalid table format: got {:?}", token)
            }
            SetTableError::AlreadyDeclared(table) => {
                write!(f, "{} table has already been declared", table)
            }
            SetTableError::InvalidTable => {
                write!(f, "the requested table is invalid")
            }
        }
    }
}

impl Error for SetTableError<'_> {}

pub fn set<'config, T: ParseFromLexer<'config>>(
    lexer: &mut Lexer<'config>,
    opt: &OnceCell<T>,
) -> Result<(), SetValueError<'config, T::Error>> {
    lexer.skip_whitespace();

    if let Err(token) = lexer.consume(|tok| tok == Token::Equals) {
        return Err(SetValueError::InvalidFormat(token));
    }

    lexer.skip_whitespace();

    let value = T::parse(lexer).map_err(SetValueError::InvalidValue)?;

    if opt.set(value).is_err() {
        return Err(SetValueError::AlreadySet);
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetValueError<'config, E: Error> {
    /// The format of the key-value pair was invalid.
    InvalidFormat(Token<'config>),
    /// The value provided was an invalid value for the given key.
    InvalidValue(E),
    /// The key was already set.
    AlreadySet,
}

impl<E: Error> Display for SetValueError<'_, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SetValueError::InvalidFormat(token) => {
                write!(f, "invalid key-value format: got {:?}", token)
            }
            SetValueError::InvalidValue(err) => {
                write!(f, "invalid value for associated key: {}", err)
            }
            SetValueError::AlreadySet => write!(f, "key has already been set"),
        }
    }
}

impl<E: Error> Error for SetValueError<'_, E> {}

/// Returns `true` if `ch` is a valid TOML whitespace character.
fn is_whitespace(ch: char) -> bool {
    ch == '\u{9}' || ch == '\u{20}'
}

/// Returns `true` if `s` starts with a valid TOML newline sequence.
fn is_newline(s: &str) -> bool {
    let mut chars = s.chars();

    match chars.next() {
        Some('\n') => true,
        Some('\r') if chars.next() == Some('\n') => true,
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use super::parse_configuration_file;

    const K: &str = "# Randomize the contents of RAM at bootup in order
# to find bugs related to non-zeroed memory or 
# for security reasons
# This option will slow boot times significantly
randomize_memory = false

logging.global = \"off\"
logging.\"serial\" = \"off\"
\"logging\".\"framebuffer\" = \"off\"

[kernel]
path = \"\"
loaded_modules = [\"root\", \"serial\", \"ramfs\"]
# We have no arguments as of now
args = [\"\"]

# placeholder
[[module]]
name = \"root\"
path = \"\"
args = [\"\"]

[[module]]
name = \"serial\" # Will be useful for debugging.
path = \"\"

[[module]]
name = \"ramfs\"
path = [\"\"]
";

    #[test]
    fn debugging() {
        parse_configuration_file(K).unwrap();
    }
}
