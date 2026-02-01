pub fn round_up_to_power_of_10(x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    10.0_f64.powf(x.log10().ceil())
}

pub fn round_up_to_power_of_10_times_block_size(x: f64, block_size: f64) -> f64 {
    if x <= block_size {
        return block_size;
    }
    let n = (x / block_size).log10().ceil();
    10.0_f64.powf(n) * block_size
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_round_up_to_power_of_10() {
        assert!(is_close(round_up_to_power_of_10(95.0), 100.0));
        assert!(is_close(round_up_to_power_of_10(100.0), 100.0));
        assert!(is_close(round_up_to_power_of_10(100.001), 1000.0));
        assert!(is_close(round_up_to_power_of_10(1.0), 1.0));
        assert!(is_close(round_up_to_power_of_10(0.5), 1.0));
        assert!(is_close(round_up_to_power_of_10(0.1), 0.1));
        assert!(is_close(round_up_to_power_of_10(0.09), 0.1));
        assert!(is_close(round_up_to_power_of_10(0.0), 1.0));
        assert!(is_close(round_up_to_power_of_10(-1.0), 1.0));
    }

    #[test]
    fn test_round_up_to_power_of_10_times_block_size() {
        let block_size = 1024.0;
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(0.5, block_size),
            1024.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(1.0, block_size),
            1024.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(1023.0, block_size),
            1024.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(1024.0, block_size),
            1024.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(1025.0, block_size),
            10240.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(10240.0, block_size),
            10240.0
        ));
        assert!(is_close(
            round_up_to_power_of_10_times_block_size(10241.0, block_size),
            102400.0
        ));
    }
}
