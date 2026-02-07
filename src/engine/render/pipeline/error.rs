use vulkanalia::bytecode::BytecodeError;

#[derive(Debug)]
pub enum RenderPipelineError {
    ByteCodeError(BytecodeError),
    Unprepared,
}

impl From<BytecodeError> for RenderPipelineError {
    fn from(value: BytecodeError) -> Self {
        Self::ByteCodeError(value)
    }
}
