use crate::model::{CardCatalog, CardDef};
use std::{fs, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum CardCatalogLoadError {
    #[error("カード定義ファイルの読み込みに失敗しました: {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },

    #[error("カード定義YAMLのパースに失敗しました: {path}: {source}")]
    ParseYaml {
        path: String,
        source: serde_yaml::Error,
    },
}

pub fn load_card_catalog(path: impl AsRef<Path>) -> Result<CardCatalog, CardCatalogLoadError> {
    let path_ref = path.as_ref();
    let s = fs::read_to_string(path_ref).map_err(|source| CardCatalogLoadError::ReadFile {
        path: path_ref.display().to_string(),
        source,
    })?;

    let defs: Vec<CardDef> =
        serde_yaml::from_str(&s).map_err(|source| CardCatalogLoadError::ParseYaml {
            path: path_ref.display().to_string(),
            source,
        })?;

    Ok(CardCatalog::from_defs(defs))
}
