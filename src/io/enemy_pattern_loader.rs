use crate::model::{EnemyPattern, EnemyPatternDef};
use std::{fs, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum EnemyPatternLoadError {
    #[error("敵行動定義ファイルの読み込みに失敗しました: {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },

    #[error("敵行動定義YAMLのパースに失敗しました: {path}: {source}")]
    ParseYaml {
        path: String,
        source: serde_yaml::Error,
    },
}

pub fn load_enemy_pattern(path: impl AsRef<Path>) -> Result<EnemyPattern, EnemyPatternLoadError> {
    let path_ref = path.as_ref();
    let s = fs::read_to_string(path_ref).map_err(|source| EnemyPatternLoadError::ReadFile {
        path: path_ref.display().to_string(),
        source,
    })?;

    let def: EnemyPatternDef =
        serde_yaml::from_str(&s).map_err(|source| EnemyPatternLoadError::ParseYaml {
            path: path_ref.display().to_string(),
            source,
        })?;

    Ok(EnemyPattern::from_def(def))
}
