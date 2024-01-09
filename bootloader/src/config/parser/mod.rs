//! Utilities to parse the configuration file, which must be a valid TOML file.

use core::error::Error;
use core::fmt::Display;
use core::{cell::OnceCell, fmt::Debug};

use digest::sha512::Digest;
use log::LevelFilter;

use crate::arena::Frame;
use crate::config::{Kernel, LoggingFilters, Module, StringStorage};
use crate::DEFAULT_LOGGING_LEVEL;
use crate::{config::parser::lexer::Lexer, vec::Vec};

use self::lexer::{Token, TokenKind};
use self::strings::{MultiplexedStringIterator, StringLike};

use super::{Config, Init, PathStorage};

mod impls;
mod lexer;
mod strings;

use impls::ParseFromLexer;

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct ConfigState<'config> {
    randomize_memory: OnceCell<bool>,

    // Logging table settings
    logging_declared: OnceCell<()>,
    logging: LoggingState,

    // Kernel table settings
    kernel_declared: OnceCell<()>,
    kernel: KernelState<'config>,

    // Init table settings
    init_declared: OnceCell<()>,
    init: InitState<'config>,

    // Module table settings
    modules_declared: OnceCell<()>,
    modules: Vec<ModuleState<'config>>,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Debug, Default)]
struct LoggingState {
    global: OnceCell<LevelFilter>,
    framebuffer: OnceCell<LevelFilter>,
    serial: OnceCell<LevelFilter>,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct KernelState<'config> {
    path: OnceCell<MultiplexedStringIterator<'config>>,
    checksum: OnceCell<Digest>,
    args: OnceCell<Vec<MultiplexedStringIterator<'config>>>,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct InitState<'config> {
    path: OnceCell<MultiplexedStringIterator<'config>>,
    checksum: OnceCell<Digest>,
    args: OnceCell<Vec<MultiplexedStringIterator<'config>>>,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Default)]
struct ModuleState<'config> {
    path: OnceCell<MultiplexedStringIterator<'config>>,
    checksum: OnceCell<Digest>,
    args: OnceCell<Vec<MultiplexedStringIterator<'config>>>,
}

#[allow(clippy::missing_docs_in_private_items)]
struct ConfigParser<'config> {
    current_table: Table,
    lexer: Lexer<'config>,
    toml_state: ConfigState<'config>,
}

/// Parses [`Config`] from `toml_str`.
pub fn parse_configuration_file(
    toml_str: &str,
    frame: &mut Frame,
) -> Result<Config, ParseConfigError> {
    let mut table = ConfigParser::parse_configuration_file(toml_str, frame)?;
    let mut strings = StringStorage::new();
    let mut paths = PathStorage::new();

    let randomize_memory = table.randomize_memory.get().copied().unwrap_or(false);

    let logging = convert_logging(&table.logging);

    let kernel = convert_kernel(&mut table.kernel, &mut paths, &mut strings)?;
    let init = convert_init(&mut table.init, &mut paths, &mut strings)?;

    let mut modules = Vec::with_capacity(table.modules.len()).unwrap();

    for parsed_module in table.modules.as_slice() {
        let path = if let Some(path) = parsed_module.path.get() {
            if path.clone().count() == 0 {
                return Err(ParseConfigError::UnsetMustSet);
            }
            paths.add_path_from_chars(path.clone()).unwrap()
        } else {
            log::error!("`module.path` must be set");

            return Err(ParseConfigError::UnsetMustSet);
        };

        let Some(checksum) = parsed_module.checksum.get().copied() else {
            log::error!("`module.checksum` must be set");

            return Err(ParseConfigError::UnsetMustSet);
        };

        let parsed_args = if let Some(args) = table.kernel.args.get_mut() {
            let mut tmp = Vec::new();

            core::mem::swap(&mut tmp, args);

            tmp
        } else {
            Vec::new()
        };

        let mut args = Vec::with_capacity(parsed_args.len()).unwrap();

        for arg in parsed_args.as_slice() {
            args.push_within_capacity(strings.add_str_from_chars(arg.clone()).unwrap())
                .unwrap();
        }

        let module = Module {
            path,
            checksum,
            args,
        };

        modules.push_within_capacity(module).unwrap();
    }

    let config = Config {
        randomize_memory,
        logging,
        kernel,
        init,
        modules,
        strings,
        paths,
    };

    Ok(config)
}

/// Converts a [`LoggingState`] into a [`LoggingFilters`].
fn convert_logging(state: &LoggingState) -> LoggingFilters {
    let global = state.global.get().copied().unwrap_or(DEFAULT_LOGGING_LEVEL);
    let serial = state.serial.get().copied().unwrap_or(DEFAULT_LOGGING_LEVEL);
    let framebuffer = state
        .framebuffer
        .get()
        .copied()
        .unwrap_or(DEFAULT_LOGGING_LEVEL);

    LoggingFilters {
        global,
        serial,
        framebuffer,
    }
}

/// Converts a [`KernelState`] into a [`Kernel`].
fn convert_kernel(
    kernel: &mut KernelState,
    paths: &mut PathStorage,
    strings: &mut StringStorage,
) -> Result<Kernel, ParseConfigError> {
    let path = if let Some(path) = kernel.path.get() {
        if path.clone().count() == 0 {
            return Err(ParseConfigError::UnsetMustSet);
        }

        paths.add_path_from_chars(path.clone()).unwrap()
    } else {
        log::error!("`kernel.path` must be set");

        return Err(ParseConfigError::UnsetMustSet);
    };

    let Some(checksum) = kernel.checksum.get().copied() else {
        log::error!("`kernel.path` must be set");

        return Err(ParseConfigError::UnsetMustSet);
    };

    let parsed_args = if let Some(args) = kernel.args.get_mut() {
        let mut tmp = Vec::new();

        core::mem::swap(&mut tmp, args);

        tmp
    } else {
        Vec::new()
    };

    let mut args = Vec::with_capacity(parsed_args.len()).unwrap();

    for arg in parsed_args.as_slice() {
        args.push_within_capacity(strings.add_str_from_chars(arg.clone()).unwrap())
            .unwrap();
    }

    let kernel = Kernel {
        path,
        checksum,
        args,
    };

    Ok(kernel)
}

/// Converts a [`InitState`] into a [`Init`].
fn convert_init(
    init: &mut InitState,
    paths: &mut PathStorage,
    strings: &mut StringStorage,
) -> Result<Init, ParseConfigError> {
    let path = if let Some(path) = init.path.get() {
        if path.clone().count() == 0 {
            return Err(ParseConfigError::UnsetMustSet);
        }

        paths.add_path_from_chars(path.clone()).unwrap()
    } else {
        log::error!("`kernel.path` must be set");

        return Err(ParseConfigError::UnsetMustSet);
    };

    let Some(checksum) = init.checksum.get().copied() else {
        log::error!("`kernel.path` must be set");

        return Err(ParseConfigError::UnsetMustSet);
    };

    let parsed_args = if let Some(args) = init.args.get_mut() {
        let mut tmp = Vec::new();

        core::mem::swap(&mut tmp, args);

        tmp
    } else {
        Vec::new()
    };

    let mut args = Vec::with_capacity(parsed_args.len()).unwrap();

    for arg in parsed_args.as_slice() {
        args.push_within_capacity(strings.add_str_from_chars(arg.clone()).unwrap())
            .unwrap();
    }

    let init = Init {
        path,
        checksum,
        args,
    };

    Ok(init)
}

impl<'config> ConfigParser<'config> {
    #[allow(clippy::missing_docs_in_private_items)]
    fn parse_configuration_file(
        toml_str: &'config str,
        frame: &mut Frame,
    ) -> Result<ConfigState<'config>, ParseConfigError> {
        let mut parser = ConfigParser {
            current_table: Table::Global,
            lexer: Lexer::new(toml_str),
            toml_state: ConfigState::default(),
        };

        loop {
            let mut parsed_key = false;

            let token = parser.lexer.next();

            match token.kind {
                TokenKind::BareString(key) => {
                    parser.parse_key(&key).unwrap();
                    parsed_key = true;
                }
                TokenKind::BasicString(key) => {
                    parser.parse_key(&key).unwrap();
                    parsed_key = true;
                }
                TokenKind::Comment | TokenKind::Whitespace | TokenKind::Newline => {}
                TokenKind::LeftSquareBracket => {
                    if parser.lexer.consume(TokenKind::LeftSquareBracket).is_ok() {
                        parser.parse_module_header().unwrap();
                    } else {
                        parser.switch_table().unwrap();
                    }
                    parsed_key = true;
                }

                TokenKind::Eof => {
                    break;
                }
                TokenKind::LexingError => {
                    log::error!("lexing error occurred");
                    return Err(ParseConfigError::LexingError);
                }
                token => {
                    log::error!("unexpected token: {token:?}");
                    todo!()
                }
            }

            if parsed_key {
                loop {
                    let token = parser.lexer.next();

                    match token.kind {
                        TokenKind::Newline | TokenKind::Eof => {
                            break;
                        }
                        TokenKind::Whitespace | TokenKind::Comment => {}
                        token => panic!(
                            "there must be a newline or EOF after a key-value pair: {:?}",
                            token
                        ),
                    }
                }
            }
        }

        Ok(parser.toml_state)
    }

    /// Parse a module declaration and sets up a new [`ModuleState`] to be modified.
    fn parse_module_header(&mut self) -> Result<(), ParseModuleHeaderError> {
        if self.toml_state.modules_declared.get().is_some() {
            return Err(ParseModuleHeaderError::AlreadyDeclared);
        }

        self.lexer.skip_whitespace();

        let key_token = self.lexer.next();

        let key: &dyn StringLike = match key_token.kind {
            TokenKind::BareString(ref key) => key,
            TokenKind::BasicString(ref key) => key,
            _ => {
                return Err(ParseModuleHeaderError::InvalidFormat(key_token));
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
            .consume(TokenKind::RightSquareBracket)
            .map_err(ParseModuleHeaderError::InvalidFormat)?;
        self.lexer
            .consume(TokenKind::RightSquareBracket)
            .map_err(ParseModuleHeaderError::InvalidFormat)?;

        Ok(())
    }

    /// Switchs the table. Assumes `self.current_table` is [`Table::Global`].
    fn switch_table(&mut self) -> Result<(), SetTableError> {
        self.lexer.skip_whitespace();

        let key_token = self.lexer.next();

        let key: &dyn StringLike = match key_token.kind {
            TokenKind::BareString(ref key) => key,
            TokenKind::BasicString(ref key) => key,
            _ => {
                return Err(SetTableError::InvalidFormat(key_token));
            }
        };

        if key.eq("logging") {
            log::trace!("moving to the logging table");

            self.toml_state
                .logging_declared
                .set(())
                .map_err(|()| SetTableError::AlreadyDeclared(Table::Logging))?;
            self.current_table = Table::Logging;
        } else if key.eq("kernel") {
            log::trace!("moving to the kernel table");

            self.toml_state
                .kernel_declared
                .set(())
                .map_err(|()| SetTableError::AlreadyDeclared(Table::Kernel))?;
            self.current_table = Table::Kernel;
        } else if key.eq("init") {
            log::trace!("moving to the init table");

            self.toml_state
                .init_declared
                .set(())
                .map_err(|()| SetTableError::AlreadyDeclared(Table::Kernel))?;
            self.current_table = Table::Init;
        } else {
            log::error!("`{}` is not a valid table", key);
            return Err(SetTableError::InvalidTable);
        }

        self.lexer.skip_whitespace();

        self.lexer
            .consume(TokenKind::RightSquareBracket)
            .map_err(|token| {
                if token.kind == TokenKind::Dot {
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
            Table::Init => self.parse_init_key(key),
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

            let token = self.lexer.next();

            match token.kind {
                TokenKind::Dot => {
                    let _ = self.toml_state.logging_declared.set(());

                    self.lexer.skip_whitespace();

                    let token = self.lexer.next();

                    match token.kind {
                        TokenKind::BareString(key) => self.parse_logging_key(&key),
                        TokenKind::BasicString(key) => self.parse_logging_key(&key),
                        _ => return Err(SetKeyError::InvalidFormat(token)),
                    }?;
                }
                TokenKind::Equals => {
                    todo!("implement inline tables");
                }
                _ => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else if key.eq("kernel") {
            self.lexer.skip_whitespace();

            let token = self.lexer.next();

            match token.kind {
                TokenKind::Dot => {
                    let _ = self.toml_state.kernel_declared.set(());

                    let token = self.lexer.next();

                    match token.kind {
                        TokenKind::BareString(key) => self.parse_kernel_key(&key),
                        TokenKind::BasicString(key) => self.parse_kernel_key(&key),
                        _ => return Err(SetKeyError::InvalidFormat(token)),
                    }?;
                }
                TokenKind::Equals => {
                    todo!("implement inline tables");
                }
                _ => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else if key.eq("init") {
            self.lexer.skip_whitespace();

            let token = self.lexer.next();

            match token.kind {
                TokenKind::Dot => {
                    let _ = self.toml_state.init_declared.set(());

                    let token = self.lexer.next();

                    match token.kind {
                        TokenKind::BareString(key) => self.parse_init_key(&key),
                        TokenKind::BasicString(key) => self.parse_init_key(&key),
                        _ => return Err(SetKeyError::InvalidFormat(token)),
                    }?;
                }
                TokenKind::Equals => {
                    todo!("implement inline tables");
                }
                _ => return Err(SetKeyError::InvalidFormat(token)),
            }
        } else if key.eq("modules") {
            self.lexer.skip_whitespace();

            if self.toml_state.modules_declared.set(()).is_err() {
                log::error!("`modules` array of tables has already been declared");
                return Err(SetKeyError::AlreadySet);
            }

            let token = self.lexer.next();

            match token.kind {
                TokenKind::Equals => {
                    todo!("implement arrays");
                }
                _ => return Err(SetKeyError::InvalidFormat(token)),
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

            if let Err(err) = set(&mut self.lexer, &self.toml_state.logging.global) {
                log::error!("error setting `logging.global`: {err}");
                return Err(err.into());
            }
        } else if key.eq("serial") {
            log::trace!("setting `logging.serial`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.logging.serial) {
                log::error!("error setting `logging.serial`: {err}");
                return Err(err.into());
            }
        } else if key.eq("framebuffer") {
            log::trace!("setting `logging.framebuffer`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.logging.framebuffer) {
                log::error!("error setting `logging.framebuffer`: {err}");
                return Err(err.into());
            }
        } else {
            log::error!("`{}` is not a valid key for the logging table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }

    /// Function to control parsing of the kernel table.
    fn parse_kernel_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        if key.eq("path") {
            log::trace!("setting `kernel.path`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.kernel.path) {
                log::error!("error setting `kernel.path`: {err}");
                return Err(err.into());
            }
        } else if key.eq("checksum") {
            log::trace!("setting `kernel.checksum`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.kernel.checksum) {
                log::error!("error setting `kernel.checksum`: {err}");
                return Err(err.into());
            }
        } else if key.eq("args") {
            log::trace!("setting `kernel.args`");

            if let Err(err) = set_array(&mut self.lexer, &self.toml_state.kernel.args) {
                log::error!("error setting `kernel.args`: {err}");
                return Err(err.into());
            }
        } else {
            log::error!("`{}` is not a valid key for the kernel table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }

    /// Function to control parsing of the kernel table.
    fn parse_init_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        if key.eq("path") {
            log::trace!("setting `init.path`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.init.path) {
                log::error!("error setting `init.path`: {err}");
                return Err(err.into());
            }
        } else if key.eq("checksum") {
            log::trace!("setting `init.checksum`");

            if let Err(err) = set(&mut self.lexer, &self.toml_state.init.checksum) {
                log::error!("error setting `init.checksum`: {err}");
                return Err(err.into());
            }
        } else if key.eq("args") {
            log::trace!("setting `init.args`");

            if let Err(err) = set_array(&mut self.lexer, &self.toml_state.init.args) {
                log::error!("error setting `init.args`: {err}");
                return Err(err.into());
            }
        } else {
            log::error!("`{}` is not a valid key for the init table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }

    /// Function to control parsing of a module table.
    fn parse_module_key(&mut self, key: &dyn StringLike) -> Result<(), SetKeyError> {
        let module = self.toml_state.modules.as_slice_mut().last().unwrap();

        if key.eq("path") {
            log::trace!("setting `module.path`");

            if let Err(err) = set(&mut self.lexer, &module.path) {
                log::error!("error setting `module.path`: {err}");
                return Err(err.into());
            }
        } else if key.eq("checksum") {
            log::trace!("setting `module.checksum`");

            if let Err(err) = set(&mut self.lexer, &module.checksum) {
                log::error!("error setting `module.checksum`: {err}");
                return Err(err.into());
            }
        } else if key.eq("args") {
            log::trace!("setting `module.args`");

            if let Err(err) = set_array(&mut self.lexer, &module.args) {
                log::error!("error setting `module.args`: {err}");
                return Err(err.into());
            }
        } else {
            log::error!("`{}` is not a valid key for a module table", key);
            return Err(SetKeyError::InvalidKey);
        }

        Ok(())
    }
}

/// Various errors that can occur while parsing a config file.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseConfigError {
    /// An error in the lexer occurred.
    LexingError,
    /// A key that was not set has no reasonable default value,
    /// so it must be set.
    UnsetMustSet,
}

/// Various errors that can occur while setting a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetKeyError<'config> {
    /// The specified key is invalid.
    InvalidKey,
    /// The key-value assignment was in an invalid format.
    InvalidFormat(Token<'config>),
    /// The value provided was invalid for the given key.
    InvalidValue,
    /// The key has already been set.
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

/// Various errors that can occur while parsing a module header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseModuleHeaderError<'config> {
    /// The module has already been declared.
    AlreadyDeclared,
    /// The format of the header was in an invalid format.
    InvalidFormat(Token<'config>),
    /// The selected table was not a valid table.
    InvalidTable,
}

/// The four different types of tables.
///
/// 3 are unique, namely `Global`, `Logging`, and `Kernel`, and one is an array,
/// namely `Modules`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Table {
    /// The first, implicit table.
    Global,
    /// The table controlling logging.
    Logging,
    /// The table controlling the kernel.
    Kernel,
    /// The table controlling the initial program.
    Init,
    /// Tables controlling their respective module.
    Modules,
}

impl Display for Table {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Table::Global => f.write_str("global"),
            Table::Logging => f.write_str("logging"),
            Table::Kernel => f.write_str("kernel"),
            Table::Init => f.write_str("init"),
            Table::Modules => f.write_str("modules"),
        }
    }
}

/// Various errors that might occur when adjusting the current table we are working on.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SetTableError<'config> {
    /// An unexpected token was run into.
    InvalidFormat(Token<'config>),
    /// The requested table has already been declared.
    AlreadyDeclared(Table),
    /// The requested table is not valid.
    InvalidTable,
}

impl Display for SetTableError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SetTableError::InvalidFormat(token) => {
                write!(f, "invalid table format: got {token:?}")
            }
            SetTableError::AlreadyDeclared(table) => {
                write!(f, "{table} table has already been declared")
            }
            SetTableError::InvalidTable => {
                write!(f, "the requested table is invalid")
            }
        }
    }
}

impl Error for SetTableError<'_> {}

/// Parses a value, consuming the equals sign.
fn set<'config, T: ParseFromLexer<'config>>(
    lexer: &mut Lexer<'config>,
    opt: &OnceCell<T>,
) -> Result<(), SetValueError<'config, T::Error>> {
    lexer.skip_whitespace();

    if let Err(token) = lexer.consume(TokenKind::Equals) {
        return Err(SetValueError::InvalidFormat(token));
    }

    lexer.skip_whitespace();

    let value = T::parse(lexer).map_err(SetValueError::InvalidValue)?;

    if opt.set(value).is_err() {
        return Err(SetValueError::AlreadySet);
    }

    Ok(())
}

/// Parses an array of a single type of values, consuming the equals sign.
fn set_array<'config, T: ParseFromLexer<'config>>(
    lexer: &mut Lexer<'config>,
    opt: &OnceCell<Vec<T>>,
) -> Result<(), SetValueError<'config, T::Error>> {
    lexer.skip_whitespace();

    if let Err(token) = lexer.consume(TokenKind::Equals) {
        return Err(SetValueError::InvalidFormat(token));
    }

    lexer.skip_whitespace();

    if let Err(token) = lexer.consume(TokenKind::LeftSquareBracket) {
        return Err(SetValueError::InvalidFormat(token));
    }

    let mut vec = Vec::new();

    while lexer.peek().kind != TokenKind::RightSquareBracket {
        lexer.skip_noncontent();

        let value = T::parse(lexer).map_err(SetValueError::InvalidValue)?;

        if let Err(value) = vec.push_within_capacity(value) {
            let new_capacity = vec.capacity().saturating_mul(2);

            let new_capacity = if new_capacity != 0 { new_capacity } else { 8 };

            assert_ne!(new_capacity, vec.capacity());

            if vec.try_reserve(new_capacity).is_err() {
                assert!(vec.try_reserve(1).is_ok());
            }

            assert!(vec.push_within_capacity(value).is_ok());
        }

        lexer.skip_noncontent();

        if lexer.consume(TokenKind::Comma).is_err() {
            break;
        }

        lexer.skip_noncontent();
    }

    if let Err(token) = lexer.consume(TokenKind::RightSquareBracket) {
        return Err(SetValueError::InvalidFormat(token));
    }

    if opt.set(vec).is_err() {
        return Err(SetValueError::AlreadySet);
    }

    Ok(())
}

/// Various errors that might occur when setting a value.
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
                write!(f, "invalid key-value format: got {token:?}")
            }
            SetValueError::InvalidValue(err) => {
                write!(f, "invalid value for associated key: {err}")
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
