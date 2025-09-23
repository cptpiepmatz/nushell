use std::{
    borrow::Cow,
    fmt::Write,
    ops::Range,
    path::{Path, PathBuf},
};
use serde::{Serialize, Deserialize};

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseUri {
    raw_path: PathBuf,
    encoded_path: String,
}

impl DatabaseUri {
    pub fn new<S, P, Q, K, V>(schema: S, path: P, params: Q) -> Self
    where
        S: AsRef<str>,
        P: AsRef<Path>,
        Q: IntoIterator<IntoIter: ExactSizeIterator<Item = (K, V)>>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let path = path.as_ref();
        let raw_path = path.to_owned();

        let mut encoded_path = format!(
            "{schema}:{path}",
            schema = schema.as_ref(),
            path = utf8_percent_encode(&path.to_string_lossy(), NON_ALPHANUMERIC)
        );

        let mut first = true;
        let params = params.into_iter();
        if params.len() != 0 {
            for (k, v) in params {
                match first {
                    false => encoded_path.push('&'),
                    true => {
                        encoded_path.push('?');
                        first = false;
                    }
                }

                let k = utf8_percent_encode(k.as_ref(), NON_ALPHANUMERIC);
                let v = utf8_percent_encode(v.as_ref(), NON_ALPHANUMERIC);
                write!(encoded_path, "{k}={v}").expect("infallible on string");
            }
        }

        Self {raw_path, encoded_path}
    }

    pub fn uri(&self) -> &Path {
        Path::new(&self.encoded_path)
    }

    pub fn path(&self) -> &Path {
        &self.raw_path
    }
}
