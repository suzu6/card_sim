use crate::model::{Deck, DeckDef};
use std::{fs, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum DeckLoadError {
    #[error("デッキ定義ファイルの読み込みに失敗しました: {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },

    #[error("デッキ定義YAMLのパースに失敗しました: {path}: {source}")]
    ParseYaml {
        path: String,
        source: serde_yaml::Error,
    },
}

pub fn load_deck(path: impl AsRef<Path>) -> Result<Deck, DeckLoadError> {
    let path_ref = path.as_ref();
    let s = fs::read_to_string(path_ref).map_err(|source| DeckLoadError::ReadFile {
        path: path_ref.display().to_string(),
        source,
    })?;

    let def: DeckDef = serde_yaml::from_str(&s).map_err(|source| DeckLoadError::ParseYaml {
        path: path_ref.display().to_string(),
        source,
    })?;

    Ok(Deck::from_def(def))
}
