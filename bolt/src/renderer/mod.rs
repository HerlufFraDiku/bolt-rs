// GOALS:
// 1. Draw one quad

pub enum Shape2D {
    Quad { position: [f32; 3], color: [f32; 4] },
}

pub struct Renderer2D {}

impl Renderer2D {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&self, shape: Shape2D) {
        match shape {
            Shape2D::Quad { position, color } => {
                println!("draw quad position: {:?} color: {:?}", position, color);
            }
        }
    }
}
