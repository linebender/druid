use crate::environment::Value;
use std::convert::{TryFrom, TryInto};

#[derive(Debug)]
pub struct Animation {
    duration: f64,
    reverse: bool,
    loop_: bool,
    components: Vec<Component>,
}

#[derive(Debug)]
struct Component {
    ident: &'static str,
    curve: AnimationCurve,
    start: Value,
    end: Value,
    current: Value,
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Animation {
    pub fn with_duration(secs: f64) -> Self {
        Animation {
            duration: secs,
            reverse: false,
            loop_: false,
            components: Vec::new(),
        }
    }

    pub fn adding_component<T: Into<Value>>(
        mut self,
        ident: &'static str,
        curve: AnimationCurve,
        start: T,
        end: T,
    ) -> Self {
        let start = start.into();

        let component = Component {
            ident,
            curve,
            start: start.clone(),
            end: end.into(),
            current: start,
        };
        self.components.push(component);
        self
    }

    pub fn reversing(mut self, reversing: bool) -> Self {
        self.reverse = reversing;
        self
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.loop_ = looping;
        self
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn current_value<V: TryFrom<Value, Error = String>>(&self, key: &str) -> V {
        self.components
            .iter()
            .find(|c| c.ident == key)
            .map(|c| c.current)
            .ok_or_else(|| format!("no component with identifier '{}'", key))
            .and_then(Value::try_into)
            .unwrap()
    }

    pub fn rerun_after_completion(&mut self) -> bool {
        if self.reverse {
            self.components.iter_mut().for_each(|c| {
                let tmp = c.start;
                c.start = c.end;
                c.end = tmp;
            });
            // if we're looping, keep reverse true
            self.reverse = self.loop_;
            true
        } else {
            self.loop_
        }
    }

    pub(crate) fn update_components(&mut self, t: f64) {
        for c in self.components.iter_mut() {
            c.current = c.start.interpolate(c.end, t)
        }
    }
}

trait Interpolate: Copy {
    /// Calculate the interpolate between `self` and `other` at timestep `t`.
    fn interpolate(self, other: Self, t: f64) -> Self;
}

impl Interpolate for f64 {
    fn interpolate(self, other: Self, t: f64) -> Self {
        assert!(t >= 0. && t <= 1.0, "{}", t);
        self + (other - self) * t
    }
}

impl Interpolate for u32 {
    fn interpolate(self, other: Self, t: f64) -> Self {
        //gross: we're using u32 for colors but i need floats for the math (which I stole from rik)
        let (r1, g1, b1, a1) = rgba_u32_to_floats(self);
        let (r2, g2, b2, a2) = rgba_u32_to_floats(other);
        let ot = 1.0 - t;

        let rgba = (
            r1 * ot + r2 * t,
            g1 * ot + g2 * t,
            b1 * ot + b2 * t,
            a1 * ot + a2 * t,
        );

        rgba_float_to_u32(rgba)
    }
}

impl Interpolate for Value {
    fn interpolate(self, other: Self, t: f64) -> Self {
        use Value::*;
        match (self, other) {
            (Point(one), Point(_two)) => Point(one),
            (Size(one), Size(_two)) => Size(one),
            (Rect(one), Rect(_two)) => Rect(one),
            (Color(one), Color(two)) => Color(one.interpolate(two, t)),
            (Float(one), Float(two)) => Float(one.interpolate(two, t)),
            (String(one), String(_two)) => String(one),
            _ => panic!("attempt to interpolate unlike values"),
        }
    }
}

fn rgba_float_to_u32(color: (f64, f64, f64, f64)) -> u32 {
    let mut out = (255. * color.3) as u32;
    out = out | ((255. * color.0) as u32) << 24;
    out = out | ((255. * color.1) as u32) << 16;
    out = out | ((255. * color.2) as u32) << 8;
    out
}

fn rgba_u32_to_floats(rgba: u32) -> (f64, f64, f64, f64) {
    (
        ((rgba & (0xFF << 24)) >> 24) as f64 / 255.,
        ((rgba & (0xFF << 16)) >> 16) as f64 / 255.,
        ((rgba & (0xFF << 8)) >> 8) as f64 / 255.,
        (rgba & 0xFF) as f64 / 255.,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_conversions() {
        assert_eq!(rgba_float_to_u32((1.0, 1.0, 0.0, 0.0)), 0xFF_FF_00_00);
        assert_eq!(rgba_float_to_u32((0.0, 0.0, 1.0, 1.0)), 0x00_00FF_FF);

        assert_eq!(rgba_u32_to_floats(0xFF_FF_00_00), (1.0, 1.0, 0.0, 0.0));
        assert_eq!(rgba_u32_to_floats(0x00_00FF_FF), (0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn color() {
        assert_eq!(10_u32.interpolate(20, 0.0), 10);
        assert_eq!(10_u32.interpolate(20, 1.0), 20);
        assert_eq!(0xFF_00_00_00.interpolate(0x00_FF_00_00, 1.0), 0x00_FF_00_00);
    }
}
