use ecs::Component;

#[derive(Debug, Clone, PartialEq)]
pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RGBA {
    pub fn new(rgba: [f32; 4]) -> Self {
        Self {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: RGBA,
}

impl Material {
    pub fn new(color: RGBA) -> Self {
        Self { color }
    }
}

impl Component for Material {}
