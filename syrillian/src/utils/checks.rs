use wgpu::naga::front::wgsl;
use wgpu::naga::front::wgsl::ParseError;
use wgpu::naga::valid::{Capabilities, ModuleInfo, ValidationError, ValidationFlags, Validator};
use wgpu::naga::WithSpan;

#[derive(Debug)]
pub enum ShaderValidError {
    Parse(ParseError),
    ValidationError(WithSpan<ValidationError>),
}

impl ShaderValidError {
    pub fn emit_to_stderr(&self, source: &str) {
        match self {
            ShaderValidError::Parse(e) => e.emit_to_stderr(source),
            ShaderValidError::ValidationError(e) => e.emit_to_stderr(source),
        }
    }

    pub fn emit_to_stderr_with_path(&self, source: &str, path: &str) {
        match self {
            ShaderValidError::Parse(e) => e.emit_to_stderr_with_path(source, path),
            ShaderValidError::ValidationError(e) => e.emit_to_stderr_with_path(source, path),
        }
    }
}

pub fn validate_wgsl_source(shader: &str) -> Result<ModuleInfo, ShaderValidError> {
    let module = wgsl::parse_str(&shader).map_err(ShaderValidError::Parse)?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator
        .validate(&module)
        .map_err(ShaderValidError::ValidationError)
}
