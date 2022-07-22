use statrs::distribution::{Binomial, DiscreteCDF};

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