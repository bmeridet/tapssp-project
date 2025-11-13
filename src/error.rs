
#[derive(Debug)]
pub enum LoxError {
    CompileError(String),
    RuntimeError(String),
}