use strum::EnumCount;

#[derive(Copy, Clone, Debug, EnumCount)]
pub enum Channel {
    Left,
    Right,
    Center,
    LeftSurround,
    RightSurround,
}

impl Channel {
    pub fn weight(&self) -> f64 {
        match self {
            &Channel::Left | &Channel::Right | &Channel::Center => 1.0,
            &Channel::LeftSurround | &Channel::RightSurround => 1.41,
        }
    }
}
