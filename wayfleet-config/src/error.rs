use derive_more::{Display, Error, From};
use miette::{Diagnostic, NamedSource, SourceSpan};

#[derive(Debug, Display, Error, From, Diagnostic)]
pub enum ConfigError {
    #[display("failed to read config file")]
    #[diagnostic(code(config::io))]
    Io(std::io::Error),

    #[display("failed to parse config: {message}")]
    #[diagnostic(code(config::parse))]
    #[from(ignore)]
    Parse {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("{message}")]
        span: SourceSpan,
    },

    #[display("failed to validate config")]
    #[diagnostic(transparent)]
    Validation(ValidationError),
}

impl ConfigError {
    pub(crate) fn from_serde(error: toml::de::Error, src: NamedSource<String>) -> Self {
        let message = error.message();
        let span = error.span().unwrap_or(0..0);

        ConfigError::Parse {
            message: message.to_string(),
            src,
            span: span.into(),
        }
    }
}

#[derive(Debug, Error, Display, Diagnostic)]
pub enum ValidationError {
    #[display("grid.size and grid.cells cannot both be \"auto\"")]
    #[diagnostic(
        code(config::validation::map::ambiguos_auto::all),
        help("set both values of grid.size or grid.cells to an explicit value")
    )]
    MapValidationAll {
        #[source_code]
        src: NamedSource<String>,

        #[label("this is auto")]
        loc1: SourceSpan,

        #[label("this is auto")]
        loc2: SourceSpan
    },

    #[display("grid.size.height and grid.cells.rows cannot both be \"auto\"")]
    #[diagnostic(
        code(config::validation::map::ambiguos_auto::vert),
        help("set grid.size.rows or grid.cells.rows to an explicit value")
    )]
    MapValidationRows {
        #[source_code]
        src: NamedSource<String>,

        #[label("this is auto")]
        loc1: SourceSpan,

        #[label("this is auto")]
        loc2: SourceSpan
    },

    #[display("grid.size.width and grid.cells.columns cannot both be \"auto\"")]
    #[diagnostic(
        code(config::validation::map::ambiguos_auto::horiz),
        help("set both values of grid.size or grid.cells to an explicit value")
    )]
    MapValidationColumns {
        #[source_code]
        src: NamedSource<String>,

        #[label("this is auto")]
        loc1: SourceSpan,

        #[label("this is auto")]
        loc2: SourceSpan
    },

    #[display("grid.size or grid.cells cannot be \"auto\" if the other field isn't fully specified")]
    #[diagnostic(
        code(config::validation::map::ambiguos_auto::misc),
        help("set all values of one field to allow \"auto\" on the other")
    )]
    MapValidationGeneric {
        #[source_code]
        src: NamedSource<String>,

        #[label("this is auto")]
        loc1: SourceSpan,

        #[label("this is auto")]
        loc2: SourceSpan
    }
}