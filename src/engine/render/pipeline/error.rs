use vulkanalia::bytecode::BytecodeError;

#[derive(Debug)]
pub enum NeuclidioRenderPipelineError {
    ByteCodeError(BytecodeError),
}

impl From<BytecodeError> for NeuclidioRenderPipelineError {
    fn from(value: BytecodeError) -> Self {
        Self::ByteCodeError(value)
    }
}
