use num_bigint::{BigInt, BigUint, Sign};
use num_rational::BigRational;
use num_traits::cast::ToPrimitive;
use statrs::distribution::{Binomial, DiscreteCDF};
use domichain_program::hash::Hash;

pub fn select(money: u64, total_money: u64, expected_size: f64, vrf_output: Hash) -> u64 {
    let binomial_p = expected_size / total_money as f64;

    let t = BigInt::from_bytes_be(Sign::Plus, vrf_output.as_ref());
    let h = BigRational::from_integer(t);

    let max_float = BigUint::from(2u8).pow(256) - 1u8;
    let cratio = (h / BigInt::from(max_float)).to_f64().unwrap();

    sortition_binomial_cdf_walk(money, binomial_p, cratio, money)
}

fn sortition_binomial_cdf_walk(n: u64, p: f64, ratio: f64, money: u64) -> u64 {
    let binomial = Binomial::new(p, n).unwrap();
    for j in 0..money {
        let boundary = binomial.cdf(j);
        if ratio <= boundary {
            return j;
        }
    }
    return money;
}

#[test]
fn test_select() {
    use rand::Fill;
    let mut rng = rand::thread_rng();

    let mut hitcount = 0;
    let n = 1000;
    let expected_size = 20.0;
    let my_money = 100;
    let total_money = 200;

    for _ in 0..n {
        let mut vrf_output = [0u8; 32];
        vrf_output.try_fill(&mut rng).unwrap();
        let vrf_output = Hash::new(&vrf_output);

        let selected = select(my_money, total_money, expected_size, vrf_output);
        hitcount += selected
    }
    let expected = (n as f64 * expected_size / 2.0) as u64;
    let d = expected.abs_diff(hitcount);
    let maxd = expected / 50;
    assert!(d <= maxd, "wanted {expected} selections but got {hitcount}, d={d}, maxd={maxd}");
}