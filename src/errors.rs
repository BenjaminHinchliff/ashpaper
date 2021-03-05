use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cranelift_module error: {0}")]
    CraneliftModuleError(#[from] cranelift_module::ModuleError),
}

pub type Result<T> = ::std::result::Result<T, Error>;
