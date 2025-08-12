pub fn db_to_gain(db: f32) -> f32 {
    let factor = 0.05;
    10.0_f32.powf(db * factor)
}

pub fn gain_to_db(gain: f32) -> f32 {
    let factor = 0.05;
    (gain.log10() / factor).round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::compare;

    #[test]
    fn test_db_to_gain() {
        let db = 3.0;
        let act_gain = db_to_gain(db);
        let exp_gain = 1.4125376;
        assert!(compare::near_relative(act_gain, exp_gain, 0.001));

        let db = f32::NEG_INFINITY;
        let act_gain = db_to_gain(db);
        assert_eq!(act_gain, 0.0);
    }

    #[test]
    fn test_gain_to_db() {
        let act_db = gain_to_db(1.0);
        let exp_db = 0.0;
        assert!(compare::near_relative(act_db, exp_db, 0.001));

        // 0 -> -inf dB
        let act_db = gain_to_db(0.0);
        assert!(act_db.is_infinite());
        assert!(act_db.is_sign_negative());
    }
}
