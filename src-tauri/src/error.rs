use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("variável HOME ausente no ambiente")]
    MissingHome,
    #[error("falha de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("categoria desconhecida: {0}")]
    UnknownCategory(String),
    #[error("o item {0} não está no último scan; escaneie de novo antes de limpar")]
    UnknownItem(String),
    #[error("{0} está fora das pastas que o Sam pode limpar; nada foi apagado")]
    OutsideAllowedRoots(PathBuf),
    #[error("não foi possível mover para a Lixeira: {0}")]
    Trash(String),
    #[error("o comando {program} falhou: {message}")]
    Command { program: String, message: String },
    #[error("configuração inválida: {0}")]
    InvalidSettings(String),
    #[error("dados inválidos: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::MissingHome => "missingHome",
            Self::Io(_) => "io",
            Self::UnknownCategory(_) => "unknownCategory",
            Self::UnknownItem(_) => "unknownItem",
            Self::OutsideAllowedRoots(_) => "outsideAllowedRoots",
            Self::Trash(_) => "trash",
            Self::Command { .. } => "command",
            Self::InvalidSettings(_) => "invalidSettings",
            Self::Serialization(_) => "serialization",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut payload = serializer.serialize_struct("AppError", 2)?;
        payload.serialize_field("kind", self.kind())?;
        payload.serialize_field("message", &self.to_string())?;
        payload.end()
    }
}
