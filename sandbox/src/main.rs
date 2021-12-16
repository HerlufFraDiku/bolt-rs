use bolt::{Renderer2D, Shape2D};

fn main() {
    let renderer = Renderer2D::new();

    let quad = Shape2D::Quad {
        position: [0.0, 0.0, 0.0],
        color: [0.1, 0.2, 0.3, 1.0],
    };
    renderer.draw(quad);
}
