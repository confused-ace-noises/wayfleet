#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    EaseInOut,
    EaseOutBack,
}

impl Easing {
    pub fn func(&self) -> fn(f64) -> f64 {
        match self {
            Easing::Linear => Self::linear,
            Easing::EaseInOut => Self::ease_in_out,
            Easing::EaseOutBack => Self::ease_out_back,
        }
    }

    fn linear(x: f64) -> f64 {
        x
    }

    fn ease_in_out(x: f64) -> f64 {
        if x < 0.5 {
            4. * x * x * x
        } else {
            1. - (-2. * x + 2.).powi(3) / 2.
        }
    }

    fn ease_out_back(x: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.;

        1. + c3 * (x - 1.).powi(3) + c1 * (x - 1.).powi(2)
    }
}

