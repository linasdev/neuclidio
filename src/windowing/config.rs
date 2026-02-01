pub struct WindowConfig {
    pub bit_depth: u8,
    pub position_x: i16,
    pub position_y: i16,
    pub width: u16,
    pub height: u16,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            bit_depth: 24,
            position_x: 0,
            position_y: 0,
            width: 640,
            height: 480,
        }
    }
}
