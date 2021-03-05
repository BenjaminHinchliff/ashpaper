#[cfg(feature = "jit")]
pub mod jit {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum JitError {
        #[error("cranelift_module error: {0}")]
        CraneliftModuleError(#[from] cranelift_module::ModuleError),
    }

    pub type JitResult<T> = ::std::result::Result<T, JitError>;
}
