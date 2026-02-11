use vulkanalia::bytecode::BytecodeError;

#[derive(Debug)]
pub enum RenderPipelineError {
    ByteCodeError(BytecodeError),
    RenderableNotAllocated,
    Unprepared,
}

impl From<BytecodeError> for RenderPipelineError {
    fn from(value: BytecodeError) -> Self {
        Self::ByteCodeError(value)
    }
}
