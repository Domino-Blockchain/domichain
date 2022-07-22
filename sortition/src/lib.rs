use primitive_types::U256;
use domichain_program::hash::Hash;

pub fn select(
    money: u64,
    total_money: u64,
    expected_size: f64,
    vrf_output: Hash,
) -> u64 {
    let binomial_n = money;
    let binomial_p = expected_size / total_money as f64;
    let cratio = get_cratio(vrf_output);

    sortition_binomial_cdf_walk(binomial_n, binomial_p, cratio, money)
}

fn get_cratio(vrf_output: Hash) -> f64 {
    let t = U256::from_little_endian(vrf_output.as_ref()).to_f64_lossy();
    let max_float = U256::MAX.to_f64_lossy();
    t / max_float
}

#[cfg(feature = "inverse_cdf")]
mod inverse_cdf {
    use probability::distribution::{Binomial, Inverse};

    pub fn sortition_binomial_cdf_walk(n: u64, p: f64, ratio: f64, money: u64) -> u64 {
        let boundary_money = Binomial::new(n as usize, p).inverse(ratio) as u64;
        boundary_money.min(money)
    }
}

// Different implementations
cfg_if::cfg_if! {
    if #[cfg(feature = "inverse_cdf")] {
        use inverse_cdf::sortition_binomial_cdf_walk;
    } else if #[cfg(feature = "algorand_cdf_walk")] {
        mod algorand_cdf_walk;
        use algorand_cdf_walk::sortition_binomial_cdf_walk;
    } else if #[cfg(feature = "cdf_binary_search")] {
        mod cdf_binary_search;
        use cdf_binary_search::sortition_binomial_cdf_walk;
    } else {
        compile_error!("One of the feature should be enabled");
        fn sortition_binomial_cdf_walk(n: u64, p: f64, ratio: f64, money: u64) -> u64 {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{Fill, prelude::StdRng, SeedableRng};
    use super::*;

    #[test]
    fn test_random_select() {
        let mut rng = StdRng::from_seed(*b"12345678901234567890123456789012");

        let mut ok = 0;
        let mut err = 0;
        let mut avg = 0;

        for _ in 0..1000 {
            let mut hitcount = 0;
            let n = 1000;
            let expected_size = 20.0;
            let my_money = 100;
            let total_money = 200;

            for _ in 0..n {
                let mut vrf_output = [0u8; 32];
                vrf_output.try_fill(&mut rng).unwrap();
                let vrf_output = Hash::new_from_array(vrf_output);

                let selected = select(
                    my_money,
                    total_money,
                    expected_size,
                    vrf_output,
                );

                hitcount += selected
            }
            let expected = (n as f64 * expected_size / 2.0) as u64;
            let d = expected.abs_diff(hitcount);
            // within 2% good enough
            let maxd = expected / 50; // or by 20 for 5%?
            avg += d;
            if d > maxd {
                err += 1;
            } else {
                ok += 1;
            }
        }
        println!("ok: {ok}, err: {err}, avg: {avg}", avg=avg / 1000);
    }

    #[test]
    fn test_sampled_select() {
        let sample: [([u8; 32], f64); 1000] = include!("../samples/sample.txt");

        let mut hitcount = 0;
        let n = 1000;
        let expected_size = 20.0;
        let my_money = 100;
        let total_money = 200;

        for i in 0..n {
            let (arr, _exp_cratio) = sample[i];
            let vrf_output = Hash::new_from_array(arr);

            let selected = select(
                my_money,
                total_money,
                expected_size,
                vrf_output,
            );

            hitcount += selected
        }
        let expected = (n as f64 * expected_size / 2.0) as u64;
        let d = expected.abs_diff(hitcount);
        // within 2% good enough
        let maxd = expected / 50; // or by 20 for 5%?
        if d > maxd {
            panic!("wanted {expected} selections but got {hitcount}, d={d}, maxd={maxd}");
        }
    }

    #[test]
    fn test_random_cratio() {
        let mut rng = StdRng::from_seed(*b"12345678901234567890123456789012");
        let n = 1_000_000;

        let mut avg_cratio = 0.0;
        let mut min_cratio = f64::MAX;
        let mut max_cratio = f64::MIN;

        for _ in 0..n {
            let mut vrf_output = [0u8; 32];
            vrf_output.try_fill(&mut rng).unwrap();
            let vrf_output = Hash::new_from_array(vrf_output);
            let cratio = get_cratio(vrf_output);

            avg_cratio += cratio;

            min_cratio = min_cratio.min(cratio);
            max_cratio = max_cratio.max(cratio);
        }

        avg_cratio = avg_cratio / n as f64;
        assert!((avg_cratio - 0.5).abs() < 0.01, "avg_cratio={avg_cratio} should be close to 0.5");

        assert!((min_cratio - 0.0).abs() < 0.01, "min_cratio={min_cratio} should be close to 0");
        assert!((max_cratio - 1.0).abs() < 0.01, "max_cratio={max_cratio} should be close to 1");
    }
}