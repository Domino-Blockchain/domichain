use statrs::distribution::{Binomial, DiscreteCDF};
use binary_search::{binary_search, Direction};

fn sortition_binomial_cdf_walk(n: u64, p: f64, ratio: f64, money: u64) -> u64 {
    let binomial = Binomial::new(p, n).unwrap();

    let ((largest_low, _), (smallest_high, _)) =
        binary_search((0, ()), (money, ()), |j| {
            let boundary = binomial.cdf(j);
            if ratio > boundary { Direction::Low(()) } else { Direction::High(()) }
        });

    let boundary = binomial.cdf(largest_low);
    if ratio <= boundary {
        largest_low
    } else {
        smallest_high
    }
}